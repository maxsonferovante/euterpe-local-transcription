use std::{fs, path::Path};

use chrono::{DateTime, Local};
use euterpe::{
    adapters::SqliteOperationRepository,
    domain::{OperationKind, OperationStatus},
    ports::OperationRepository,
};
use uuid::Uuid;

fn temporary_data_dir() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("mcp-audio-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).expect("create temporary data directory");
    path
}

#[test]
fn completed_operations_remain_available_after_reopening_the_repository() {
    let data_dir = temporary_data_dir();
    let created = {
        let repository = SqliteOperationRepository::new(data_dir.clone()).expect("open repository");
        let operation = repository
            .create(
                OperationKind::Transcription,
                Path::new("/input/interview.mp3"),
                None,
            )
            .expect("create operation");
        repository
            .update(&operation.id, OperationStatus::Completed, 100, None)
            .expect("complete operation");
        repository
            .create(
                OperationKind::Conversion,
                Path::new("/input/clip.mov"),
                None,
            )
            .expect("create conversion");
        operation
    };

    let reopened = SqliteOperationRepository::new(data_dir.clone()).expect("reopen repository");
    let loaded = reopened
        .find(&created.id)
        .expect("query operation")
        .expect("operation exists");
    let operation_date = DateTime::parse_from_rfc3339(&created.created_at)
        .expect("RFC3339 timestamp")
        .with_timezone(&Local)
        .date_naive();
    let listed = reopened
        .list_conversions_on(operation_date)
        .expect("query conversions by date");

    assert_eq!(loaded.status, OperationStatus::Completed);
    assert_eq!(loaded.input_path, "/input/interview.mp3");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].kind, OperationKind::Conversion);
    fs::remove_dir_all(data_dir).expect("remove temporary data directory");
}
