# ADR 0002: Use a Lightweight Hexagonal Architecture

- Status: Accepted
- Date: 2026-11-09

## Context

The service has replaceable technical details: MCP transport, persistence, filesystem policy, media processing, and transcript serialization. The use cases should remain independent of those choices.

## Decision

Keep a small domain module, define outbound port traits, implement use cases against those ports, and place concrete implementations in adapters. `main.rs` selects the adapters and wires the MCP server. The MCP adapter is the driving adapter; SQLite, filesystem, FFmpeg/whisper.cpp, and JSON storage are driven adapters.

Media inspection is part of the `MediaProcessor` port. The media adapter owns the `ffprobe` invocation, while the application owns expected-format comparison and abort-before-persistence behavior.

## Consequences

- Use cases can be exercised through port implementations without starting MCP stdio.
- Concrete storage and media implementations can be replaced without changing domain rules.
- The current file layout is intentionally a small modular monolith rather than separate crates.
- Adapter DTOs handle protocol and serialization concerns; domain types stay independent from Serde.

## Reference

The structure follows the ports/traits, adapters/implementations, and application/use-case separation in [Hexagonal Architecture in Rust: A Beginner's Guide](https://www.40tude.fr/docs/06_programmation/rust/024_hexagonal/hexagonal_lite.html).
