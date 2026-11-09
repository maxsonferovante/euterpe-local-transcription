use std::{path::Path, sync::Arc};

use anyhow::Result;
use chrono::NaiveDate;

use crate::domain::{Operation, OperationKind, OperationStatus, Transcript};

/// Outbound port for persistent operation state.
pub trait OperationRepository: Send + Sync {
    fn create(
        &self,
        kind: OperationKind,
        input_path: &Path,
        parent_id: Option<&str>,
    ) -> Result<Operation>;
    fn find(&self, id: &str) -> Result<Option<Operation>>;
    fn list_conversions_on(&self, date: NaiveDate) -> Result<Vec<Operation>>;
    fn update(
        &self,
        id: &str,
        status: OperationStatus,
        progress: u8,
        error: Option<&str>,
    ) -> Result<()>;
}

/// Outbound port for resolving and validating user supplied local files.
pub trait InputFileAccess: Send + Sync {
    fn resolve_allowed_file(&self, path: &str) -> Result<std::path::PathBuf>;
}

/// Outbound port for progress updates from long-running media work.
pub trait ProgressReporter: Send + Sync {
    fn report_progress(&self, progress: u8);
}

/// Outbound port for local media processing.
pub trait MediaProcessor: Send + Sync {
    fn convert_video_to_mp3(&self, input: &Path, output: &Path) -> Result<()>;
    fn inspect_media(&self, input: &Path) -> Result<MediaMetadata>;
    fn transcribe_portuguese(
        &self,
        input: &Path,
        operation_id: &str,
        progress: Arc<dyn ProgressReporter>,
    ) -> Result<Transcript>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaMetadata {
    pub container: String,
    pub audio_codec: Option<String>,
    pub audio_streams: usize,
}

/// Outbound port for storing and loading transcript artifacts.
pub trait TranscriptRepository: Send + Sync {
    fn save(&self, path: &Path, transcript: &Transcript) -> Result<()>;
    fn load(&self, path: &Path) -> Result<Transcript>;
}
