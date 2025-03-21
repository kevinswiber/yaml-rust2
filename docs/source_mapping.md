# Source Mapping for YAML Documents

This guide explains how to use the source mapping functionality in yaml-rust2 to map between YAML nodes and their original positions in the source document.

## Overview

Source mapping provides a bidirectional mapping between YAML nodes and their source locations. This is particularly useful for:

- Error reporting with precise location information
- Creating editor tools that highlight specific parts of YAML documents
- Building diagnostics tools that analyze YAML documents
- Implementing intelligent autocompletion for YAML editors

## Enabling Source Mapping

Source mapping is available as a feature flag. To use it, add the `source_mapping` feature to your dependency:

```toml
[dependencies]
yaml-rust2 = { version = "0.10.0", features = ["source_mapping"] }
```

Note that the `source_mapping` feature automatically enables the `position_tracked_loader` feature, as it builds on top of the position tracking functionality.

## Basic Usage

Here's a basic example of how to use source mapping:

```rust
use yaml_rust2::{PositionTrackedLoader, source_map::SourceMapSupport, Yaml};

// Parse a YAML document with position tracking
let yaml_str = r#"
key1: value1
key2:
  - item1
  - item2
"#;

// Load the document using PositionTrackedLoader
let loader = PositionTrackedLoader::load_from_str(yaml_str).unwrap();

// Build source maps for all documents
let source_maps = loader.build_source_maps();

// If you know there's only one document, you can access it directly
let source_map = &source_maps[0];

// Find a node by position (line, column) - both are 1-based
let node_id = source_map.find_node_at_position(2, 1).unwrap();

// Get the node and its location
let node = source_map.get_node(node_id).unwrap();
let location = source_map.get_location(node_id).unwrap();

println!("Found node at line {}, column {}: {:?}", 
         location.start_line(), location.start_column(), node);
```

## Working with Source Maps

### Creating Source Maps

The `SourceMapBuilder` provides a way to construct a source map:

```rust
use yaml_rust2::{PositionTrackedLoader, source_map::{SourceMapBuilder, SourceMapSupport}};

// Load the document with position tracking
let loader = PositionTrackedLoader::load_from_str(yaml_str).unwrap();

// Build a source map for a specific document
let document_index = 0;
let source_map = loader.build_source_map_for_document(document_index).unwrap();
```

### Finding Nodes by Position

One of the most common use cases is to find a node at a specific position in the document:

```rust
// Find node at a specific line and column (1-based)
let node_id = source_map.find_node_at_position(line, column);

if let Some(id) = node_id {
    let node = source_map.get_node(id).unwrap();
    println!("Found node: {:?}", node);
} else {
    println!("No node found at position ({}, {})", line, column);
}
```

### Finding Nodes by Byte Offset

You can also find nodes by byte offset in the original document:

```rust
// Find node at a specific byte offset
let node_id = source_map.find_node_at_byte_offset(byte_offset);
```

### Getting Node Positions

Once you have a node ID, you can get its source location:

```rust
if let Some(location) = source_map.get_location(node_id) {
    println!("Node starts at line {}, column {}", 
             location.start_line(), location.start_column());
    
    if let Some(end_line) = location.end_line() {
        println!("Node ends at line {}, column {}", 
                 end_line, location.end_column().unwrap());
    }
}
```

## Working with Complex Documents

For complex documents with anchors and aliases, the source map will track both the anchor definitions and references:

```rust
let yaml_str = r#"
anchored: &anchor_1 test
reference: *anchor_1
"#;

let loader = PositionTrackedLoader::load_from_str(yaml_str).unwrap();
let source_maps = loader.build_source_maps();
let source_map = &source_maps[0];

// Find the anchored value
let anchored_id = source_map.find_node_at_position(2, 10).unwrap();
let anchored_node = source_map.get_node(anchored_id).unwrap();

// Find the reference
let reference_id = source_map.find_node_at_position(3, 11).unwrap();
let reference_node = source_map.get_node(reference_id).unwrap();

// Both nodes will refer to the same value
assert_eq!(anchored_node, reference_node);
```

## Advanced Usage

### Custom Node Types

The `SourceMap` is generic over the node type, so you can use it with custom node types:

```rust
use yaml_rust2::source_map::{SourceMap, SourceLocation, NodeId};

// Create a source map for custom node types
let mut custom_source_map = SourceMap::<MyCustomNode>::new();

// Register nodes
let id = custom_source_map.register_node(
    my_node,
    SourceLocation::new(position_span)
);
```

### Traversing All Nodes

You can traverse all nodes in the source map:

```rust
// Get all node IDs
let node_ids = source_map.get_all_node_ids();

// Iterate over all nodes
for id in node_ids {
    let node = source_map.get_node(id).unwrap();
    let location = source_map.get_location(id).unwrap();
    
    // Process node and location
    println!("Node at line {}, column {}: {:?}", 
             location.start_line(), location.start_column(), node);
}
```

## Performance Considerations

Source mapping adds some memory overhead, as it needs to store additional information about each node. If you're working with very large YAML documents and memory usage is a concern, you might want to only enable source mapping when needed.

The lookup operations (`find_node_at_position`, `get_node`, `get_location`) are optimized for performance, but if you're doing many lookups on very large documents, you might want to consider caching the results.

## Limitations

- Source mapping works best with the `PositionTrackedLoader`, which is enabled by the `position_tracked_loader` feature.
- Position information is most accurate for nodes that have anchor IDs or are part of flow collections.
- For some node types, only the start position might be available, not the end position.
- Byte offset information is optional and might not always be available. 