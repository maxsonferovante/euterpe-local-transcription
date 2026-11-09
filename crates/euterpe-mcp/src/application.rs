use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use chrono::NaiveDate;
use std::sync::Arc;

use crate::{
    domain::{Operation, OperationKind, OperationStatus, Transcript, TranscriptPage},
    ports::{
        InputFileAccess, MediaProcessor, OperationRepository, ProgressReporter,
        TranscriptRepository,
    },
};

pub fn start_conversion(
    repository: &dyn OperationRepository,
    files: &dyn InputFileAccess,
    path: &str,
) -> Result<Operation> {
    let input = files.resolve_allowed_file(path)?;
    repository.create(OperationKind::Conversion, &input, None)
}

pub fn start_transcription(
    repository: &dyn OperationRepository,
    files: &dyn InputFileAccess,
    media: &dyn MediaProcessor,
    path: &str,
    expected_format: &str,
) -> Result<Operation> {
    let input = files.resolve_allowed_file(path)?;
    let extension = input
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let expected = expected_format.to_ascii_lowercase();
    if !["mp3", "mp4"].contains(&expected.as_str()) {
        bail!("unsupported expected_format '{expected_format}'; expected 'mp3' or 'mp4'");
    }
    let metadata = media.inspect_media(&input)?;
    let container_matches = match expected.as_str() {
        "mp3" => metadata.container.split(',').any(|name| name == "mp3"),
        "mp4" => metadata
            .container
            .split(',')
            .any(|name| name == "mov" || name == "mp4" || name == "m4a"),
        _ => false,
    };
    let extension_matches = extension == expected;
    let audio_present = metadata.audio_streams > 0;
    if !extension_matches || !container_matches || !audio_present {
        bail!(
            "media validation failed for '{}': agent expected format '{}'; file extension is '.{}' ({}), detected container '{}' ({}), audio streams: {} ({}). Operation aborted before transcription.",
            input.display(),
            expected,
            extension,
            if extension_matches {
                "matches"
            } else {
                "mismatch"
            },
            metadata.container,
            if container_matches {
                "matches"
            } else {
                "mismatch"
            },
            metadata.audio_streams,
            if audio_present { "present" } else { "missing" }
        );
    }
    if expected == "mp3" && metadata.audio_codec.as_deref() != Some("mp3") {
        bail!(
            "media validation failed for '{}': agent expected an MP3 audio stream, but ffprobe detected codec '{}'; operation aborted before transcription.",
            input.display(),
            metadata.audio_codec.as_deref().unwrap_or("none")
        );
    }
    repository.create(OperationKind::Transcription, &input, None)
}

pub fn conversion_status_by_date(
    repository: &dyn OperationRepository,
    date: NaiveDate,
) -> Result<Vec<Operation>> {
    repository.list_conversions_on(date)
}

pub fn transcription_status(repository: &dyn OperationRepository, id: &str) -> Result<Operation> {
    let operation = repository
        .find(id)?
        .ok_or_else(|| anyhow!("transcription not found"))?;
    if operation.kind != OperationKind::Transcription {
        bail!("the supplied ID does not belong to a transcription");
    }
    Ok(operation)
}

pub fn transcript_json(
    repository: &dyn OperationRepository,
    transcripts: &dyn TranscriptRepository,
    id: &str,
) -> Result<(String, Transcript)> {
    let operation = transcription_status(repository, id)?;
    if operation.status != OperationStatus::Completed {
        bail!(
            "transcription is not complete yet (status: {})",
            operation.status.as_str()
        );
    }
    let transcript = transcripts
        .load(Path::new(&operation.output_path))
        .context("failed to load the transcript JSON file")?;
    Ok((operation.output_path, transcript))
}

pub fn transcript_page(
    repository: &dyn OperationRepository,
    transcripts: &dyn TranscriptRepository,
    id: &str,
    offset: usize,
    limit: usize,
) -> Result<TranscriptPage> {
    let (_, transcript) = transcript_json(repository, transcripts, id)?;
    let limit = limit.clamp(1, 500);
    let end = offset.saturating_add(limit).min(transcript.segments.len());
    let segments = transcript
        .segments
        .get(offset..end)
        .ok_or_else(|| anyhow!("offset is beyond the end of the transcript"))?
        .to_vec();
    Ok(TranscriptPage {
        operation_id: transcript.operation_id,
        offset,
        segments,
        next_offset: (end < transcript.segments.len()).then_some(end),
        done: end >= transcript.segments.len(),
    })
}

pub fn reprocess_operation(
    repository: &dyn OperationRepository,
    files: &dyn InputFileAccess,
    id: &str,
) -> Result<Operation> {
    let previous = repository
        .find(id)?
        .ok_or_else(|| anyhow!("operation not found"))?;
    let input = files.resolve_allowed_file(&previous.input_path)?;
    repository.create(previous.kind, &input, Some(&previous.id))
}

/// Runs one queued use case. Adapters are borrowed through ports; this function
/// knows neither SQLite nor FFmpeg/whisper.cpp implementations.
pub fn process_operation(
    repository: &dyn OperationRepository,
    media: &dyn MediaProcessor,
    transcripts: &dyn TranscriptRepository,
    operation: &Operation,
    progress: Arc<dyn ProgressReporter>,
) -> Result<()> {
    repository.update(&operation.id, OperationStatus::Processing, 1, None)?;
    let result = match operation.kind {
        OperationKind::Conversion => media.convert_video_to_mp3(
            Path::new(&operation.input_path),
            Path::new(&operation.output_path),
        ),
        OperationKind::Transcription => {
            let transcript = media.transcribe_portuguese(
                Path::new(&operation.input_path),
                &operation.id,
                progress,
            )?;
            repository.update(&operation.id, OperationStatus::Processing, 95, None)?;
            transcripts.save(Path::new(&operation.output_path), &transcript)
        }
    };
    match result {
        Ok(()) => repository.update(&operation.id, OperationStatus::Completed, 100, None),
        Err(error) => {
            let message = format!("{error:#}");
            let _ = repository.update(&operation.id, OperationStatus::Failed, 0, Some(&message));
            Err(error)
        }
    }
}
