# PositionTrackedLoader User Guide

The `PositionTrackedLoader` is an enhanced YAML loader that extends the basic functionality of `YamlLoader` with position tracking capabilities. It is particularly useful for applications that need to track the exact positions of YAML nodes, including anchors and their references, in YAML documents.

## Enabling the Feature

The position tracked loader is available through the `position_tracked_loader` feature flag. To enable it, add the following to your `Cargo.toml`:

```toml
[dependencies]
yaml-rust2 = { version = "0.10.0", features = ["position_tracked_loader"] }
```

## Basic Usage

When the feature is enabled, you can use the loader in two ways:

### Direct Usage

```rust
use yaml_rust2::yaml::PositionTrackedLoader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_str = r#"
    anchors:
      seq: &seq_anchor [1, 2, 3]
      map: &map_anchor {a: 1, b: 2}
    references:
      seq_ref: *seq_anchor
      map_ref: *map_anchor
    "#;
    
    // Parse using PositionTrackedLoader
    let documents = PositionTrackedLoader::load_from_str(yaml_str)?;
    
    // Access the parsed documents
    println!("Number of documents: {}", documents.len());
    println!("First document: {:?}", documents[0]);
    
    Ok(())
}
```

### Using Feature-Based Loader Type

```rust
use yaml_rust2::yaml::loader::{DefaultLoader, load_from_str};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_str = r#"
    anchors:
      seq: &seq_anchor [1, 2, 3]
      map: &map_anchor {a: 1, b: 2}
    references:
      seq_ref: *seq_anchor
      map_ref: *map_anchor
    "#;
    
    // Parse using the default loader (which will be PositionTrackedLoader when the feature is enabled)
    let documents = load_from_str(yaml_str)?;
    
    // Access the parsed documents
    println!("Number of documents: {}", documents.len());
    println!("First document: {:?}", documents[0]);
    
    Ok(())
}
```

## Advanced Usage: Accessing Position Information

For more advanced use cases, you can access the position tracker directly to get information about nodes and their positions:

```rust
use yaml_rust2::yaml::PositionTrackedLoader;
use yaml_rust2::parser::Parser;
use yaml_rust2::position::PositionSpan;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_str = r#"
    anchors:
      seq: &seq_anchor [1, 2, 3]
      map: &map_anchor {a: 1, b: 2}
    references:
      seq_ref: *seq_anchor
      map_ref: *map_anchor
    "#;
    
    // Create and use a loader directly to access the position tracker
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true)?;
    
    // Access the position tracker
    let position_tracker = loader.position_tracker();
    
    // Getting position information for a node
    let documents = loader.documents();
    let seq_node = &documents[0]["anchors"]["seq"];
    
    // Find the position span for a node
    if let Some(position_span) = position_tracker.get_node_position(seq_node) {
        // Access start position
        let start = &position_span.start;
        println!("Sequence starts at line {}, column {}", start.line(), start.col());
        
        // Access end position (if available)
        if let Some(end) = position_span.end {
            println!("Sequence ends at line {}, column {}", end.line(), end.col());
        }
    }
    
    // Find the anchor ID for a node
    if let Some(anchor_id) = position_tracker.find_anchor_id(seq_node) {
        // Get the position of the anchor
        if let Some(position) = position_tracker.get_anchor_position(anchor_id) {
            println!("Sequence anchor is at line {}, column {}", position.line(), position.col());
        }
        
        // Get the node associated with the anchor
        if let Some(anchor_node) = position_tracker.get_anchor_node(anchor_id) {
            println!("Anchor node content: {:?}", anchor_node);
        }
    }
    
    Ok(())
}
```

## Using Source Maps for Enhanced Position Tracking

When using `PositionTrackedLoader` with the `source_mapping` feature enabled, you get access to enhanced source mapping capabilities:

```rust
use yaml_rust2::yaml::PositionTrackedLoader;
use yaml_rust2::source_map::SourceMapSupport;
use yaml_rust2::source_map_utils::YamlWithSourceMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_str = r#"
    config:
      server:
        host: example.com
        port: 8080
      database:
        url: postgres://localhost/db
    "#;
    
    // Option 1: Use low-level API with PositionTrackedLoader
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true)?;
    
    // Build source maps for all documents
    let source_maps = loader.build_source_maps();
    let source_map = &source_maps[0];
    
    // Access node positions and content
    let docs = loader.documents();
    
    // Option 2: Use high-level API with YamlWithSourceMap
    let yaml_with_map = YamlWithSourceMap::parse(yaml_str)?;
    
    // Find node by path
    if let Some(server_node) = yaml_with_map.find_node_by_path(&["config", "server"]) {
        println!("Found server node: {:?}", server_node);
        
        // Get position with both start and end
        if let Some(location) = yaml_with_map.get_node_location(server_node) {
            let start = location.span.start;
            let end = location.span.end.unwrap_or(start);
            println!("Server node spans from ({}:{}) to ({}:{})",
                     start.line(), start.col(),
                     end.line(), end.col());
        }
    }
    
    Ok(())
}
```

## Complete Position Information

The position tracking system now provides complete position information for all YAML node types:

1. **Start and End Positions**: Every node has both start and end position information, allowing precise mapping to the source document.

2. **Block Collections**: Block mappings and sequences have accurate start positions at their first content element and end positions at their last content element.

3. **Flow Collections**: Flow mappings and sequences have precise start and end positions that point to their opening and closing delimiters.

4. **Scalar Values**: All scalar values (strings, integers, etc.) have accurate start and end positions, even for multi-line values.

5. **Anchors and Aliases**: Anchor declarations and alias references are precisely positioned in the source document.

## Deep Node Comparison

When working with node lookups, particularly when using path-based access, the `equal_yaml_content` function provides deep comparison between nodes:

```rust
use yaml_rust2::source_map_utils::equal_yaml_content;
use yaml_rust2::source_map_utils::YamlWithSourceMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_str = r#"
    config:
      server:
        settings:
          timeout: 30
          max_connections: 100
    "#;
    
    let yaml_with_map = YamlWithSourceMap::parse(yaml_str)?;
    
    // Two ways to access the same node
    let settings1 = &yaml_with_map.document["config"]["server"]["settings"];
    let settings2 = yaml_with_map.find_node_by_path(&["config", "server", "settings"]).unwrap();
    
    // Deep comparison returns true even though they're different references
    assert!(equal_yaml_content(settings1, settings2));
    
    // This works for all node types including complex nested structures
    let timeout1 = &settings1["timeout"];
    let timeout2 = yaml_with_map.find_node_by_path(&["config", "server", "settings", "timeout"]).unwrap();
    assert!(equal_yaml_content(timeout1, timeout2));
    
    Ok(())
}
```

## Key Features

1. **Universal Position Tracking:** Tracks positions for all nodes in YAML documents, not just anchored ones
2. **Complete Position Spans:** Provides both start and end positions for all node types
3. **Enhanced Anchor Tracking:** Tracks both the position and content of anchors in YAML documents
4. **Complete Alias Resolution:** Properly resolves aliases with references to the original nodes
5. **Source Mapping Integration:** Seamlessly works with the source mapping system for advanced use cases
6. **Path-Based Node Access:** Find nodes using intuitive path-based access with robust deep comparison
7. **Feature-Gated:** Can be enabled with a simple feature flag for applications that need it

## Performance Considerations

The `PositionTrackedLoader` adds a small overhead for tracking positions and nodes compared to the basic `YamlLoader`. For most applications, this overhead is negligible, but if you're parsing extremely large YAML documents and don't need position information, you might want to stick with the basic loader.

## Comparison with YamlLoader

Feature | YamlLoader | PositionTrackedLoader
--------|------------|----------------------
Basic YAML parsing | ✅ | ✅
Anchor/alias support | ✅ | ✅
Position tracking | ❌ | ✅
End position tracking | ❌ | ✅
Non-anchored node tracking | ❌ | ✅
Source mapping integration | ❌ | ✅
Path-based node access | ❌ | ✅
Nested anchor resolution | Limited | ✅
Self-reference handling | Limited | ✅
Access to anchor positions | ❌ | ✅

## Error Handling

The `PositionTrackedLoader` provides the same error handling as the basic `YamlLoader`, returning a `ScanError` with position information when parsing fails:

```rust
match PositionTrackedLoader::load_from_str(yaml_str) {
    Ok(docs) => {
        // Process the documents
    }
    Err(e) => {
        println!("Error at line {}, column {}: {}", e.mark.line(), e.mark.col(), e.info);
    }
}
```

## Further Reading

For more details on the implementation, see the following resources:

1. [Source Mapping Documentation](./source_mapping.md) - Comprehensive guide to using source maps
2. [API Documentation](../src/yaml.rs) - Full documentation of the `PositionTrackedLoader` API
3. [Position Tracker](../src/position.rs) - Details of the underlying position tracking system
4. [Tests](../tests/position_tracked_loader_test.rs) - Examples of how to use the loader in different scenarios

# Position Tracking for YAML Nodes

This document provides a detailed analysis of how the position tracking system works for different YAML node types, tracing the data flow from the Scanner through the Parser, PositionTrackedLoader, and finally to the SourceMap.

## Position Tracking Overview

Position tracking in yaml-rust2 follows these key stages:

1. **Scanner** - Identifies tokens and their positions in the source
2. **Parser** - Processes tokens into events with position markers
3. **PositionTrackedLoader** - Builds YAML nodes with position information
4. **SourceMap** - Provides a queryable interface to find nodes by position

## Position Data Structure

Position information is stored in several related structures:

- **Marker** (`scanner::Marker`) - A single point in the source document:
  ```rust
  pub struct Marker {
      pub(crate) index: usize,  // Byte index in the source
      pub(crate) line: usize,   // 1-based line number
      pub(crate) col: usize,    // 0-based column number
  }
  ```

- **PositionSpan** (`position::PositionSpan`) - A range with start and optional end positions:
  ```rust
  pub struct PositionSpan {
      pub start: Marker,        // Starting position
      pub end: Option<Marker>,  // Optional ending position
  }
  ```

- **SourceLocation** (`source_map::SourceLocation`) - A range with additional byte offset information:
  ```rust
  pub struct SourceLocation {
      pub span: PositionSpan,
      pub byte_range: Option<Range<usize>>,
  }
  ```

## Data Flow from Scanner to SourceMap

### 1. Scanner Stage

The Scanner (`scanner::Scanner`) reads the YAML document character by character and produces tokens (`scanner::Token`), each with a position marker.

- A `Token` contains a `Marker` indicating its position in the source document
- For complex tokens like flow collections, the Scanner tracks opening delimiters in a stack

For flow collections, the Scanner tracks:
```rust
flow_collection_start_marks: Vec<(TokenType, Marker)>,
```

This stack keeps track of the beginning positions of flow sequences (`[`) and flow mappings (`{`).

### 2. Parser Stage

The Parser (`parser::Parser`) transforms tokens into a stream of events (`parser::Event`), each with position information. 

The Parser adds additional position tracking for flow collections:

```rust
position_tracker: crate::position::PositionTracker,
```

For each event type, the Parser tracks positions differently:

#### Scalar Values

For scalar values, the Parser creates a `Event::Scalar` event with the position at the start of the scalar. The ending position is determined based on the scalar's length, which is calculated by:

```rust
// In get_position_span method
let end_mark = Marker::new(
    mark.line(),
    mark.col() + value.len(),
    mark.index() + value.len()
);
```

#### Flow Sequences

For flow sequences, the Parser:
1. Tracks the position of the opening `[` when it sees a `FlowSequenceStart` token
2. When it sees a `FlowSequenceEnd` token, it creates a `SequenceEnd` event with a span from the start to the end position

```rust
// When starting a flow sequence
let start_pos = self.mark();
self.position_tracker.push(anchor_id, start_pos);

// When ending a flow sequence
let start_pos = self.position_tracker.pop().unwrap().1;
let end_pos = self.mark();
let span = PositionSpan::with_end(start_pos, end_pos);
```

#### Flow Mappings

Flow mappings follow a similar pattern to flow sequences, tracking the position of the opening `{` and creating a span when the closing `}` is encountered.

#### Block Collections

Block collections (sequences and mappings) are tracked by indentation level. The Parser uses the indentation to determine when a block collection ends.

For block sequences, the first dash `-` marks the start position, and the last item's end marks the end position.

For block mappings, the first key marks the start position, and the last value's end marks the end position.

### 3. PositionTrackedLoader Stage

The `PositionTrackedLoader` receives events from the Parser through the `MarkedEventReceiver` trait's `on_positioned_event` method. This method receives both the event and its complete position span.

```rust
fn on_positioned_event(&mut self, ev: Event, span: crate::position::PositionSpan) {
    // Process the event with its position information
}
```

As the PositionTrackedLoader builds the YAML structure, it stores position information in its `position_tracker` field:

```rust
position_tracker: std::cell::RefCell<PositionTracker>,
```

The PositionTracker maintains mappings between:
- Nodes and their positions
- Anchor IDs and the nodes they refer to
- Node paths and their positions

For each YAML node type, the position is recorded when the node is created:

#### Scalar Nodes

For scalar nodes (strings, integers, booleans, etc.), both start and end positions are recorded directly using the span from the event:

```rust
// In PositionTrackedLoader::on_event_impl
match ev {
    Event::Scalar(value, style, anchor_id, tag) => {
        // Create the Yaml node
        let node = convert_scalar_value(&value, &style, &tag);
        // Store the position information
        self.position_tracker.borrow_mut().store_anchor_node(anchor_id, node.clone());
        // The event's span contains both start and end positions
    }
}
```

#### Sequence Nodes

For sequences, position tracking depends on the style:

- **Flow Sequences** (`[1, 2, 3]`): The span includes the brackets, from `[` to `]`
- **Block Sequences** (`- item1\n- item2`): The span starts at the first dash and ends after the last item

```rust
// In PositionTrackedLoader::on_event_impl
match ev {
    Event::SequenceStart(anchor_id, _) => {
        // Create a new sequence node
        let node = Yaml::Array(Vec::new());
        // Store both the node and its position span
        self.position_tracker.borrow_mut().store_anchor_node(anchor_id, node.clone());
        // The event's span initially only has a start position
        // End position is added when SequenceEnd is received
    }
    Event::SequenceEnd => {
        // Update the end position in the span for the current sequence
    }
}
```

#### Mapping Nodes

For mappings, similar to sequences, position tracking depends on the style:

- **Flow Mappings** (`{key1: val1, key2: val2}`): The span includes the braces, from `{` to `}`
- **Block Mappings** (`key1: val1\nkey2: val2`): The span starts at the first key and ends after the last value

```rust
// In PositionTrackedLoader::on_event_impl
match ev {
    Event::MappingStart(anchor_id, _, style) => {
        // Create a new mapping node
        let node = Yaml::Hash(LinkedHashMap::new());
        // Store both the node and its position span
        self.position_tracker.borrow_mut().store_anchor_node(anchor_id, node.clone());
        // The event's span initially only has a start position
        // End position is added when MappingEnd is received
    }
    Event::MappingEnd => {
        // Update the end position in the span for the current mapping
    }
}
```

#### Anchors and Aliases

Anchors and their references (aliases) have special handling:

- For an anchor (`&anchor`), the position of the node it's attached to is recorded along with the anchor ID
- For an alias (`*anchor`), the position of the alias reference is recorded, but the node content comes from the original anchor

```rust
// In PositionTrackedLoader::on_event_impl
match ev {
    Event::Alias(anchor_id) => {
        // Get the original node from the anchor
        if let Some(node) = self.position_tracker.borrow().get_anchor_yaml(anchor_id) {
            // Process the resolved node but keep the alias's position
        }
    }
}
```

### 4. SourceMap Stage

The `SourceMap` provides a queryable interface to find nodes by their positions in the source document. It builds on the position information collected by the PositionTrackedLoader.

The SourceMap is created using the `build_source_maps` method:

```rust
// In SourceMapSupport implementation for PositionTrackedLoader
fn build_source_maps(&self) -> Vec<SourceMap<Yaml>> {
    let mut result = Vec::new();
    for i in 0..self.documents().len() {
        if let Some(map) = self.build_source_map_for_document(i) {
            result.push(map);
        }
    }
    result
}
```

For each node in the YAML document, the SourceMap:
1. Assigns a unique NodeId
2. Creates a SourceLocation based on the PositionSpan
3. Stores mappings between NodeId, node content, and location

```rust
// In SourceMapBuilder::build
fn register_nodes(
    builder: &mut SourceMapBuilder,
    node: &Yaml,
    position_spans: &HashMap<*const Yaml, PositionSpan>,
) -> Option<NodeId> {
    let node_ptr = node as *const Yaml;
    let span = position_spans.get(&node_ptr)?;
    
    let node_id = builder.register_node(node.clone(), *span);
    
    // Recursively register child nodes
    match node {
        Yaml::Array(array) => {
            for item in array {
                register_nodes(builder, item, position_spans);
            }
        }
        Yaml::Hash(hash) => {
            for (key, value) in hash {
                register_nodes(builder, key, position_spans);
                register_nodes(builder, value, position_spans);
            }
        }
        _ => {}
    }
    
    Some(node_id)
}
```

## Position Tracking Details by Node Type

### 1. Scalar Values

Scalar values (strings, integers, floats, booleans, etc.) have positions that:
- Start at the first character of the scalar value
- End after the last character of the scalar value

For example, in `key: value`:
- Start: Column position of 'v' in "value"
- End: Column position after 'e' in "value"

Quotes and style markers are included in the span:
- `"quoted"`: Start at the opening quote, end after the closing quote
- `|` block literals: Start at the `|` marker, end after the last line

### 2. Flow Sequences

Flow sequences (`[item1, item2]`) have positions that:
- Start at the opening bracket `[`
- End after the closing bracket `]`

Each item within the sequence has its own position information.

### 3. Block Sequences

Block sequences (starting with `-`) have positions that:
- Start at the first dash `-` or the first line's indentation
- End after the last item in the sequence

For example:
```yaml
- item1
- item2
- item3
```
- Start: Column position of the first `-`
- End: After the last character of "item3"

### 4. Flow Mappings

Flow mappings (`{key1: val1, key2: val2}`) have positions that:
- Start at the opening brace `{`
- End after the closing brace `}`

Keys and values within the mapping have their own position information.

### 5. Block Mappings

Block mappings have positions that:
- Start at the first key's indentation
- End after the last value in the mapping

For example:
```yaml
key1: value1
key2: value2
```
- Start: Column position of "key1"
- End: After the last character of "value2"

### 6. Anchors and Aliases

- **Anchors** (`&anchor`) are attached to nodes and don't have separate positions.
  The anchor marker is considered part of the node's position span.

- **Aliases** (`*anchor`) have positions that:
  - Start at the `*` character
  - End after the last character of the anchor name

### 7. Tags

Tags (`!tag`) have positions that:
- Start at the `!` character
- End after the last character of the tag name

## End Position Calculation

End positions are calculated differently depending on the node type:

1. **Scalar Values**: End position is calculated based on the length of the scalar value.

2. **Flow Collections**: End position is taken directly from the closing delimiter position.

3. **Block Collections**: End position is derived from the last element in the collection.

## Summary of Position Tracking Flow

1. **Scanner** tokenizes the input and attaches `Marker` positions to each token.

2. **Parser** converts tokens to events, enhancing them with `PositionSpan` information that includes both start and end positions.

3. **PositionTrackedLoader** builds the YAML document structure, storing position information in its `PositionTracker`.

4. **SourceMap** creates a queryable mapping between nodes and their positions in the source document.

This comprehensive tracking system ensures that every node in the YAML document can be precisely located in the source, enabling accurate error reporting, source mapping, and interactive tools. 