#![cfg(feature = "source_mapping")]

use std::collections::HashMap;
use yaml_rust2::{
    parser::Parser,
    position::PositionSpan,
    source_map::{SourceMap, SourceMapBuilder, SourceMapSupport},
    PositionTrackedLoader, Yaml,
};

/// This test tracks positions for nodes that don't have anchors
/// and shows where source mapping currently has limitations
#[test]
fn test_track_positions_for_non_anchored_nodes() {
    let yaml_str = r#"
# Simple YAML document without anchors
document:
  string_value: Plain string
  integer_value: 42
  nested:
    - item1
    - item2
    - key: value
"#;

    // Parse the YAML
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true).unwrap();

    // Build source maps
    let source_maps = loader.build_source_maps();
    let source_map = &source_maps[0];
    let document = &loader.documents()[0];

    // Print all nodes in the source map
    println!("Source map nodes (automatic tracking):");
    for id in source_map.get_all_node_ids() {
        if let Some(node) = source_map.get_node(id) {
            if let Some(location) = source_map.get_location(id) {
                println!(
                    "Node at ({},{}): {:?}",
                    location.span.start.line(),
                    location.span.start.col(),
                    node_type(node)
                );
            }
        }
    }

    // The current implementation may not track non-anchored nodes
    // So let's manually create a source map with all nodes to compare
    println!("\nManually created source map (all nodes):");
    let mut builder = SourceMapBuilder::new();
    let mut position_map = HashMap::new();

    // Register all the nodes with approximate positions
    register_yaml_nodes(&mut builder, document, &mut position_map, 1, 1);

    let manual_source_map = builder.build_empty();

    // Print all nodes in the manual source map
    for id in manual_source_map.get_all_node_ids() {
        if let Some(node) = manual_source_map.get_node(id) {
            if let Some(location) = manual_source_map.get_location(id) {
                println!(
                    "Node at ({},{}): {:?}",
                    location.span.start.line(),
                    location.span.start.col(),
                    node_type(node)
                );
            }
        }
    }

    // Find nodes by position in both source maps
    println!("\nAutomatically tracked source map:");
    test_position_lookup(source_map, 4, 3); // string_value
    test_position_lookup(source_map, 5, 3); // integer_value
    test_position_lookup(source_map, 6, 3); // nested

    println!("\nManually created source map:");
    test_position_lookup(&manual_source_map, 4, 3); // string_value
    test_position_lookup(&manual_source_map, 5, 3); // integer_value
    test_position_lookup(&manual_source_map, 6, 3); // nested
}

/// Register all nodes in a YAML structure with estimated positions
fn register_yaml_nodes(
    builder: &mut SourceMapBuilder,
    node: &Yaml,
    position_map: &mut HashMap<*const Yaml, (usize, usize)>,
    line: usize,
    col: usize,
) -> (usize, usize) {
    let start_pos = (line, col);

    // Store this position for the node
    position_map.insert(node as *const Yaml, start_pos);

    // Calculate the end position based on node type
    let end_pos = match node {
        Yaml::Hash(hash) => {
            let mut current_line = line;
            let mut current_col = col;

            for (key, value) in hash {
                // Key is at current position
                register_yaml_nodes(builder, key, position_map, current_line, current_col);

                // Value is at a new position (estimate)
                current_line += 1;
                let (new_line, _) = register_yaml_nodes(
                    builder,
                    value,
                    position_map,
                    current_line,
                    current_col + 2,
                );

                // Update current position for the next key-value pair
                current_line = new_line + 1;
                current_col = col; // Reset column to start of line
            }

            (current_line, current_col)
        }
        Yaml::Array(array) => {
            let mut current_line = line;
            let mut current_col = col;

            for item in array {
                // Each item is on a new line with increased indentation
                current_line += 1;
                let (new_line, _) =
                    register_yaml_nodes(builder, item, position_map, current_line, current_col + 2);
                current_line = new_line;
            }

            (current_line, current_col)
        }
        Yaml::String(s) => (line, col + s.len() + 1),
        Yaml::Integer(_) => (line, col + 3),
        Yaml::Real(_) => (line, col + 5),
        Yaml::Boolean(_) => (line, col + 5),
        Yaml::Alias(_) => (line, col + 10),
        _ => (line, col + 1),
    };

    // Create a PositionSpan with start and end positions
    let span = PositionSpan::with_end(
        create_marker(start_pos.0, start_pos.1),
        create_marker(end_pos.0, end_pos.1),
    );

    // Register the node with its position
    builder.register_node(node.clone(), span);

    end_pos
}

/// Create a marker for a specific line:column
fn create_marker(line: usize, col: usize) -> yaml_rust2::scanner::Marker {
    // This is a hack to create a marker for testing
    struct MarkerTest {
        index: usize,
        line: usize,
        col: usize,
    }

    let index = (line - 1) * 80 + (col - 1);
    unsafe { std::mem::transmute(MarkerTest { index, line, col }) }
}

/// Test finding a node at a specific position
fn test_position_lookup(source_map: &SourceMap<Yaml>, line: usize, col: usize) {
    println!("Looking for node at position ({},{})", line, col);
    if let Some(id) = source_map.find_node_at_position(line, col) {
        if let Some(node) = source_map.get_node(id) {
            if let Some(location) = source_map.get_location(id) {
                println!(
                    "Found node: {:?} at ({},{})",
                    node_type(node),
                    location.span.start.line(),
                    location.span.start.col()
                );
            }
        }
    } else {
        println!("No node found at position ({},{})", line, col);
    }
}

/// Get the type of a YAML node as a string
fn node_type(node: &Yaml) -> &'static str {
    match node {
        Yaml::Real(_) => "Real",
        Yaml::Integer(_) => "Integer",
        Yaml::String(_) => "String",
        Yaml::Boolean(_) => "Boolean",
        Yaml::Array(_) => "Array",
        Yaml::Hash(_) => "Hash",
        Yaml::Alias(_) => "Alias",
        Yaml::Null => "Null",
        Yaml::BadValue => "BadValue",
    }
}

#[test]
fn test_enhanced_position_tracking() {
    // This test demonstrates how position tracking could be improved
    // by directly tracking positions during parsing

    let yaml_str = r#"
# Simple document with various node types
top_level:
  string: simple value
  number: 42
  sequence:
    - item1
    - item2
    - nested:
        key: value
"#;

    // Parse the YAML with position tracking
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true).unwrap();

    // Print out the parsed document structure
    println!("Document structure:");
    print_yaml(&loader.documents()[0], 0);

    // Build the source map
    let source_maps = loader.build_source_maps();
    let source_map = &source_maps[0];

    // Print all nodes that have positions in the source map
    println!("\nNodes with position information:");
    print_nodes_with_positions(source_map);

    // Here we would add enhanced position tracking by modifying:
    // 1. The PositionTracker to track positions for all nodes, not just anchors
    // 2. The PositionTrackedLoader to capture start/end positions for all constructs
    // 3. The Parser to provide more precise position information to the loader

    println!("\nEnhancement ideas for position tracking:");
    println!("1. Track positions for all nodes during parsing, not just anchors");
    println!("2. Capture both start and end positions for all constructs");
    println!("3. Improve the PositionTracker to handle non-anchored nodes");
    println!("4. Modify collect_position_spans to include all nodes, not just anchored ones");
    println!("5. Directly associate positions with nodes during parsing");
}

/// Print all nodes that have positions in a source map
fn print_nodes_with_positions(source_map: &SourceMap<Yaml>) {
    for id in source_map.get_all_node_ids() {
        if let Some(node) = source_map.get_node(id) {
            if let Some(location) = source_map.get_location(id) {
                println!(
                    "* Node at ({},{}) - Type: {}",
                    location.span.start.line(),
                    location.span.start.col(),
                    node_type(node)
                );

                // Print the node value for reference
                match node {
                    Yaml::String(s) => println!("  Value: \"{}\"", s),
                    Yaml::Integer(i) => println!("  Value: {}", i),
                    Yaml::Real(r) => println!("  Value: {}", r),
                    Yaml::Boolean(b) => println!("  Value: {}", b),
                    Yaml::Hash(h) => println!("  Hash with {} entries", h.len()),
                    Yaml::Array(a) => println!("  Array with {} items", a.len()),
                    _ => {}
                }
            }
        }
    }
}

/// Print a YAML node with indentation
fn print_yaml(node: &Yaml, indent: usize) {
    let indent_str = " ".repeat(indent * 2);

    match node {
        Yaml::Hash(hash) => {
            println!("{}Hash with {} entries:", indent_str, hash.len());
            for (k, v) in hash {
                print!("{}Key: ", indent_str);
                print_yaml(k, 0);
                print_yaml(v, indent + 1);
            }
        }
        Yaml::Array(array) => {
            println!("{}Array with {} items:", indent_str, array.len());
            for item in array {
                print_yaml(item, indent + 1);
            }
        }
        Yaml::String(s) => println!("{}String: \"{}\"", indent_str, s),
        Yaml::Integer(i) => println!("{}Integer: {}", indent_str, i),
        Yaml::Real(r) => println!("{}Real: {}", indent_str, r),
        Yaml::Boolean(b) => println!("{}Boolean: {}", indent_str, b),
        Yaml::Null => println!("{}Null", indent_str),
        Yaml::BadValue => println!("{}BadValue", indent_str),
        Yaml::Alias(anchor_id) => println!("{}Alias to anchor {}", indent_str, anchor_id),
    }
}

// Add a test that verifies end span tracking
#[test]
#[cfg(feature = "source_mapping")]
fn test_end_span_tracking() {
    use yaml_rust2::parser::Parser;
    use yaml_rust2::source_map::SourceMapSupport;
    use yaml_rust2::PositionTrackedLoader;

    let yaml_str = r#"
# Test YAML document
key1: value1
key2:
  nested:
    inner: value2
key3:
  - item1
  - item2
  - submap:
      foo: bar
"#;

    // Parse with position tracking
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true).unwrap();

    // Build source maps
    let source_maps = loader.build_source_maps();
    assert!(!source_maps.is_empty(), "Source maps should be generated");

    let source_map = &source_maps[0];
    let doc = &loader.documents()[0];

    // Verify end spans exist for various types of nodes
    let node_positions = collect_node_positions(source_map);
    println!("Source map contains {} nodes", node_positions.len());

    // Print all node positions for debugging
    for (node_desc, start, end) in &node_positions {
        println!(
            "Node {:?} at ({},{}) - ({},{})",
            node_desc, start.0, start.1, end.0, end.1
        );
    }

    // Find key1 scalar value node
    let key1_value = find_node_with_desc(&node_positions, "String(value1)");
    assert!(key1_value.is_some(), "value1 node should be found");
    if let Some((_, start, end)) = key1_value {
        assert!(end.0 > 0 && end.1 > 0, "End position should be populated");
        assert!(
            end.0 >= start.0,
            "End line should be >= start line, got {}-{}",
            start.0,
            end.0
        );
    }

    // Find the nested mapping
    let nested_map = find_node_with_desc(&node_positions, "Hash");
    assert!(nested_map.is_some(), "Nested hash should be found");
    if let Some((_, start, end)) = nested_map {
        assert!(end.0 > 0 && end.1 > 0, "End position should be populated");
        assert!(
            end.0 >= start.0,
            "End line should be >= start line, got {}-{}",
            start.0,
            end.0
        );
    }

    // Find array/sequence
    let array_node = find_node_with_desc(&node_positions, "Array");
    assert!(array_node.is_some(), "Array node should be found");
    if let Some((_, start, end)) = array_node {
        assert!(end.0 > 0 && end.1 > 0, "End position should be populated");
        assert!(
            end.0 >= start.0,
            "End line should be >= start line, got {}-{}",
            start.0,
            end.0
        );
        // Array should span multiple lines
        assert!(end.0 > start.0, "Array should span multiple lines");
    }
}

// Helper to collect all node positions from source map
#[cfg(feature = "source_mapping")]
fn collect_node_positions(
    source_map: &yaml_rust2::source_map::SourceMap<yaml_rust2::Yaml>,
) -> Vec<(String, (usize, usize), (usize, usize))> {
    use yaml_rust2::Yaml;

    let mut positions = Vec::new();

    for id in source_map.get_all_node_ids() {
        if let Some(node) = source_map.get_node(id) {
            if let Some(location) = source_map.get_location(id) {
                let start = (location.span.start.line(), location.span.start.col());

                // Use (0,0) if end is None
                let end = location.span.end.map_or((0, 0), |m| (m.line(), m.col()));

                // Get a description of the node
                let desc = match node {
                    Yaml::Real(r) => format!("Real({})", r),
                    Yaml::Integer(i) => format!("Integer({})", i),
                    Yaml::String(s) => format!("String({})", s),
                    Yaml::Boolean(b) => format!("Boolean({})", b),
                    Yaml::Array(a) => format!("Array({})", a.len()),
                    Yaml::Hash(h) => format!("Hash({})", h.len()),
                    Yaml::Alias(id) => format!("Alias({})", id),
                    Yaml::Null => "Null".to_string(),
                    Yaml::BadValue => "BadValue".to_string(),
                };

                positions.push((desc, start, end));
            }
        }
    }

    positions
}

// Helper to find a node with a specific description
#[cfg(feature = "source_mapping")]
fn find_node_with_desc<'a>(
    positions: &'a [(String, (usize, usize), (usize, usize))],
    desc_pattern: &str,
) -> Option<&'a (String, (usize, usize), (usize, usize))> {
    positions
        .iter()
        .find(|(desc, _, _)| desc.contains(desc_pattern))
}
