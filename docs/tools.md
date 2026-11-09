# Tools and Technology Choices

| Concern | Choice | Role |
| --- | --- | --- |
| Language and build | Rust, Cargo | MCP server, application core, adapters, and build configuration. |
| MCP server | `rmcp` | Rust SDK for MCP server tools over stdio. |
| Video/audio conversion | FFmpeg CLI | Local media decoding and MP3 encoding. Invoked with argument lists, not a shell. |
| Input metadata validation | `ffprobe` CLI | Detects container format and audio streams before transcription; must be available on `PATH` or configured through `FFPROBE_BIN`. |
| Speech recognition | `whisper-rs` + `whisper.cpp` | Local inference through Rust bindings to the whisper.cpp engine. |
| Model | Whisper `base` GGML | Offline Portuguese speech recognition model at `models/ggml-base.bin`. |
| Operation persistence | SQLite via `rusqlite` | Durable operation state and date/ID lookups. |
| Transcript artifacts | JSON via `serde_json` | Full transcript persistence and MCP response serialization. |
| Async work | Tokio | MCP stdio runtime and background job scheduling; CPU inference runs on a blocking worker. |
| Time and IDs | `chrono`, `uuid` | Local timestamps, date queries, and operation identifiers. |

## Model-format decision

The existing `models/base.pt` is a PyTorch checkpoint used by the original Python prototype. whisper.cpp expects a converted GGML model, so the Rust server uses `models/ggml-base.bin`. Keeping inference inside the Rust process through `whisper-rs` avoids a Python subprocess and external transcription API. The `whisper.cpp` engine itself is native C/C++ exposed through the Rust binding.

## Runtime requirements

Rust 1.88 or newer, FFmpeg/ffprobe on `PATH`, and the GGML model. Configure `MCP_AUDIO_MODEL`, `MCP_AUDIO_DATA_DIR`, `MCP_AUDIO_ALLOWED_ROOTS`, `FFMPEG_BIN`, or `FFPROBE_BIN` to override defaults. Model download and client configuration steps are in the root README.
