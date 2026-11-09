use std::{env, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use euterpe::adapters::mcp::McpAdapter;
use euterpe::adapters::{
    JsonTranscriptStore, LocalFileAccess, LocalMediaProcessor, SqliteOperationRepository,
};
use rmcp::{ServiceExt, transport::stdio};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let base = env::current_exe()?
        .parent()
        .context("server executable has no parent directory")?
        .to_path_buf();
    let data_dir = env::var_os("MCP_AUDIO_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("runtime"));
    std::fs::create_dir_all(data_dir.join("operations"))?;

    let allowed_roots = env::var_os("MCP_AUDIO_ALLOWED_ROOTS")
        .map(|paths| env::split_paths(&paths).collect())
        .unwrap_or_else(|| {
            vec![
                env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or(base.clone()),
            ]
        });
    let repository = Arc::new(SqliteOperationRepository::new(data_dir.clone())?);
    let files = Arc::new(LocalFileAccess::new(allowed_roots, data_dir.clone())?);
    let media = Arc::new(LocalMediaProcessor::new(
        env::var("FFMPEG_BIN").unwrap_or_else(|_| "ffmpeg".into()),
        env::var_os("MCP_AUDIO_MODEL")
            .map(PathBuf::from)
            .unwrap_or_else(|| base.join("../Resources/models/ggml-base.bin")),
    ));
    let transcripts = Arc::new(JsonTranscriptStore);

    info!(data_dir = %data_dir.display(), "starting local audio MCP server");
    let service = McpAdapter::new(repository, files, media, transcripts);
    let server = service
        .serve(stdio())
        .await
        .context("MCP stdio server start failed")?;
    server
        .waiting()
        .await
        .context("MCP server stopped with error")?;
    Ok(())
}
