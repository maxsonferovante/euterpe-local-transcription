# Contributing

Thanks for helping improve the local audio transcription MCP. Project documentation and source code are maintained in English; the root README is bilingual (English and Brazilian Portuguese).

## Before making changes

- Read [the product specification](docs/spec.md), [architecture diagram](docs/arch.md), and relevant [ADRs](docs/adr/).
- Keep domain and application code independent of MCP, SQLite, FFmpeg, whisper.cpp, and filesystem implementation details.
- Keep the MCP transcription contract, README files, and specification aligned: callers pass `expected_format` (`mp3` or `mp4`), and the application validates extension plus media metadata before creating an operation. Discrepancy errors should include expected and detected details.
- For behavior changes, write a test through an agreed public seam before implementing the change (red → green → refactor).
- Avoid adding tests that assert internal calls or query persistence through a side channel. Prefer behavior through public use cases and adapter interfaces.

## Development checks

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

FFmpeg and the local GGML model are required for end-to-end media inference. Keep ordinary tests deterministic and offline by using boundary fakes or small local fixtures at agreed interfaces.

## Pull request guidance

- Describe the user-visible behavior and the reason for the change.
- Update the specification, architecture, domain guide, or ADRs when their contracts or decisions change.
- Include test commands and their results.
- Do not commit runtime databases, generated transcripts, build output, API secrets, or downloaded model binaries.
- Keep MCP protocol output on stdout and diagnostics on stderr.

## Community

Participation is subject to the [Code of Conduct](CODE_OF_CONDUCT.md).
