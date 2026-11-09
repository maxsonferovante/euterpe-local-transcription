#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Conversion,
    Transcription,
}

impl OperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Conversion => "conversion",
            Self::Transcription => "transcription",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    Queued,
    Processing,
    Completed,
    Failed,
}

impl OperationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub id: String,
    pub kind: OperationKind,
    pub input_path: String,
    pub output_path: String,
    pub parent_id: Option<String>,
    pub status: OperationStatus,
    pub progress: u8,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct TranscriptSegment {
    pub index: usize,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Transcript {
    pub operation_id: String,
    pub language: String,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
}

#[derive(Debug, Clone)]
pub struct TranscriptPage {
    pub operation_id: String,
    pub offset: usize,
    pub segments: Vec<TranscriptSegment>,
    pub next_offset: Option<usize>,
    pub done: bool,
}
