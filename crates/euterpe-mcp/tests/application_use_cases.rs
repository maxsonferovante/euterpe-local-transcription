use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{Result, bail};
use chrono::NaiveDate;
use euterpe::{
    adapters::OperationProgressReporter,
    application,
    domain::{
        Operation, OperationKind, OperationStatus, Transcript, TranscriptPage, TranscriptSegment,
    },
    ports::{
        InputFileAccess, MediaMetadata, MediaProcessor, OperationRepository, ProgressReporter,
        TranscriptRepository,
    },
};

#[derive(Default)]
struct InMemoryOperations {
    operations: Mutex<Vec<Operation>>,
    updates: Mutex<Vec<(String, OperationStatus, u8)>>,
}

impl OperationRepository for InMemoryOperations {
    fn create(
        &self,
        kind: OperationKind,
        input_path: &Path,
        parent_id: Option<&str>,
    ) -> Result<Operation> {
        let mut operations = self.operations.lock().expect("operations mutex");
        let id = format!("operation-{}", operations.len() + 1);
        let operation = Operation {
            id,
            kind,
            input_path: input_path.to_string_lossy().into_owned(),
            output_path: format!("/runtime/{}.out", operations.len() + 1),
            parent_id: parent_id.map(str::to_owned),
            status: OperationStatus::Queued,
            progress: 0,
            error: None,
            created_at: "2026-11-09T10:00:00-03:00".into(),
            updated_at: "2026-11-09T10:00:00-03:00".into(),
        };
        operations.push(operation.clone());
        Ok(operation)
    }

    fn find(&self, id: &str) -> Result<Option<Operation>> {
        Ok(self
            .operations
            .lock()
            .expect("operations mutex")
            .iter()
            .find(|operation| operation.id == id)
            .cloned())
    }

    fn list_conversions_on(&self, date: NaiveDate) -> Result<Vec<Operation>> {
        let date = date.format("%Y-%m-%d").to_string();
        Ok(self
            .operations
            .lock()
            .expect("operations mutex")
            .iter()
            .filter(|operation| {
                operation.kind == OperationKind::Conversion
                    && operation.created_at.starts_with(&date)
            })
            .cloned()
            .collect())
    }

    fn update(
        &self,
        id: &str,
        status: OperationStatus,
        progress: u8,
        error: Option<&str>,
    ) -> Result<()> {
        let mut operations = self.operations.lock().expect("operations mutex");
        let Some(operation) = operations.iter_mut().find(|operation| operation.id == id) else {
            bail!("operation not found")
        };
        operation.status = status;
        operation.progress = progress;
        operation.error = error.map(str::to_owned);
        self.updates.lock().expect("operation updates mutex").push((
            id.to_owned(),
            status,
            progress,
        ));
        Ok(())
    }
}

struct AllowInputPath;

impl InputFileAccess for AllowInputPath {
    fn resolve_allowed_file(&self, path: &str) -> Result<std::path::PathBuf> {
        Ok(path.into())
    }
}

#[derive(Default)]
struct InMemoryTranscripts(Mutex<Option<Transcript>>);

impl TranscriptRepository for InMemoryTranscripts {
    fn save(&self, _path: &Path, transcript: &Transcript) -> Result<()> {
        *self.0.lock().expect("transcripts mutex") = Some(transcript.clone());
        Ok(())
    }

    fn load(&self, _path: &Path) -> Result<Transcript> {
        self.0
            .lock()
            .expect("transcripts mutex")
            .clone()
            .ok_or_else(|| anyhow::anyhow!("transcript not found"))
    }
}

struct ProgressReportingMedia;

impl MediaProcessor for ProgressReportingMedia {
    fn inspect_media(&self, _input: &Path) -> Result<MediaMetadata> {
        Ok(MediaMetadata {
            container: "mp3".into(),
            audio_codec: Some("mp3".into()),
            audio_streams: 1,
        })
    }

    fn convert_video_to_mp3(&self, _input: &Path, _output: &Path) -> Result<()> {
        bail!("conversion was not expected in this use case")
    }

    fn transcribe_portuguese(
        &self,
        _input: &Path,
        operation_id: &str,
        progress: Arc<dyn ProgressReporter>,
    ) -> Result<Transcript> {
        progress.report_progress(50);
        Ok(Transcript {
            operation_id: operation_id.to_owned(),
            language: "pt-BR".into(),
            text: "A sample transcript.".into(),
            segments: vec![],
        })
    }
}

fn completed_transcription(repository: &InMemoryOperations) -> Operation {
    let mut operation = repository
        .create(
            OperationKind::Transcription,
            Path::new("/audio/source.mp3"),
            None,
        )
        .expect("create operation");
    repository
        .update(&operation.id, OperationStatus::Completed, 100, None)
        .expect("complete operation");
    operation.status = OperationStatus::Completed;
    operation
}

#[test]
fn starting_transcription_returns_a_queued_operation_for_the_resolved_input() {
    let repository = InMemoryOperations::default();
    let operation = application::start_transcription(
        &repository,
        &AllowInputPath,
        &ProgressReportingMedia,
        "/audio/talk.mp3",
        "mp3",
    )
    .expect("start transcription");

    assert_eq!(operation.kind, OperationKind::Transcription);
    assert_eq!(operation.input_path, "/audio/talk.mp3");
    assert_eq!(operation.status, OperationStatus::Queued);
    assert_eq!(operation.progress, 0);
}

#[test]
fn format_disagreement_aborts_before_creating_an_operation() {
    let repository = InMemoryOperations::default();
    let error = application::start_transcription(
        &repository,
        &AllowInputPath,
        &ProgressReportingMedia,
        "/audio/talk.mp3",
        "mp4",
    )
    .expect_err("mismatched metadata must abort");

    let message = format!("{error:#}");
    assert!(message.contains("expected format 'mp4'"));
    assert!(message.contains("extension is '.mp3'"));
    assert!(message.contains("detected container 'mp3'"));
    assert!(message.contains("audio streams: 1"));
    assert!(
        repository
            .operations
            .lock()
            .expect("operations mutex")
            .is_empty()
    );
}

#[test]
fn unsupported_expected_format_is_rejected_before_metadata_inspection() {
    let repository = InMemoryOperations::default();
    let error = application::start_transcription(
        &repository,
        &AllowInputPath,
        &ProgressReportingMedia,
        "/audio/talk.mp3",
        "wav",
    )
    .expect_err("unsupported expected format must abort");
    assert!(error.to_string().contains("expected 'mp3' or 'mp4'"));
    assert!(
        repository
            .operations
            .lock()
            .expect("operations mutex")
            .is_empty()
    );
}

#[test]
fn reprocessing_creates_a_new_operation_linked_to_the_original() {
    let repository = InMemoryOperations::default();
    let original = repository
        .create(
            OperationKind::Conversion,
            Path::new("/video/source.mov"),
            None,
        )
        .expect("create original operation");

    let retry = application::reprocess_operation(&repository, &AllowInputPath, &original.id)
        .expect("reprocess operation");

    assert_ne!(retry.id, original.id);
    assert_eq!(retry.kind, OperationKind::Conversion);
    assert_eq!(retry.input_path, original.input_path);
    assert_eq!(retry.parent_id.as_deref(), Some(original.id.as_str()));
    assert_eq!(retry.status, OperationStatus::Queued);
}

#[test]
fn transcript_pages_preserve_segment_order_and_return_a_continuation_offset() {
    let operations = InMemoryOperations::default();
    let operation = completed_transcription(&operations);
    let transcripts = InMemoryTranscripts::default();
    *transcripts.0.lock().expect("transcripts mutex") = Some(Transcript {
        operation_id: operation.id.clone(),
        language: "pt-BR".into(),
        text: "Primeira frase. Segunda frase.".into(),
        segments: vec![
            TranscriptSegment {
                index: 0,
                start_ms: 0,
                end_ms: 900,
                text: "Primeira frase.".into(),
            },
            TranscriptSegment {
                index: 1,
                start_ms: 900,
                end_ms: 1_800,
                text: "Segunda frase.".into(),
            },
            TranscriptSegment {
                index: 2,
                start_ms: 1_800,
                end_ms: 2_600,
                text: "Terceira frase.".into(),
            },
        ],
    });

    let page: TranscriptPage =
        application::transcript_page(&operations, &transcripts, &operation.id, 1, 1)
            .expect("read transcript page");

    assert_eq!(page.offset, 1);
    assert_eq!(page.segments[0].text, "Segunda frase.");
    assert_eq!(page.next_offset, Some(2));
    assert!(!page.done);
}

#[test]
fn conversion_status_is_scoped_to_conversion_operations() {
    let repository = InMemoryOperations::default();
    repository
        .create(
            OperationKind::Conversion,
            Path::new("/video/clip.mov"),
            None,
        )
        .expect("create conversion");
    repository
        .create(
            OperationKind::Transcription,
            Path::new("/audio/clip.mp3"),
            None,
        )
        .expect("create transcription");

    let operations = application::conversion_status_by_date(
        &repository,
        NaiveDate::from_ymd_opt(2026, 11, 9).expect("valid date"),
    )
    .expect("list conversions");

    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].kind, OperationKind::Conversion);

    let prior_day = application::conversion_status_by_date(
        &repository,
        NaiveDate::from_ymd_opt(2026, 11, 8).expect("valid date"),
    )
    .expect("list conversions on a different date");
    assert!(prior_day.is_empty());
}

#[test]
fn transcription_status_persists_intermediate_whisper_progress() {
    let repository = Arc::new(InMemoryOperations::default());
    let operation = repository
        .create(
            OperationKind::Transcription,
            Path::new("/audio/talk.mp3"),
            None,
        )
        .expect("create transcription");

    let progress = Arc::new(OperationProgressReporter::new(
        repository.clone(),
        operation.id.clone(),
    ));
    application::process_operation(
        &*repository,
        &ProgressReportingMedia,
        &InMemoryTranscripts::default(),
        &operation,
        progress,
    )
    .expect("process transcription");

    let updates = repository.updates.lock().expect("operation updates mutex");
    assert!(updates.contains(&(operation.id.clone(), OperationStatus::Processing, 50)));
    assert_eq!(updates.last().unwrap().1, OperationStatus::Completed);
}
