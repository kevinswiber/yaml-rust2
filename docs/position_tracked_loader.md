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