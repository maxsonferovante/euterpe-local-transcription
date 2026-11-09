# Domain Model

## Vocabulary

| Term | Meaning |
| --- | --- |
| Operation | A persisted unit of work for converting a video or transcribing audio. |
| Conversion | An operation that produces an MP3 from a video input. |
| Transcription | An operation that produces timestamped Portuguese transcript JSON from an audio input. |
| Operation ID | A UUID identifying one immutable attempt. Reprocessing creates a new ID. |
| Parent operation | The earlier operation that a reprocessed operation refers to. |
| Transcript | Full text, language metadata, and ordered timestamped segments. |
| Expected media format | Caller declaration (`mp3` or `mp4`) checked against the path extension and detected media container before a transcription operation is created. |
| Media metadata | Container name, audio stream count, and first audio stream codec reported by `ffprobe`. MP4 requires an audio stream; MP3 requires an MP3 audio codec. |
| Transcript page | An ordered slice of transcript segments with a continuation offset. |

## Types and lifecycle

`OperationKind` is either `Conversion` or `Transcription`. `OperationStatus` is `Queued`, `Processing`, `Completed`, or `Failed`. Progress is an integer percentage from 0 through 100. The persisted operation also records its input and output paths, optional parent ID, error detail, and creation/update timestamps. During transcription, progress reflects Whisper's inference callback via the `ProgressReporter` port; conversion jobs expose lifecycle state without an encoding percentage.

The normal lifecycle is:

```text
Queued -> Processing -> Completed
                     -> Failed
```

Only a completed transcription can be read as transcript JSON or paged. Conversion status can be queried by the local creation date. A reprocess request creates a new operation, retains the source operation ID as its parent, and reads the original input path.

## Transcript shape

- Language metadata: `pt-BR`.
- Recognition language: `pt`; translation is disabled.
- Segment indexes start at zero and follow audio order.
- Timestamps are milliseconds from the start of the media.
- Full text is the concatenated segment text with surrounding whitespace trimmed.
- Pages are limited to 1–500 segments per request.

## Invariants

- Each operation has exactly one kind and one source input.
- A new attempt always gets a new ID; earlier operation records are not overwritten.
- A transcript artifact is written to a temporary sibling file and renamed into place after serialization.
- User-provided input paths must resolve inside an allowed root. The server's configured output directory is also accepted as an input root for chaining conversion into transcription.
