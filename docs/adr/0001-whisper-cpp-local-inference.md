# ADR 0001: Use whisper.cpp for Local Inference

- Status: Accepted
- Date: 2026-11-09

## Context

The service must transcribe Portuguese audio locally, with the MCP orchestration and tools implemented in Rust. The project already contains an OpenAI Whisper PyTorch checkpoint and Python script, but the server should not need a Python runtime or a remote transcription API.

## Decision

Use `whisper-rs` to call the whisper.cpp engine from Rust and ship/configure a GGML model (`ggml-base.bin`). Configure language `pt` and disable translation. Store the transcript language label as `pt-BR`.

The MCP caller declares `expected_format` as `mp3` or `mp4`; local `ffprobe` checks the extension and detected container before inference. MP4 files must contain an audio stream, and MP3 files must contain an MP3 audio codec. Discrepancies abort before operation creation.

## Consequences

- Inference stays local and can be called from the Rust application adapter.
- The existing `base.pt` file cannot be loaded by whisper.cpp and remains only for the prior Python prototype.
- Model downloads are format-specific; the GGML model is a separate local artifact.
- whisper.cpp's engine is native C/C++ under the Rust binding.
- The currently pinned binding version builds in this project. Review upstream maintenance status when upgrading the binding.

## Alternatives considered

- OpenAI Whisper Python package: has a PyTorch runtime and uses the existing `.pt` model format, but adds a Python implementation/runtime boundary to the Rust server.
- Remote transcription API: avoids local model packaging but conflicts with the offline/local processing requirement.
