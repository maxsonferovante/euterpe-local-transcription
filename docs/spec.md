# Product Specification

## Purpose

Provide a local MCP server that turns video and audio files into MP3 files and Brazilian Portuguese transcript JSON. The server must be usable from an MCP client over stdio and must not send media to a remote transcription service.

## Scope

- Video-to-MP3 conversion using a locally installed FFmpeg executable.
- Transcription accepts MP3 audio and MP4 video containing audio.
- The MCP caller supplies `expected_format` (`mp3` or `mp4`); before creating an operation, the server compares it with the file extension and `ffprobe` container/audio-stream metadata. Any mismatch aborts the request with details and does not enqueue work.
- Portuguese (`pt`) speech recognition with local model inference; transcript metadata is `pt-BR`.
- Persistent operation IDs and status across server restarts.
- Full transcript retrieval as JSON and incremental retrieval by segment pages.
- Reprocessing an operation from the original input file.
- Local filesystem inputs restricted to configured allowed roots.

## User-visible operations

1. A caller submits a local video path. The server validates it, creates a conversion operation, returns its ID and output path, and performs conversion asynchronously.
2. A caller asks for conversion operations on a local calendar date (`YYYY-MM-DD`). The server returns every conversion created on that date, including status and any failure detail.
3. A caller submits an audio/video path and its expected format (`mp3` or `mp4`). The server validates the extension, detected container, and presence of an audio stream before creating an operation. A discrepancy is reported with expected and detected values and aborts without scheduling work. A valid file creates a transcription operation and is processed asynchronously.
4. A caller retrieves transcription status by operation ID.
5. After completion, a caller retrieves either the complete transcript JSON or a page of timestamped segments. Pages include a `next_offset` or signal completion.
6. A caller reprocesses any known conversion or transcription operation. The server creates a new operation ID linked to the prior operation and uses its original input.

## Transcript contract

The JSON file contains `operation_id`, `language`, `text`, and `segments`. Each segment has a zero-based `index`, `start_ms`, `end_ms`, and `text`. The output language label is `pt-BR`; Whisper is configured with language code `pt` and translation disabled.

## Operation lifecycle

`queued` → `processing` → `completed` or `failed`. The database stores IDs, operation kind, original input, output path, parent ID, progress, error, and timestamps. During transcription, Whisper progress callbacks update the persisted percentage while inference is running; conversion currently reports lifecycle states without an encoding percentage. Jobs left queued or processing after a server restart are marked failed; automatic resume is outside the current scope.

## Acceptance criteria

- The server compiles as a Rust project and starts as an MCP stdio process.
- All listed operations are exposed as MCP tools.
- Conversion and transcription happen locally through the configured tools and model.
- MP3 and MP4 transcription requests require matching caller-declared format, file extension, and detected container; MP4 must contain an audio stream.
- Format discrepancies are returned before operation creation and include the declared format, extension, detected container, and audio-stream count; MP3 codec mismatches are also reported.
- Inputs outside configured roots are rejected; generated artifacts under the configured data directory can be used as later inputs.
- Status and output artifacts are persisted and can be retrieved by operation ID/date.
- Transcript pages preserve segment order and timestamps and expose a continuation offset.
- Reprocessing creates a new job without mutating the previous operation.
- Use cases depend on ports, not on SQLite, FFmpeg, whisper.cpp, or MCP types.

## Out of scope

- Remote transcription or video downloads.
- Live token streaming while inference is running; current incremental retrieval is paginated after transcript completion.
- Automatic retry/resume after a process restart.
- Speaker diarization, translation, and languages other than Portuguese.
- A user interface beyond MCP tools.
