#![cfg(feature = "source_mapping")]

use yaml_rust2::{
    parser::Parser, position::PositionSpan, source_map::SourceMapSupport, PositionTrackedLoader,
    Yaml,
};

// Helper function to find a node in the source map by type and path
fn find_node_by_path<'a>(
    document: &'a Yaml,
    source_map: &yaml_rust2::source_map::SourceMap<Yaml>,
    path: &[&str],
) -> Option<(yaml_rust2::source_map::NodeId, &'a Yaml)> {
    let mut current = document;

    // Navigate to the node at the specified path
    for &key in path {
        match current {
            Yaml::Hash(hash) => {
                if let Some(value) = hash.get(&Yaml::String(key.to_string())) {
                    current = value;
                } else {
                    return None;
                }
            }
            Yaml::Array(array) => {
                if let Ok(index) = key.parse::<usize>() {
                    if let Some(value) = array.get(index) {
                        current = value;
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    // Find the node ID that corresponds to this node
    for id in source_map.get_all_node_ids() {
        if let Some(node) = source_map.get_node(id) {
            // Check if this is the node we're looking for by comparing references
            if std::ptr::eq(node as *const _, current as *const _) {
                return Some((id, current));
            }
        }
    }

    None
}

fn print_source_map_nodes(source_map: &yaml_rust2::source_map::SourceMap<Yaml>) {
    println!("Source map nodes:");
    for id in source_map.get_all_node_ids() {
        if let Some(node) = source_map.get_node(id) {
            if let Some(location) = source_map.get_location(id) {
                let node_type = match node {
                    Yaml::Hash(_) => "Hash",
                    Yaml::Array(_) => "Array",
                    Yaml::String(_) => "String",
                    Yaml::Integer(_) => "Integer",
                    Yaml::Real(_) => "Real",
                    Yaml::Boolean(_) => "Boolean",
                    Yaml::Null => "Null",
                    Yaml::BadValue => "BadValue",
                    Yaml::Alias(_) => "Alias",
                };
                println!(
                    "Node {:?} at ({},{}) - ({},{}): {}",
                    id,
                    location.span.start.line(),
                    location.span.start.col(),
                    location.span.end.map_or(0, |m| m.line()),
                    location.span.end.map_or(0, |m| m.col()),
                    node_type
                );
            }
        }
    }
}

#[test]
fn test_automatic_position_tracking_with_anchors() {
    let yaml_str = r#"
# This is a test document with anchors
document:
  basic_types: &basic
    string: This is a string
    integer: &int_val 42
    float: &float_val 3.14159
  nested: &nested_items
    - item1
    - item2
    - mapping: &map
        key: value
  ref_integer: *int_val
  ref_float: *float_val
"#;

    // Parse the YAML
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true).unwrap();

    // Build source maps
    let source_maps = loader.build_source_maps();
    assert!(!source_maps.is_empty(), "Source maps should not be empty");

    let source_map = &source_maps[0];
    let document = &loader.documents()[0];

    // Debug output of all nodes
    print_source_map_nodes(source_map);

    // Verify anchored nodes have position information
    let (basic_id, basic_node) =
        find_node_by_path(document, source_map, &["document", "basic_types"])
            .expect("Should find basic_types node");

    let (int_id, int_node) = find_node_by_path(
        document,
        source_map,
        &["document", "basic_types", "integer"],
    )
    .expect("Should find integer node");

    let (nested_id, _) = find_node_by_path(document, source_map, &["document", "nested"])
        .expect("Should find nested node");

    // Test that we can find these nodes at their positions
    let basic_loc = source_map
        .get_location(basic_id)
        .expect("Should have position for basic_types");
    let int_loc = source_map
        .get_location(int_id)
        .expect("Should have position for integer");
    let nested_loc = source_map
        .get_location(nested_id)
        .expect("Should have position for nested");

    // Assert that we can find these nodes by position
    let found_basic =
        source_map.find_node_at_position(basic_loc.span.start.line(), basic_loc.span.start.col());
    let found_int =
        source_map.find_node_at_position(int_loc.span.start.line(), int_loc.span.start.col());
    let found_nested =
        source_map.find_node_at_position(nested_loc.span.start.line(), nested_loc.span.start.col());

    assert!(
        found_basic.is_some(),
        "Should find basic_types node by position"
    );
    assert!(found_int.is_some(), "Should find integer node by position");
    assert!(
        found_nested.is_some(),
        "Should find nested node by position"
    );

    // Verify that the integer node has the right value
    match int_node {
        Yaml::Integer(value) => {
            assert_eq!(*value, 42, "Integer value should be 42");
        }
        _ => panic!("Expected integer node"),
    }
}

#[test]
fn test_automatic_position_tracking_flow_collections() {
    let yaml_str = r#"
# Test with flow collections
flow_mapping: {key1: value1, key2: [item1, item2, {nested_key: nested_value}]}
mixed:
  block_key: {flow_key: flow_value}
  flow_seq: [1, 2, 3]
"#;

    // Parse the YAML
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true).unwrap();

    // Build source maps
    let source_maps = loader.build_source_maps();
    assert!(!source_maps.is_empty(), "Source maps should not be empty");

    let source_map = &source_maps[0];
    let document = &loader.documents()[0];

    // Debug output of all nodes
    print_source_map_nodes(source_map);

    // Verify flow collections have position information
    let (flow_mapping_id, _) = find_node_by_path(document, source_map, &["flow_mapping"])
        .expect("Should find flow_mapping node");

    let flow_mapping_loc = source_map
        .get_location(flow_mapping_id)
        .expect("Should have position for flow_mapping");

    println!(
        "Flow mapping position: ({},{})",
        flow_mapping_loc.span.start.line(),
        flow_mapping_loc.span.start.col()
    );

    // Try to find the node at its position
    let found_flow_mapping = source_map.find_node_at_position(
        flow_mapping_loc.span.start.line(),
        flow_mapping_loc.span.start.col(),
    );

    assert!(
        found_flow_mapping.is_some(),
        "Should find flow_mapping node by position"
    );
}

#[test]
fn test_automatic_position_tracking_complex_document() {
    let yaml_str = r#"
---
# A complex YAML document with anchors, flow collections, and blocks
metadata:
  version: 1.0
  created_at: 2023-10-15
  author: &author
    name: Test User
    email: test@example.com

config: &default_config
  database:
    host: localhost
    port: &db_port 5432
    credentials: {username: admin, password: secret}
  
  logging:
    level: info
    format: &log_fmt json
    targets: [console, file]

environments:
  development:
    <<: *default_config
    database:
      port: 3306
  
  production:
    <<: *default_config
    logging:
      level: warning
      format: *log_fmt

owner: *author
---
# Second document
simple: value
"#;

    // Parse the YAML
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true).unwrap();

    // Build source maps
    let source_maps = loader.build_source_maps();
    assert_eq!(
        source_maps.len(),
        2,
        "Should have 2 source maps for 2 documents"
    );

    let source_map = &source_maps[0]; // First document
    let document = &loader.documents()[0];

    // Debug output of all nodes
    print_source_map_nodes(source_map);

    // Check if anchored nodes have position information
    let (author_id, _) = find_node_by_path(document, source_map, &["metadata", "author"])
        .expect("Should find author node");

    let (config_id, _) =
        find_node_by_path(document, source_map, &["config"]).expect("Should find config node");

    let (db_port_id, _) = find_node_by_path(document, source_map, &["config", "database", "port"])
        .expect("Should find db_port node");

    // Test that we can find these nodes at their positions
    let author_loc = source_map
        .get_location(author_id)
        .expect("Should have position for author");
    let config_loc = source_map
        .get_location(config_id)
        .expect("Should have position for config");
    let db_port_loc = source_map
        .get_location(db_port_id)
        .expect("Should have position for db_port");

    // Assert that we can find these nodes by position
    let found_author =
        source_map.find_node_at_position(author_loc.span.start.line(), author_loc.span.start.col());
    let found_config =
        source_map.find_node_at_position(config_loc.span.start.line(), config_loc.span.start.col());
    let found_db_port = source_map
        .find_node_at_position(db_port_loc.span.start.line(), db_port_loc.span.start.col());

    assert!(
        found_author.is_some(),
        "Should find author node by position"
    );
    assert!(
        found_config.is_some(),
        "Should find config node by position"
    );
    assert!(
        found_db_port.is_some(),
        "Should find db_port node by position"
    );

    // Check second document
    let second_source_map = &source_maps[1];
    let second_document = &loader.documents()[1];

    print_source_map_nodes(second_source_map);
}

#[test]
fn test_automatic_position_tracking_for_block_sequences() {
    let yaml_str = r#"
# Test with block sequences
block_sequence:
  - item1
  - item2:
      nested_key: nested_value
  - item3
"#;

    // Parse the YAML
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true).unwrap();

    // Build source maps
    let source_maps = loader.build_source_maps();
    assert!(!source_maps.is_empty(), "Source maps should not be empty");

    let source_map = &source_maps[0];
    let document = &loader.documents()[0];

    // Debug output of all nodes
    print_source_map_nodes(source_map);

    // Verify sequence has position information
    let (sequence_id, _) = find_node_by_path(document, source_map, &["block_sequence"])
        .expect("Should find block_sequence node");

    let sequence_loc = source_map
        .get_location(sequence_id)
        .expect("Should have position for block_sequence");

    println!(
        "Block sequence position: ({},{})",
        sequence_loc.span.start.line(),
        sequence_loc.span.start.col()
    );

    // Try to find items in the sequence
    if let Yaml::Array(items) = &document["block_sequence"] {
        for (i, item) in items.iter().enumerate() {
            for id in source_map.get_all_node_ids() {
                if let Some(node) = source_map.get_node(id) {
                    if std::ptr::eq(node as *const _, item as *const _) {
                        if let Some(loc) = source_map.get_location(id) {
                            println!(
                                "Item {} found at position ({},{})",
                                i,
                                loc.span.start.line(),
                                loc.span.start.col()
                            );
                        }
                    }
                }
            }
        }
    }
}
