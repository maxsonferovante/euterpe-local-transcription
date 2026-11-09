# Graph Report - mcp-audio-transcription-prototype  (2026-11-09)

## Corpus Check
- 32 files · ~26,967 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 336 nodes · 586 edges · 56 communities (18 shown, 38 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- anyhow
- application_use_cases.rs
- mcp.rs
- euterpe-cli/src/main.rs
- McpAdapter
- Operation
- What You Must Do When Invoked
- Euterpe Project Description
- Result
- Code of Conduct Policy
- MCP Protocol Usage
- euterpe-cli
- euterpe-mcp
- Euterpe
- media_adapter.rs
- graphify reference: extra exports and benchmark
- graphify reference: query, path, explain
- ADR 0002: Use a Lightweight Hexagonal Architecture
- graphify reference: commit hook and native CLAUDE.md integration
- graphify reference: add a URL and watch a folder
- graphify reference: incremental update and cluster-only
- graphify reference: GitHub clone and cross-repo merge
- graphify reference: transcribe video and audio
- extraction-spec.md
- arc
- bail
- command
- context
- datetime
- env
- fs
- fullparams
- io
- jsontranscriptstore
- local
- localfileaccess
- localmediaprocessor
- map
- mediaprocessor
- operationrepository
- optionalextension
- params
- pathbuf
- samplingstrategy
- schemars
- serviceext
- sqliteoperationrepository
- stdio
- tool
- tool_router
- transcriptpage
- value
- whispercontext
- whispercontextparameters
- write

## God Nodes (most connected - your core abstractions)
1. `Operation` - 23 edges
2. `OperationRepository` - 17 edges
3. `McpAdapter` - 16 edges
4. `install()` - 12 edges
5. `InMemoryOperations` - 12 edges
6. `What You Must Do When Invoked` - 12 edges
7. `SqliteOperationRepository` - 11 edges
8. `LocalMediaProcessor` - 11 edges
9. `transcribe_sync()` - 10 edges
10. `InputFileAccess` - 10 edges

## Surprising Connections (you probably didn't know these)
- `Euterpe Project Description` --uses--> `FFmpeg Media Tool`  [EXTRACTED]
  README.md → docs/tools.md
- `Euterpe Project Description` --uses--> `Hexagonal Architecture Pattern`  [EXTRACTED]
  README.md → docs/arch.md
- `Euterpe Project Description` --uses--> `Rust Language Choice`  [EXTRACTED]
  README.md → docs/tools.md
- `Euterpe Project Description` --uses--> `Whisper-RS Binding`  [EXTRACTED]
  README.md → docs/adr/0001-whisper-cpp-local-inference.md
- `Euterpe Project Description` --references--> `Graphify Agent Rules`  [EXTRACTED]
  README.md → AGENTS.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Hexagonal Architecture Layers** — docs_domain_layer_concepts, port_traits_interface, application_use_cases [EXTRACTED 1.00]
- **Port Implementations Pattern** — sqlite_persistence_layer, ffmpeg_media_tool, whisper_rs_binding [EXTRACTED 1.00]

## Communities (56 total, 38 thin omitted)

### Community 0 - "anyhow"
Cohesion: 0.33
Nodes (5): anyhow, main(), Result, info, mcpadapter

### Community 1 - "application_use_cases.rs"
Cohesion: 0.09
Nodes (24): OperationKind, OperationStatus, MediaMetadata, Option, String, AllowInputPath, completed_transcription(), conversion_status_is_scoped_to_conversion_operations() (+16 more)

### Community 2 - "mcp.rs"
Cohesion: 0.13
Nodes (24): ChunkArgs, DateArgs, OperationDto, PathArgs, Arc, From, Option, Self (+16 more)

### Community 3 - "euterpe-cli/src/main.rs"
Cohesion: 0.20
Nodes (24): configure_claude(), configure_codex(), copy_executable(), home(), init(), install(), install_service(), locate_model() (+16 more)

### Community 4 - "McpAdapter"
Cohesion: 0.42
Nodes (5): CallToolResult, IdArgs, McpAdapter, Display, Parameters

### Community 5 - "Operation"
Cohesion: 0.20
Nodes (24): conversion_status_by_date(), process_operation(), reprocess_operation(), Arc, NaiveDate, Result, String, Transcript (+16 more)

### Community 6 - "What You Must Do When Invoked"
Cohesion: 0.08
Nodes (24): For /graphify add and --watch, For /graphify query, For the commit hook and native CLAUDE.md integration, For --update and --cluster-only, /graphify, Honesty Rules, Interpreter guard for subcommands, Part A - Structural extraction for code files (+16 more)

### Community 7 - "Euterpe Project Description"
Cohesion: 0.18
Nodes (13): Graphify Agent Rules, Application Use Cases, Cargo Build System, Domain Layer Concepts, FFmpeg Media Tool, GGML Model Format, Graph JSON Artifact, Hexagonal Architecture Pattern (+5 more)

### Community 8 - "Result"
Cohesion: 0.12
Nodes (26): Connection, inspect_media(), JsonTranscriptStore, LocalFileAccess, LocalMediaProcessor, OperationProgressReporter, row_to_operation(), Arc (+18 more)

### Community 14 - "Euterpe"
Cohesion: 0.11
Nodes (17): Acceptance criteria, Operation lifecycle, Out of scope, Product Specification, Purpose, Scope, Transcript contract, User-visible operations (+9 more)

### Community 15 - "media_adapter.rs"
Cohesion: 0.20
Nodes (5): IgnoreProgress, completed_operations_remain_available_after_reopening_the_repository(), PathBuf, temporary_data_dir(), uuid

### Community 16 - "graphify reference: extra exports and benchmark"
Cohesion: 0.22
Nodes (8): graphify reference: extra exports and benchmark, Step 6b - Wiki (only if --wiki flag), Step 7 - Neo4j export (only if --neo4j or --neo4j-push flag), Step 7a - FalkorDB export (only if --falkordb or --falkordb-push flag), Step 7b - SVG export (only if --svg flag), Step 7c - GraphML export (only if --graphml flag), Step 7d - MCP server (only if --mcp flag), Step 8 - Token reduction benchmark (only if total_words > 5000)

### Community 17 - "graphify reference: query, path, explain"
Cohesion: 0.33
Nodes (5): For /graphify explain, For /graphify path, graphify reference: query, path, explain, Step 0 — Constrained query expansion (REQUIRED before traversal), Step 1 — Traversal

### Community 18 - "ADR 0002: Use a Lightweight Hexagonal Architecture"
Cohesion: 0.33
Nodes (5): ADR 0002: Use a Lightweight Hexagonal Architecture, Consequences, Context, Decision, Reference

### Community 19 - "graphify reference: commit hook and native CLAUDE.md integration"
Cohesion: 0.40
Nodes (4): For git commit hook, For native CLAUDE.md integration, graphify reference: commit hook and native CLAUDE.md integration, Source-read freshness

### Community 20 - "graphify reference: add a URL and watch a folder"
Cohesion: 0.50
Nodes (3): For /graphify add, For --watch, graphify reference: add a URL and watch a folder

### Community 21 - "graphify reference: incremental update and cluster-only"
Cohesion: 0.50
Nodes (3): For --cluster-only, For --update (incremental re-extraction), graphify reference: incremental update and cluster-only

## Knowledge Gaps
- **73 isolated node(s):** `Usage`, `What graphify is for`, `Step 0 - GitHub repos and multi-path merge (only if a URL or several paths)`, `Step 1 - Ensure graphify is installed`, `Step 2 - Detect files` (+68 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **38 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Operation` connect `Operation` to `Result`, `application_use_cases.rs`, `mcp.rs`, `McpAdapter`?**
  _High betweenness centrality (0.058) - this node is a cross-community bridge._
- **Why does `McpAdapter` connect `McpAdapter` to `mcp.rs`, `Operation`?**
  _High betweenness centrality (0.038) - this node is a cross-community bridge._
- **Why does `OperationRepository` connect `Operation` to `Result`, `application_use_cases.rs`, `McpAdapter`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._
- **What connects `Usage`, `What graphify is for`, `Step 0 - GitHub repos and multi-path merge (only if a URL or several paths)` to the rest of the system?**
  _73 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `application_use_cases.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.09102564102564102 - nodes in this community are weakly interconnected._
- **Should `mcp.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.12873563218390804 - nodes in this community are weakly interconnected._
- **Should `What You Must Do When Invoked` be split into smaller, more focused modules?**
  _Cohesion score 0.08 - nodes in this community are weakly interconnected._