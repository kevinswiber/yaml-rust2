# YAML Tags

YAML tags are annotations that specify the type of a node. This document outlines the specification, syntax, and implementation details of YAML tags in yaml-rust2, including position tracking aspects.

## YAML 1.2.2 Specification

According to the YAML 1.2.2 specification:

- Tags identify the type of a node, allowing for application-specific interpretation
- Tags may be either explicit or implicit
- Tags use URI-based namespaces to ensure uniqueness and prevent collisions
- YAML includes predefined tag prefixes for common types

### Tag Syntax

Tags can appear in several forms:

1. **Verbatim tag**: `!<tag:example.com,2011:type>`
2. **Global tag**: `!!float`
3. **Local tag**: `!local`

The tag's structure consists of:
- A handle (e.g., `!!`, `!`, or a custom handle)
- A suffix (the remainder of the tag)

### Tag Resolution

Tags are resolved following these rules:

1. If an explicit tag is provided, it is used as-is
2. If no explicit tag is provided, an implicit tag is determined by:
   - Node style (flow vs. block)
   - Node content (scalar patterns)
   - Context (mapping key vs. value)

### Predefined Tag Handles

YAML 1.2 includes several predefined tag handles:

- The primary tag handle: `!` - local tags
- The secondary tag handle: `!!` - YAML's language-independent types
- Named tag handles: `!prefix!` - custom prefix registered via a TAG directive

## Implementation in yaml-rust2

### Data Structures

In yaml-rust2, tags are represented using the `Tag` struct:

```rust
pub struct Tag {
    /// The handle part of the tag (e.g., "!!" in "!!str")
    pub handle: String,
    /// The suffix part of the tag (e.g., "str" in "!!str")
    pub suffix: String,
}
```

Tags are included in events like:
- `Event::Scalar` - for scalar values
- `Event::SequenceStart` - for sequences
- `Event::MappingStart` - for mappings

### Parsing Process

1. **Scanner Level**:
   - The scanner detects tag tokens
   - Tag tokens include both handle and suffix
   - The token type is `TokenType::Tag(String, String)`

2. **Parser Level**:
   - The parser maintains a `tags` HashMap to store tag handles and prefixes
   - Tag directives update this HashMap
   - The parser creates events with tag information

3. **Loader Level**:
   - The YAML loader processes events and preserves tag information
   - Tags are associated with their respective nodes

### Tag Directive Handling

The parser supports tag directives via:

- The `tags` HashMap storing tag handle mappings
- Tag directives update these mappings (e.g., `%TAG !prefix! tag:example.com,2011:`)
- A `keep_tags` option controls whether tag directives are preserved across documents

## Position Tracking

Position tracking for tags follows these principles:

### Tag Position Information

- **Start Position**: At the `!` character of the tag
- **End Position**: After the last character of the tag name

### Position Tracking Implementation

In the `PositionTrackedLoader`, tag positions are tracked as part of the node they annotate:

- When a tagged scalar, sequence, or mapping is encountered, both the node's position and the tag's position are recorded
- The `PositionSpan` includes positions for the tag itself
- Source mapping can locate the exact position of tags in the source document

### Edge Cases

Position tracking handles several edge cases:

- Tags with URI-encoded characters
- Multi-line tags (rare but possible)
- Tags with verbatim notation (`!<...>`)
- Custom tag handles that use directives

## API Usage

### Working with Tags

```rust
let yaml_str = "!!str value";
let mut loader = PositionTrackedLoader::new(yaml_str);
let (docs, source_map) = loader.load_all().unwrap();

// Accessing tag information
if let Some(node) = docs.first() {
    if let Some(tag) = node.tag() {
        println!("Tag handle: {}, suffix: {}", tag.handle, tag.suffix);
    }
    
    // Get position of the tag
    if let Some(pos) = source_map.get_span(node) {
        println!("Tag position: start={:?}, end={:?}", pos.start, pos.end);
    }
}
```

### Customizing Tag Behavior

```rust
let mut parser = Parser::new(scanner);
// Optional: preserve tag directives across documents
parser.keep_tags(true);
```

## Common Challenges

### Tag Resolution Complexity

- Resolving tags based on node content and context
- Supporting application-specific tag schemes
- Handling errors in tag syntax or resolution

### Position Tracking Issues

- Ensuring tag positions are correctly tracked separate from node positions
- Handling complex tags with special characters or URI encodings
- Tracking positions for implicit tags (which don't appear in the source)

### Serialization Considerations

- Preserving tags during round-trip operations
- Handling custom tags during serialization
- Maintaining tag handle consistency across documents

## Implementation Notes

### Tag Resolution Rules

The YAML 1.2 specification defines complex rules for tag resolution:

1. If a node has an explicit tag, use it
2. If a node has no explicit tag, use tag resolution rules:
   - `!!str` for most scalar content
   - `!!int` for integer-like content
   - `!!float` for floating-point-like content
   - `!!null` for null values
   - `!!bool` for boolean values
   - `!!seq` for sequences
   - `!!map` for mappings

### Tag Compatibility

yaml-rust2 follows the YAML 1.2 specification for tags, which differs slightly from YAML 1.1:
- YAML 1.2 has simplified tag resolution rules
- Some implicit tag resolutions changed from 1.1 to 1.2
- The core tag library (!!seq, !!map, etc.) remains consistent

## Examples

### Example 1: Explicit Tags

```yaml
# Explicit tag examples
---
!!str "A string"
!!int 42
!!float 3.14159
!!seq [1, 2, 3]
!!map {key: value}
!<tag:example.com,2011:custom> Custom data
```

### Example 2: Tag Directives

```yaml
# Tag directive example
%TAG !prefix! tag:example.com,2011:
---
# Uses the defined prefix
!prefix!type Some value
```

### Example 3: Tag Positions

When tracking positions for this YAML:

```yaml
---
!!str "Tagged string"
```

The position tracking would include:
- Tag start position at the first `!` character (column 1)
- Tag end position after the last character of `str` (column 5)
- Node position covering the string content

## Related Documentation

- [YAML Directives](directives.md) - Information about directives, including TAG directives
- [Position Tracked Loader](/docs/position_tracked_loader.md) - Details on position tracking implementation
- [Source Mapping](/docs/source_mapping.md) - How nodes map to source positions