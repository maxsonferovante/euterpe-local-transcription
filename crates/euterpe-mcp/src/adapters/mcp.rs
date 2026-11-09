use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    adapters::OperationProgressReporter,
    application,
    domain::{Operation, Transcript, TranscriptPage, TranscriptSegment},
    ports::{InputFileAccess, MediaProcessor, OperationRepository, TranscriptRepository},
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PathArgs {
    path: String,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TranscriptionArgs {
    path: String,
    expected_format: String,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DateArgs {
    date: String,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct IdArgs {
    operation_id: String,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ChunkArgs {
    operation_id: String,
    offset: usize,
    limit: Option<usize>,
}

#[derive(Clone)]
pub struct McpAdapter {
    repository: Arc<dyn OperationRepository>,
    files: Arc<dyn InputFileAccess>,
    media: Arc<dyn MediaProcessor>,
    transcripts: Arc<dyn TranscriptRepository>,
}

impl McpAdapter {
    pub fn new(
        repository: Arc<dyn OperationRepository>,
        files: Arc<dyn InputFileAccess>,
        media: Arc<dyn MediaProcessor>,
        transcripts: Arc<dyn TranscriptRepository>,
    ) -> Self {
        Self {
            repository,
            files,
            media,
            transcripts,
        }
    }

    fn schedule(&self, operation: Operation) {
        let repository = Arc::clone(&self.repository);
        let media = Arc::clone(&self.media);
        let transcripts = Arc::clone(&self.transcripts);
        let id = operation.id.clone();
        let progress = Arc::new(OperationProgressReporter::new(
            Arc::clone(&repository),
            id.clone(),
        ));
        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                application::process_operation(
                    &*repository,
                    &*media,
                    &*transcripts,
                    &operation,
                    progress,
                )
            })
            .await;
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => tracing::error!(operation_id = %id, %error, "operation failed"),
                Err(error) => {
                    tracing::error!(operation_id = %id, %error, "operation worker failed")
                }
            }
        });
    }

    fn error(error: impl std::fmt::Display) -> CallToolResult {
        CallToolResult::structured_error(json!({"error": error.to_string()}))
    }
}

#[tool_router(server_handler)]
impl McpAdapter {
    #[tool(
        description = "Start a local video-to-MP3 conversion. Accepts an allowed absolute or relative file path and returns the operation ID and status."
    )]
    async fn convert_video_to_mp3(&self, Parameters(args): Parameters<PathArgs>) -> CallToolResult {
        match application::start_conversion(&*self.repository, &*self.files, &args.path) {
            Ok(operation) => {
                self.schedule(operation.clone());
                CallToolResult::structured(json!(OperationDto::from(operation)))
            }
            Err(error) => Self::error(format!("{error:#}")),
        }
    }

    #[tool(description = "List all conversions started on the supplied local date (YYYY-MM-DD).")]
    async fn conversion_status_by_date(
        &self,
        Parameters(args): Parameters<DateArgs>,
    ) -> CallToolResult {
        let result = NaiveDate::parse_from_str(&args.date, "%Y-%m-%d")
            .context("use YYYY-MM-DD")
            .and_then(|date| application::conversion_status_by_date(&*self.repository, date));
        match result {
            Ok(operations) => CallToolResult::structured(
                json!({"date": args.date, "operations": operations.into_iter().map(OperationDto::from).collect::<Vec<_>>()}),
            ),
            Err(error) => Self::error(format!("{error:#}")),
        }
    }

    #[tool(
        description = "Validate the agent-declared expected_format (mp3 or mp4) against the file extension and ffprobe container/audio metadata, then start Portuguese transcription. Any mismatch aborts before creating an operation and returns detailed detected metadata."
    )]
    async fn transcribe_mp3(
        &self,
        Parameters(args): Parameters<TranscriptionArgs>,
    ) -> CallToolResult {
        match application::start_transcription(
            &*self.repository,
            &*self.files,
            &*self.media,
            &args.path,
            &args.expected_format,
        ) {
            Ok(operation) => {
                self.schedule(operation.clone());
                CallToolResult::structured(json!(OperationDto::from(operation)))
            }
            Err(error) => Self::error(format!("{error:#}")),
        }
    }

    #[tool(description = "Get the status and progress of a transcription by operation ID.")]
    async fn transcription_status(&self, Parameters(args): Parameters<IdArgs>) -> CallToolResult {
        match application::transcription_status(&*self.repository, &args.operation_id) {
            Ok(operation) => CallToolResult::structured(json!(OperationDto::from(operation))),
            Err(error) => Self::error(format!("{error:#}")),
        }
    }

    #[tool(
        description = "Return the complete JSON for a finished transcription and the local file path."
    )]
    async fn get_transcript_json(&self, Parameters(args): Parameters<IdArgs>) -> CallToolResult {
        match application::transcript_json(
            &*self.repository,
            &*self.transcripts,
            &args.operation_id,
        ) {
            Ok((file_path, transcript)) => CallToolResult::structured(
                json!({"file_path": file_path, "transcript": TranscriptDto::from(transcript)}),
            ),
            Err(error) => Self::error(format!("{error:#}")),
        }
    }

    #[tool(
        description = "Read one page of transcript segments; use next_offset to continue incremental retrieval."
    )]
    async fn get_transcript_chunk(
        &self,
        Parameters(args): Parameters<ChunkArgs>,
    ) -> CallToolResult {
        match application::transcript_page(
            &*self.repository,
            &*self.transcripts,
            &args.operation_id,
            args.offset,
            args.limit.unwrap_or(100),
        ) {
            Ok(page) => CallToolResult::structured(json!(TranscriptPageDto::from(page))),
            Err(error) => Self::error(format!("{error:#}")),
        }
    }

    #[tool(
        description = "Reprocess any operation from its saved input file and create a new ID linked to the original."
    )]
    async fn reprocess_operation(&self, Parameters(args): Parameters<IdArgs>) -> CallToolResult {
        match application::reprocess_operation(&*self.repository, &*self.files, &args.operation_id)
        {
            Ok(operation) => {
                self.schedule(operation.clone());
                CallToolResult::structured(json!(OperationDto::from(operation)))
            }
            Err(error) => Self::error(format!("{error:#}")),
        }
    }
}

#[derive(serde::Serialize)]
struct OperationDto {
    id: String,
    kind: &'static str,
    input_path: String,
    output_path: String,
    parent_id: Option<String>,
    status: &'static str,
    progress: u8,
    error: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<Operation> for OperationDto {
    fn from(op: Operation) -> Self {
        Self {
            id: op.id,
            kind: op.kind.as_str(),
            input_path: op.input_path,
            output_path: op.output_path,
            parent_id: op.parent_id,
            status: op.status.as_str(),
            progress: op.progress,
            error: op.error,
            created_at: op.created_at,
            updated_at: op.updated_at,
        }
    }
}

#[derive(serde::Serialize)]
struct TranscriptDto {
    operation_id: String,
    language: String,
    text: String,
    segments: Vec<SegmentDto>,
}

impl From<Transcript> for TranscriptDto {
    fn from(transcript: Transcript) -> Self {
        Self {
            operation_id: transcript.operation_id,
            language: transcript.language,
            text: transcript.text,
            segments: transcript
                .segments
                .into_iter()
                .map(SegmentDto::from)
                .collect(),
        }
    }
}

#[derive(serde::Serialize)]
struct SegmentDto {
    index: usize,
    start_ms: i64,
    end_ms: i64,
    text: String,
}

impl From<TranscriptSegment> for SegmentDto {
    fn from(segment: TranscriptSegment) -> Self {
        Self {
            index: segment.index,
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            text: segment.text,
        }
    }
}

#[derive(serde::Serialize)]
struct TranscriptPageDto {
    operation_id: String,
    offset: usize,
    segments: Vec<SegmentDto>,
    next_offset: Option<usize>,
    done: bool,
}

impl From<TranscriptPage> for TranscriptPageDto {
    fn from(page: TranscriptPage) -> Self {
        Self {
            operation_id: page.operation_id,
            offset: page.offset,
            segments: page.segments.into_iter().map(SegmentDto::from).collect(),
            next_offset: page.next_offset,
            done: page.done,
        }
    }
}
