# YAML Comments

YAML comments are non-content annotations that provide human-readable information without affecting the data model. This document outlines the specification, syntax, and future implementation plans for YAML comments in yaml-rust2, including position tracking aspects.

## YAML 1.2.2 Specification

According to the YAML 1.2.2 specification:

- Comments begin with the `#` character and continue to the end of the line
- Comments may appear after a YAML node or on their own line
- Comments are not part of the YAML information model (not included in the data)
- Comments cannot appear inside scalars (except for block scalars, where # is treated as content)

### Comment Placement

YAML comments can appear in several locations:

1. **Line comments**: After content on the same line
   ```yaml
   key: value # Comment about this value
   ```

2. **Standalone comments**: On their own line
   ```yaml
   # Comment about the following section
   section:
     # Comment about a key
     key: value
   ```

3. **End of line comments**: At the end of lines with special YAML constructs
   ```yaml
   --- # Document start comment
   ... # Document end comment
   [1, 2, # Comment in flow collection
    3, 4]
   ```

4. **Before directive comments**: Even before directives
   ```yaml
   # Header comment
   %YAML 1.2
   ---
   content
   ```

### Comment Behavior

- Comments are ignored by YAML processors in terms of the data model
- Comments should be preserved in editor environments for documentation purposes
- Line comments must be separated from the preceding token by white space
- Comments cannot be placed within quoted scalars or inside other scalar content

## Current Implementation in yaml-rust2

Currently, yaml-rust2 **does not preserve comments** during parsing. While the scanner recognizes comments, they are discarded rather than included in the parsed representation.

### Scanner Level

The scanner detects comment tokens but treats them as whitespace:

- Comments are recognized during lexical scanning
- Comments are discarded rather than converted to events or nodes
- There is no data structure to represent comments in the data model

### Parser and Loader Levels

The parser and loader do not currently process or represent comments:

- No events are generated for comments
- Comments are not associated with nearby nodes
- Comments are effectively removed from the data during parsing

## Planned Future Implementation

According to the project roadmap, comment support is planned for future implementation. Here's an outline of the expected approach:

### Comment Representation

Future implementation will likely include:

- A data structure to represent comments with position information
- Association of comments with YAML nodes based on proximity
- Preservation of comments during parsing and serialization

### Proposed Comment Types

Comments will likely be categorized as:

1. **Header Comments**: Comments before a node
2. **Inline Comments**: Comments on the same line after a node
3. **Footer Comments**: Comments after a node but before the next node
4. **Document Comments**: Comments associated with document markers
5. **Standalone Comments**: Comments not clearly associated with any node

### Position Tracking for Comments

When implemented, position tracking for comments will include:

- **Start Position**: At the `#` character
- **End Position**: At the end of the line (before the line break)
- Association with nearby nodes for context

## Future API Usage (Conceptual)

Once implemented, comment access might look like:

```rust
// Conceptual example - not currently implemented
let yaml_str = "key: value # Comment";
let mut loader = PositionTrackedLoader::new(yaml_str);
let (docs, source_map) = loader.load_all().unwrap();

// Access comments associated with a node
if let Some(node) = docs.first() {
    if let Some(comments) = node.comments() {
        for comment in comments {
            println!("Comment text: {}", comment.text);
            println!("Comment type: {:?}", comment.type); // E.g., Inline, Header, etc.
        }
    }
    
    // Get position of a comment
    if let Some(comment) = node.inline_comment() {
        if let Some(pos) = source_map.get_span(comment) {
            println!("Comment position: start={:?}, end={:?}", pos.start, pos.end);
        }
    }
}
```

## Implementation Challenges

The future implementation will need to address several challenges:

### Comment Association

- Determining which node a comment belongs to
- Handling standalone comments not clearly associated with any node
- Managing comments between nodes

### Position Tracking Complexities

- Tracking positions for comments in various locations
- Handling comments with special characters
- Preserving comment whitespace and formatting

### Round-Trip Preservation

- Preserving comments during load/dump cycles
- Maintaining comment positioning during document modifications
- Handling comments in flow vs. block contexts

## Roadmap Items

Based on the project's roadmap, the comment implementation will likely include:

1. **Scanner Enhancements**:
   - Extend the scanner to preserve comments during parsing
   - Track comment positions along with their text

2. **Data Structure Development**:
   - Create data structures to represent comments with positions
   - Define relationships between comments and nodes

3. **API Development**:
   - Add APIs to access comments associated with YAML nodes
   - Provide methods to add, modify, or remove comments

4. **Serialization Support**:
   - Preserve comments during document serialization
   - Control comment formatting and positioning

5. **Position Tracking Integration**:
   - Incorporate comments into the source mapping system
   - Track comment positions for error reporting and navigation

## Examples

### Example 1: Line Comments

```yaml
key1: value1    # Comment about value1
key2: value2    # Comment about value2
```

Future position tracking would include:
- Comment start positions at the `#` characters
- Comment end positions at the end of each line
- Association with the respective key-value pairs

### Example 2: Standalone Comments

```yaml
# Header comment
mapping:
  # Comment before key1
  key1: value1
  
  # Comment before key2
  key2: value2
# Trailing comment
```

Future position tracking would associate:
- "Header comment" with the "mapping" node
- "Comment before key1" with the "key1" node
- "Comment before key2" with the "key2" node
- "Trailing comment" might be associated with the last node or document

### Example 3: Flow Collection Comments

```yaml
[
  value1, # Comment about value1
  value2, # Comment about value2
  # Comment about value3
  value3
]
```

The comments would be tracked with their positions and associated with the appropriate sequence elements.

## Related Documentation

- [YAML Documents](document.md) - Structure of YAML documents, which can include comments
- [YAML Streams](stream.md) - Multiple documents with potential comments between them
- [Position Tracked Loader](/docs/position_tracked_loader.md) - Future integration with position tracking