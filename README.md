# Euterpe

## Why Euterpe?

Euterpe was one of the nine Muses of Greek mythology, traditionally associated with music and, in the classical period, lyric poetry. The name suits this project because it works with audio and turns speech into text. It gives the local transcription MCP a short, memorable identity inspired by the arts.

![Euterpe, muse of music](assets/euterpe.png)

An MCP server written in Rust that converts video to MP3 with FFmpeg and transcribes local MP3 audio or MP4 video audio with `whisper.cpp` through the `whisper-rs` Rust binding. Before transcription, the agent declares `expected_format` (`mp3` or `mp4`); the server checks the extension, container, and audio stream using `ffprobe` and aborts with detailed discrepancy information on any mismatch. Recognition is configured for Portuguese (`pt`); transcript metadata uses `pt-BR`. Audio stays on the local machine.

## Architecture

The project uses a lightweight hexagonal architecture:

- `crates/euterpe-mcp/src/domain.rs` contains operation and transcript concepts, without protocol or infrastructure dependencies.
- `crates/euterpe-mcp/src/ports.rs` defines traits for operation persistence, local file access, media processing, progress reporting, and transcript artifacts.
- `crates/euterpe-mcp/src/application.rs` implements use cases using only those ports.
- `crates/euterpe-mcp/src/adapters.rs` and `crates/euterpe-mcp/src/adapters/mcp.rs` provide SQLite, filesystem, FFmpeg/whisper.cpp, JSON, and MCP adapters.
- `crates/euterpe-mcp/src/main.rs` is the composition root that selects and connects the adapters.

See [the specification](docs/spec.md), [architecture diagram](docs/arch.md), [domain model](docs/domain.md), [technology inventory](docs/tools.md), and [architecture decisions](docs/adr/).

![Euterpe input validation and local transcription flow](docs/euterpe-transcription.svg)

## Requirements

- Rust 1.88 or newer
- FFmpeg and `ffprobe` available on `PATH`
- Local Whisper GGML model at `models/ggml-base.bin` by default

The previous PyTorch checkpoint at `models/base.pt` is not compatible with whisper.cpp. Download the GGML model once if it is not already present:

```sh
curl -L 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin' -o models/ggml-base.bin
```

## Configuration and launch

The server accepts files under the current working directory by default. Add explicit roots for other input folders, and override the model, FFmpeg executable, or storage directory as needed:

```sh
export MCP_AUDIO_ALLOWED_ROOTS="$PWD:/Users/your-user/Downloads"
export MCP_AUDIO_MODEL="$PWD/models/ggml-base.bin"
export MCP_AUDIO_DATA_DIR="$PWD/runtime"
# Optional: FFMPEG_BIN=/opt/homebrew/bin/ffmpeg
cargo run -p euterpe-mcp --release
```

Configure the MCP client to run `cargo run -p euterpe-mcp --release` with this project as its working directory. MCP messages use stdout; diagnostic logs use stderr.

## Installation CLI

Build both release binaries with `cargo build --release --workspace`, then run `target/release/euterpe install` with `target/release/euterpe-mcp` beside it. The installer places the app bundle with the Euterpe icon in `~/Applications/Euterpe.app` on macOS (or the user-local application directory elsewhere), creates the data directory, installs a per-user LaunchAgent on macOS, and asks whether to add the server to Codex and Claude. The Whisper model is copied when `models/ggml-base.bin` is present.

`euterpe init [DIR]` creates a project directory with `models/` and `runtime/operations/`. `euterpe uninstall` removes the installed app and service while preserving user data and agent configuration. The MCP connection remains stdio, so each coding agent starts its own MCP process when connecting.

## MCP tools

| Tool | Input | Result |
| --- | --- | --- |
| `convert_video_to_mp3` | `path` | Starts a background conversion and returns its ID, status, and output path |
| `conversion_status_by_date` | `date` (`YYYY-MM-DD`) | Lists conversions started on that local calendar date |
| `transcribe_mp3` | `path`, `expected_format` (`mp3` or `mp4`) | Verifies extension, detected container and audio stream/codec; a discrepancy aborts with details before an operation is created. Valid input starts background transcription and returns its ID |
| `transcription_status` | `operation_id` | Returns transcription status and progress |
| `get_transcript_json` | `operation_id` | Returns the full transcript JSON and saved file path |
| `get_transcript_chunk` | `operation_id`, `offset`, `limit` | Returns a segment page and `next_offset` for incremental retrieval |
| `reprocess_operation` | `operation_id` | Creates a new job for the same stage using its original input |

Transcript JSON contains the language, full text, and segments with millisecond timestamps. Incremental retrieval is paginated: request the next page using `next_offset`.

## Persistence and behavior

Operation state is stored in `runtime/operations.sqlite3`; output artifacts are stored in `runtime/operations/`. Jobs still queued or processing when the server restarts are marked failed rather than resumed. Transcription jobs are serialized so only one local Whisper inference runs at a time. Add an input directory to `MCP_AUDIO_ALLOWED_ROOTS` to process files outside the configured roots.

## Development

Run the automated checks with:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution and test guidance. This project is licensed under the MIT License; see [LICENSE](LICENSE).

---
