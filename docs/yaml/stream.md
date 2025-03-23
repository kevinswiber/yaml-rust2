# YAML Streams

YAML streams are the highest-level construct in YAML, containing one or more YAML documents. This document outlines the specification, structure, and position tracking aspects of YAML streams in yaml-rust2.

## YAML 1.2.2 Specification

According to the YAML 1.2.2 specification:

- A YAML *stream* is a sequence of zero or more documents.
- Documents are separated by document boundary markers.
- Documents within a stream may have different versions and schema declarations.
- A YAML processor may choose to accept multiple documents in a stream, or only a single document.

### Stream Structure

```
stream   ::= STREAM-START (document | +DOCUMENT-END)* STREAM-END
```

Where:
- `STREAM-START` and `STREAM-END` mark the beginning and end of the character stream
- `document` is a complete YAML document
- `DOCUMENT-END` is the document end marker (`...`)

### Document Boundaries

Documents within a stream are separated by:

- Document start marker (`---`): Indicates the start of a new document
- Document end marker (`...`): Indicates the end of the current document

### Key Characteristics

- YAML streams support multiple independent documents in a single file/input
- Each document can have different schemas or YAML versions
- The start and end markers for documents are optional in some cases:
  - The first document in a stream does not need a start marker
  - The last document in a stream does not need an end marker
- Implicit document start happens with the first non-directive content 
- Explicit document end can be indicated with `...` followed by line break

## Implementation in yaml-rust2

In yaml-rust2, YAML streams are handled starting from the lowest level components:

### Scanner Level

The `Scanner` produces token streams with:

- `StreamStart` token with encoding information
- `DocumentStart` and `DocumentEnd` tokens for document boundaries
- `StreamEnd` token at the end of input

### Parser Level

The `Parser` processes tokens from the Scanner and emits:

- `StreamStart` event
- `DocumentStart` and `DocumentEnd` events
- `StreamEnd` event

The parser handles stream parsing through the `load` method, which supports:
- Single document mode (default)
- Multi-document mode (when `multi` parameter is `true`)

### Loader Level

The `YamlLoader` builds YAML objects from parser events:

- Stream events trigger appropriate state changes
- Multiple documents are collected into a `Vec<Yaml>`
- Each document is represented as its own `Yaml` value

## Position Tracking

Position tracking for streams uses the same mechanisms as other YAML constructs:

### Stream Position Span

- `StreamStart` is marked at the beginning of the input
- `StreamEnd` is marked at the end of the input
- The entire stream's position span encapsulates all its documents

### Document Boundary Positions

- Document start markers (`---`) are tracked with position information
- Document end markers (`...`) are tracked with position information
- Implicit document boundaries still have position information at their logical locations

### Implementation Details

In the `PositionTrackedLoader`, stream position tracking is implemented through:

- The `on_positioned_event` method processes events with their positions
- Stream positions are tracked as `PositionSpan` values
- The entire stream has a logical position that begins with `StreamStart` and ends with `StreamEnd`

## API Usage

### Loading a Single Document

```rust
let yaml_str = "key: value";
let docs = YamlLoader::load_from_str(yaml_str).unwrap();
// docs is Vec<Yaml> with a single document
```

### Loading Multiple Documents

```rust
let yaml_str = "---\nkey1: value1\n...\n---\nkey2: value2";
let docs = YamlLoader::load_from_str(yaml_str).unwrap();
// docs is Vec<Yaml> with two documents
```

### Position-Tracked Loading

```rust
let yaml_str = "---\nkey: value\n...";
let mut loader = PositionTrackedLoader::new(yaml_str);
let (docs, source_map) = loader.load_all().unwrap();
// docs contains the YAML documents
// source_map contains position information for all elements
```

## Common Challenges

### Multi-Document Handling

- Identifying when a document ends and another begins
- Tracking positions across document boundaries
- Handling empty documents (those containing only directives or comments)

### Implicit vs. Explicit Boundaries

- Implicit document start (first content in stream)
- Implicit document end (next document start or stream end)
- Explicit boundaries with markers (`---` and `...`)

### Position Edge Cases

- Empty streams (no documents)
- Streams with empty documents
- Streams with only directives
- Comments between documents

## Implementation Notes

### Parse Event Flow

The parsing process in yaml-rust2 follows this pattern:

1. Scanner tokenizes the input → `StreamStart`, document tokens, `StreamEnd`
2. Parser processes tokens into events → `StreamStart`, document events, `StreamEnd`
3. Loader processes events into YAML nodes → `Vec<Yaml>` with multiple documents

### Multi-Document Control

- The `multi` parameter in the Parser's `load` method controls multi-document handling
- When `multi` is `true`, the parser continues processing documents until `StreamEnd`
- When `multi` is `false`, the parser stops after the first document

## Example: Multi-Document YAML

```yaml
# First document
---
document: 1
data:
  - item1
  - item2
...
# Second document
---
document: 2
data: {key: value}
...
```

Position tracking for this example would include:
- Stream start at the beginning of the file
- Document start markers (`---`) with their exact positions
- Document end markers (`...`) with their exact positions
- Stream end at the end of the file
- Positions for all elements within each document

## Related Documentation

- [YAML Documents](document.md) - Structure and semantics of individual YAML documents
- [Position Tracked Loader](/docs/position_tracked_loader.md) - Details on position tracking implementation
- [Source Mapping](/docs/source_mapping.md) - How YAML nodes map to source positions