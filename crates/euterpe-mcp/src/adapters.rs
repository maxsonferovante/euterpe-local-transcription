use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{Local, NaiveDate};
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::{
    domain::{Operation, OperationKind, OperationStatus, Transcript, TranscriptSegment},
    ports::{
        InputFileAccess, MediaMetadata, MediaProcessor, OperationRepository, ProgressReporter,
        TranscriptRepository,
    },
};

pub mod mcp;

pub struct SqliteOperationRepository {
    connection: Mutex<Connection>,
    data_dir: PathBuf,
}

impl SqliteOperationRepository {
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(data_dir.join("operations"))?;
        let connection = Connection::open(data_dir.join("operations.sqlite3"))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS operations (
               id TEXT PRIMARY KEY, kind TEXT NOT NULL, input_path TEXT NOT NULL,
               output_path TEXT NOT NULL, parent_id TEXT, status TEXT NOT NULL,
               progress INTEGER NOT NULL DEFAULT 0, error TEXT, created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             UPDATE operations SET status='failed', error='Server restarted while operation was running',
               updated_at=datetime('now') WHERE status IN ('queued','processing');",
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
            data_dir,
        })
    }

    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| anyhow!("database lock is unavailable"))
    }
}

impl OperationRepository for SqliteOperationRepository {
    fn create(
        &self,
        kind: OperationKind,
        input_path: &Path,
        parent_id: Option<&str>,
    ) -> Result<Operation> {
        let id = Uuid::new_v4().to_string();
        let ext = match kind {
            OperationKind::Conversion => "mp3",
            OperationKind::Transcription => "json",
        };
        let output_path = self.data_dir.join("operations").join(format!("{id}.{ext}"));
        let now = Local::now().to_rfc3339();
        self.connection()?.execute(
            "INSERT INTO operations(id,kind,input_path,output_path,parent_id,status,progress,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,'queued',0,?6,?6)",
            params![id, kind.as_str(), input_path.to_string_lossy(), output_path.to_string_lossy(), parent_id, now],
        )?;
        self.find(&id)?
            .ok_or_else(|| anyhow!("newly created operation was not found"))
    }

    fn find(&self, id: &str) -> Result<Option<Operation>> {
        self.connection()?
            .query_row(
                "SELECT id,kind,input_path,output_path,parent_id,status,progress,error,created_at,updated_at FROM operations WHERE id=?1",
                [id],
                row_to_operation,
            )
            .optional()
            .map_err(Into::into)
    }

    fn list_conversions_on(&self, date: NaiveDate) -> Result<Vec<Operation>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id,kind,input_path,output_path,parent_id,status,progress,error,created_at,updated_at FROM operations WHERE kind='conversion' AND substr(created_at,1,10)=?1 ORDER BY created_at",
        )?;
        let rows = statement.query_map([date.format("%Y-%m-%d").to_string()], row_to_operation)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn update(
        &self,
        id: &str,
        status: OperationStatus,
        progress: u8,
        error: Option<&str>,
    ) -> Result<()> {
        self.connection()?.execute(
            "UPDATE operations SET status=?2,progress=?3,error=?4,updated_at=?5 WHERE id=?1",
            params![
                id,
                status.as_str(),
                progress,
                error,
                Local::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }
}

pub struct OperationProgressReporter {
    repository: Arc<dyn OperationRepository>,
    operation_id: String,
}

impl OperationProgressReporter {
    pub fn new(repository: Arc<dyn OperationRepository>, operation_id: String) -> Self {
        Self {
            repository,
            operation_id,
        }
    }
}

impl ProgressReporter for OperationProgressReporter {
    fn report_progress(&self, progress: u8) {
        let scaled_progress = 5 + (u16::from(progress.min(100)) * 90 / 100) as u8;
        if let Err(error) = self.repository.update(
            &self.operation_id,
            OperationStatus::Processing,
            scaled_progress,
            None,
        ) {
            tracing::warn!(operation_id = %self.operation_id, %error, "failed to persist transcription progress");
        }
    }
}

fn row_to_operation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Operation> {
    let kind: String = row.get(1)?;
    let status: String = row.get(5)?;
    let kind = match kind.as_str() {
        "conversion" => OperationKind::Conversion,
        _ => OperationKind::Transcription,
    };
    let status = match status.as_str() {
        "processing" => OperationStatus::Processing,
        "completed" => OperationStatus::Completed,
        "failed" => OperationStatus::Failed,
        _ => OperationStatus::Queued,
    };
    Ok(Operation {
        id: row.get(0)?,
        kind,
        input_path: row.get(2)?,
        output_path: row.get(3)?,
        parent_id: row.get(4)?,
        status,
        progress: row.get(6)?,
        error: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub struct LocalFileAccess {
    allowed_roots: Vec<PathBuf>,
    data_dir: PathBuf,
}

impl LocalFileAccess {
    pub fn new(roots: Vec<PathBuf>, data_dir: PathBuf) -> Result<Self> {
        let roots = roots
            .iter()
            .map(fs::canonicalize)
            .collect::<std::io::Result<Vec<_>>>()
            .context("one of the allowed roots does not exist")?;
        Ok(Self {
            allowed_roots: roots,
            data_dir,
        })
    }
}

impl InputFileAccess for LocalFileAccess {
    fn resolve_allowed_file(&self, path: &str) -> Result<PathBuf> {
        let canonical = fs::canonicalize(path).with_context(|| format!("could not open {path}"))?;
        if !canonical.is_file() {
            bail!("path must point to a regular file");
        }
        let data_root = fs::canonicalize(&self.data_dir)?;
        if !self
            .allowed_roots
            .iter()
            .any(|root| canonical.starts_with(root))
            && !canonical.starts_with(data_root)
        {
            bail!("path is outside the allowed roots; configure MCP_AUDIO_ALLOWED_ROOTS");
        }
        Ok(canonical)
    }
}

fn inspect_media(ffprobe: &str, path: &Path) -> Result<MediaMetadata> {
    let result = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=format_name",
            "-show_entries",
            "stream=codec_type,codec_name",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .with_context(|| {
            format!(
                "failed to start ffprobe ({ffprobe}); install FFmpeg/ffprobe or set FFPROBE_BIN"
            )
        })?;
    if !result.status.success() {
        bail!(
            "ffprobe could not read media metadata for {}: {}",
            path.display(),
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    let value: serde_json::Value =
        serde_json::from_slice(&result.stdout).context("ffprobe returned invalid JSON metadata")?;
    let container = value
        .get("format")
        .and_then(|v| v.get("format_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_owned();
    let streams = value
        .get("streams")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let audio: Vec<_> = streams
        .iter()
        .filter(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("audio"))
        .collect();
    let audio_codec = audio
        .first()
        .and_then(|s| s.get("codec_name"))
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    Ok(MediaMetadata {
        container,
        audio_codec,
        audio_streams: audio.len(),
    })
}

pub struct LocalMediaProcessor {
    ffmpeg: String,
    ffprobe: String,
    model_path: PathBuf,
    inference: Arc<Mutex<()>>,
}

impl LocalMediaProcessor {
    pub fn new(ffmpeg: String, model_path: PathBuf) -> Self {
        Self {
            ffmpeg,
            ffprobe: std::env::var("FFPROBE_BIN").unwrap_or_else(|_| "ffprobe".into()),
            model_path,
            inference: Arc::new(Mutex::new(())),
        }
    }
}

impl MediaProcessor for LocalMediaProcessor {
    fn inspect_media(&self, input: &Path) -> Result<MediaMetadata> {
        inspect_media(&self.ffprobe, input)
    }

    fn convert_video_to_mp3(&self, input: &Path, output: &Path) -> Result<()> {
        let result = Command::new(&self.ffmpeg)
            .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
            .arg(input)
            .args(["-vn", "-codec:a", "libmp3lame", "-q:a", "2"])
            .arg(output)
            .output()
            .context("failed to start FFmpeg")?;
        if !result.status.success() {
            bail!(
                "FFmpeg exited with {}: {}",
                result.status,
                String::from_utf8_lossy(&result.stderr).trim()
            );
        }
        Ok(())
    }

    fn transcribe_portuguese(
        &self,
        input: &Path,
        operation_id: &str,
        progress: Arc<dyn ProgressReporter>,
    ) -> Result<Transcript> {
        transcribe_sync(self, input, operation_id, progress)
    }
}

fn transcribe_sync(
    processor: &LocalMediaProcessor,
    input: &Path,
    operation_id: &str,
    progress: Arc<dyn ProgressReporter>,
) -> Result<Transcript> {
    let _inference = processor
        .inference
        .lock()
        .map_err(|_| anyhow!("inference lock is unavailable"))?;
    if !processor.model_path.is_file() {
        bail!(
            "local model not found at {}; configure MCP_AUDIO_MODEL",
            processor.model_path.display()
        );
    }
    let output = Command::new(&processor.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args([
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-ar",
            "16000",
            "-ac",
            "1",
            "-",
        ])
        .output()
        .context("failed to decode audio with FFmpeg")?;
    if !output.status.success() {
        bail!(
            "FFmpeg could not decode the audio: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let audio = output
        .stdout
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect::<Vec<_>>();
    if audio.is_empty() {
        bail!("file contains no decodable audio");
    }
    let model = processor
        .model_path
        .to_str()
        .ok_or_else(|| anyhow!("model path is not valid UTF-8"))?;
    let context = WhisperContext::new_with_params(model, WhisperContextParameters::default())
        .context("failed to load the whisper.cpp model")?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("pt"));
    params.set_translate(false);
    params.set_n_threads(
        std::thread::available_parallelism()
            .map(|n| n.get().min(8) as i32)
            .unwrap_or(2),
    );
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);
    let progress_callback: Box<dyn FnMut(i32)> = Box::new(move |percent| {
        progress.report_progress(percent.clamp(0, 100) as u8);
    });
    params.set_progress_callback_safe::<_, Box<dyn FnMut(i32)>>(Some(progress_callback));
    let mut whisper_state = context
        .create_state()
        .context("failed to create the model state")?;
    whisper_state
        .full(params, &audio)
        .context("whisper.cpp inference failed")?;
    let mut segments = Vec::new();
    let mut text = String::new();
    for segment in whisper_state.as_iter() {
        let segment_text = segment.to_str()?.to_owned();
        text.push_str(&segment_text);
        segments.push(TranscriptSegment {
            index: segments.len(),
            start_ms: segment.start_timestamp() * 10,
            end_ms: segment.end_timestamp() * 10,
            text: segment_text,
        });
    }
    Ok(Transcript {
        operation_id: operation_id.to_owned(),
        language: "pt-BR".into(),
        text: text.trim().to_owned(),
        segments,
    })
}

pub struct JsonTranscriptStore;

#[derive(serde::Serialize, serde::Deserialize)]
struct TranscriptRecord {
    operation_id: String,
    language: String,
    text: String,
    segments: Vec<SegmentRecord>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SegmentRecord {
    index: usize,
    start_ms: i64,
    end_ms: i64,
    text: String,
}

impl From<&Transcript> for TranscriptRecord {
    fn from(transcript: &Transcript) -> Self {
        Self {
            operation_id: transcript.operation_id.clone(),
            language: transcript.language.clone(),
            text: transcript.text.clone(),
            segments: transcript
                .segments
                .iter()
                .map(|segment| SegmentRecord {
                    index: segment.index,
                    start_ms: segment.start_ms,
                    end_ms: segment.end_ms,
                    text: segment.text.clone(),
                })
                .collect(),
        }
    }
}

impl From<TranscriptRecord> for Transcript {
    fn from(record: TranscriptRecord) -> Self {
        Self {
            operation_id: record.operation_id,
            language: record.language,
            text: record.text,
            segments: record
                .segments
                .into_iter()
                .map(|segment| TranscriptSegment {
                    index: segment.index,
                    start_ms: segment.start_ms,
                    end_ms: segment.end_ms,
                    text: segment.text,
                })
                .collect(),
        }
    }
}

impl TranscriptRepository for JsonTranscriptStore {
    fn save(&self, path: &Path, transcript: &Transcript) -> Result<()> {
        let temp_path = PathBuf::from(format!("{}.tmp", path.display()));
        fs::write(
            &temp_path,
            serde_json::to_vec_pretty(&TranscriptRecord::from(transcript))?,
        )?;
        fs::rename(temp_path, path)?;
        Ok(())
    }

    fn load(&self, path: &Path) -> Result<Transcript> {
        serde_json::from_slice::<TranscriptRecord>(&fs::read(path)?)
            .map(Transcript::from)
            .context("invalid transcript JSON")
    }
}
