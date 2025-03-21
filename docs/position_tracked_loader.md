# PositionTrackedLoader User Guide

The `PositionTrackedLoader` is an enhanced YAML loader that extends the basic functionality of `YamlLoader` with position tracking capabilities. It is particularly useful for applications that need to track the exact positions of anchors and their references in YAML documents.

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

For more advanced use cases, you can access the position tracker directly to get information about anchors and their positions:

```rust
use yaml_rust2::yaml::PositionTrackedLoader;
use yaml_rust2::parser::Parser;

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
    
    // Find the anchor ID for a node
    if let Some(anchor_id) = position_tracker.find_anchor_id(seq_node) {
        // Get the position of the anchor
        if let Some(position) = position_tracker.get_anchor_position(anchor_id) {
            println!("Sequence anchor is at line {}, column {}", position.line, position.col);
        }
        
        // Get the node associated with the anchor
        if let Some(anchor_node) = position_tracker.get_anchor_node(anchor_id) {
            println!("Anchor node content: {:?}", anchor_node);
        }
    }
    
    Ok(())
}
```

## Key Features

1. **Enhanced Anchor Tracking:** Tracks both the position and content of anchors in YAML documents
2. **Complete Alias Resolution:** Properly resolves aliases with references to the original nodes
3. **Position Information:** Provides access to line and column numbers for anchors
4. **Self-Reference Handling:** Safely handles self-referential structures without infinite recursion
5. **Feature-Gated:** Can be enabled with a simple feature flag for applications that need it

## Performance Considerations

The `PositionTrackedLoader` adds a small overhead for tracking positions and nodes compared to the basic `YamlLoader`. For most applications, this overhead is negligible, but if you're parsing extremely large YAML documents and don't need position information, you might want to stick with the basic loader.

## Comparison with YamlLoader

Feature | YamlLoader | PositionTrackedLoader
--------|------------|----------------------
Basic YAML parsing | ✅ | ✅
Anchor/alias support | ✅ | ✅
Position tracking | ❌ | ✅
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
        println!("Error at line {}, column {}: {}", e.mark.line, e.mark.col, e.info);
    }
}
```

## Further Reading

For more details on the implementation, see the following resources:

1. [API Documentation](../src/yaml.rs) - Full documentation of the `PositionTrackedLoader` API
2. [Position Tracker](../src/position.rs) - Details of the underlying position tracking system
3. [Tests](../tests/position_tracked_loader_test.rs) - Examples of how to use the loader in different scenarios 