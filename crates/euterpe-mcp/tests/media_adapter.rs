use std::{fs, path::Path, process::Command, sync::Arc};

use euterpe::{
    adapters::LocalMediaProcessor,
    ports::{MediaProcessor, ProgressReporter},
};
use uuid::Uuid;

#[test]
fn a_local_video_is_converted_to_an_mp3_file() {
    let work_dir = std::env::temp_dir().join(format!("mcp-media-test-{}", Uuid::new_v4()));
    fs::create_dir_all(&work_dir).expect("create temporary media directory");
    let input = work_dir.join("fixture.mp4");
    let output = work_dir.join("converted.mp3");
    let fixture_status = Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x64:d=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-shortest",
            "-c:v",
            "mpeg4",
            "-c:a",
            "aac",
        ])
        .arg(&input)
        .status()
        .expect("FFmpeg must be installed to create the media fixture");
    assert!(
        fixture_status.success(),
        "FFmpeg should create the fixture video"
    );

    let processor = LocalMediaProcessor::new("ffmpeg".into(), "/models/unused.ggml".into());
    processor
        .convert_video_to_mp3(&input, &output)
        .expect("convert the video through the media port");

    assert!(output.is_file());
    assert!(fs::metadata(&output).expect("read output metadata").len() > 0);
    let metadata = processor
        .inspect_media(&input)
        .expect("inspect MP4 fixture");
    assert!(
        metadata
            .container
            .split(',')
            .any(|format| format == "mov" || format == "mp4")
    );
    assert!(metadata.audio_streams > 0);
    let mp3_metadata = processor
        .inspect_media(&output)
        .expect("inspect MP3 output");
    assert!(
        mp3_metadata
            .container
            .split(',')
            .any(|format| format == "mp3")
    );
    assert_eq!(mp3_metadata.audio_codec.as_deref(), Some("mp3"));
    fs::remove_dir_all(work_dir).expect("remove temporary media directory");
}

#[test]
fn transcription_reports_a_missing_local_model_before_decoding_audio() {
    let processor = LocalMediaProcessor::new(
        "ffmpeg-that-does-not-exist".into(),
        "/models/missing-whisper.ggml".into(),
    );

    let error = processor
        .transcribe_portuguese(
            Path::new("/audio/voice.mp3"),
            "operation-1",
            Arc::new(IgnoreProgress),
        )
        .expect_err("missing local model should fail clearly");

    assert!(error.to_string().contains("local model not found"));
}

#[test]
fn video_conversion_reports_when_ffmpeg_cannot_be_started() {
    let processor = LocalMediaProcessor::new(
        "ffmpeg-that-does-not-exist".into(),
        "/models/unused.ggml".into(),
    );

    let error = processor
        .convert_video_to_mp3(Path::new("/video/clip.mov"), Path::new("/output/clip.mp3"))
        .expect_err("missing FFmpeg should be surfaced to the caller");

    assert!(error.to_string().contains("failed to start FFmpeg"));
}

struct IgnoreProgress;

impl ProgressReporter for IgnoreProgress {
    fn report_progress(&self, _progress: u8) {}
}
