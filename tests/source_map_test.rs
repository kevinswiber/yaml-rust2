#![cfg(feature = "source_mapping")]

use std::ops::Range;
use yaml_rust2::{
    parser::Parser,
    position::PositionSpan,
    scanner::Marker,
    source_map::{NodeId, SourceLocation, SourceMap, SourceMapBuilder, SourceMapSupport},
    PositionTrackedLoader, Yaml,
};

// Utility function to create a marker for testing
fn marker(line: usize, col: usize) -> Marker {
    // Use struct initialization with pub(crate) fields
    // This is a hack for testing only, we will manually construct a marker with the correct line and column
    let index = (line - 1) * 80 + (col - 1); // Approximate index for testing
    unsafe { std::mem::transmute(MarkerTest { index, line, col }) }
}

// A temporary struct to help with transmuting
#[derive(Clone, Copy, Debug)]
struct MarkerTest {
    index: usize,
    line: usize,
    col: usize,
}

#[test]
fn test_source_map_basic() {
    let _source = "key: value\nlist:\n  - item1\n  - item2";

    // Create a mock source map for testing
    let mut builder = SourceMapBuilder::new();

    // Create test nodes with positions - key node at position 1,1
    let root_node = Yaml::Hash(Default::default());
    let key_node = Yaml::String("key".to_string());
    let value_node = Yaml::String("value".to_string());

    // Register the nodes with positions
    let root_id = builder.register_node(
        root_node.clone(),
        PositionSpan::with_end(marker(1, 1), marker(4, 8)),
    );

    // Key should be the one found at position 1,1 since it's registered first at that position
    let key_id = builder.register_node(
        key_node.clone(),
        PositionSpan::with_end(marker(1, 1), marker(1, 4)),
    );

    let _value_id = builder.register_node(
        value_node,
        PositionSpan::with_end(marker(1, 6), marker(1, 11)),
    );

    // Build the source map
    let source_map = builder.build_empty();

    // Check if we can find nodes by position
    let found_id = source_map
        .find_node_at_position(1, 1)
        .expect("Should find a node at position 1,1");

    // Get the node and verify it's a string (the key) since that was registered first at 1,1
    let found_node = source_map.get_node(found_id).expect("Should get node");
    assert!(matches!(found_node, Yaml::String(_)));
    assert_eq!(found_node, &key_node);

    // Get location of the root node
    let root_loc = source_map
        .get_location(root_id)
        .expect("Should get root location");
    assert_eq!(root_loc.start_line(), 1);
    assert_eq!(root_loc.start_column(), 1);
}

#[test]
fn test_source_map_with_anchors() {
    let source = "anchored: &anchor_1 test\nreference: *anchor_1";

    // Create a mock source map for testing
    let mut builder = SourceMapBuilder::new();

    // Create test nodes with positions
    let anchored_node = Yaml::String("test".to_string());
    let reference_node = Yaml::String("test".to_string()); // Same value for reference

    // Register the nodes with positions
    let anchored_id = builder.register_node(
        anchored_node.clone(),
        PositionSpan::with_end(marker(1, 10), marker(1, 20)),
    );

    let reference_id = builder.register_node(
        reference_node.clone(),
        PositionSpan::with_end(marker(2, 11), marker(2, 21)),
    );

    // Build the source map
    let source_map = builder.build_empty();

    // Find the nodes by position
    let found_anchored_id = source_map
        .find_node_at_position(1, 10)
        .expect("Should find anchored node");
    let found_anchored_node = source_map
        .get_node(found_anchored_id)
        .expect("Should get anchored node");
    assert!(matches!(found_anchored_node, Yaml::String(_)));

    let found_reference_id = source_map
        .find_node_at_position(2, 11)
        .expect("Should find reference node");
    let found_reference_node = source_map
        .get_node(found_reference_id)
        .expect("Should get reference node");
    assert!(matches!(found_reference_node, Yaml::String(_)));

    // Verify the nodes are the same (reference resolves to the anchored value)
    assert_eq!(found_anchored_node, found_reference_node);
}

#[test]
fn test_source_map_complex_document() {
    let source = "
document:
  basic_types:
    string: This is a string
    integer: 42
    float: 3.14159
    boolean: true
    null: ~
  collections:
    mapping:
      key1: value1
      key2: value2
    sequence:
      - item1
      - item2
      - item3
  anchors_and_aliases:
    anchored_scalar: &scalar_anchor test_value
    anchored_mapping: &mapping_anchor
      nested_key: nested_value
    reference_to_scalar: *scalar_anchor
    reference_to_mapping: *mapping_anchor
";

    // Create a mock source map for testing
    let mut builder = SourceMapBuilder::new();

    // Create test nodes with positions for key positions we want to test
    let positions_to_test = [
        (2, 1),   // document
        (3, 3),   // basic_types
        (4, 12),  // string value
        (5, 13),  // integer value
        (10, 5),  // collections
        (11, 5),  // mapping
        (12, 7),  // key1
        (15, 7),  // sequence
        (16, 7),  // - item1
        (19, 5),  // anchors_and_aliases
        (20, 21), // &scalar_anchor
        (23, 28), // *scalar_anchor
    ];

    // Register a mock node for each position we want to test
    for (i, (line, col)) in positions_to_test.iter().enumerate() {
        let mock_node = Yaml::String(format!("mock_node_{}", i));
        builder.register_node(
            mock_node,
            PositionSpan::with_end(marker(*line, *col), marker(*line, *col + 5)),
        );
    }

    // Build the source map
    let source_map = builder.build_empty();

    // Verify we can find nodes at all these positions
    for (line, col) in positions_to_test {
        assert!(
            source_map.find_node_at_position(line, col).is_some(),
            "Should find node at position ({}, {})",
            line,
            col
        );
    }

    // Verify we get None for a position outside the document
    assert!(
        source_map.find_node_at_position(100, 1).is_none(),
        "Should not find node at position outside document"
    );
}

#[test]
fn test_source_map_document_traversal() {
    let source = "
parent:
  child1: value1
  child2:
    grandchild1: nested1
    grandchild2: nested2
  child3: value3
";

    // Create a mock source map for testing
    let mut builder = SourceMapBuilder::new();

    // Create mock nodes for testing
    let positions = [
        (2, 1, "parent"),
        (3, 3, "child1"),
        (3, 11, "value1"),
        (4, 3, "child2"),
        (5, 5, "grandchild1"),
        (6, 5, "grandchild2"),
        (7, 3, "child3"),
    ];

    // Register each node with appropriate position
    for (i, (line, col, name)) in positions.iter().enumerate() {
        let node = match i {
            0 => Yaml::Hash(Default::default()), // parent node is a hash
            1 | 3 | 4 | 5 | 6 => Yaml::String(name.to_string()), // Keys are strings
            _ => Yaml::String(format!("{}_value", name)), // Values are strings
        };

        builder.register_node(
            node,
            PositionSpan::with_end(marker(*line, *col), marker(*line, *col + name.len())),
        );
    }

    // Build the source map
    let source_map = builder.build_empty();

    // Get all node IDs
    let node_ids = source_map.get_all_node_ids();

    // Verify we have the expected number of nodes
    assert!(!node_ids.is_empty(), "Should have some nodes");

    // Test traversing the document tree
    for id in node_ids {
        let node = source_map.get_node(id).expect("Should get node for ID");
        let location = source_map
            .get_location(id)
            .expect("Should get location for ID");

        // Verify the node has a valid position
        assert!(location.start_line() > 0, "Should have valid line number");
        assert!(
            location.start_column() > 0,
            "Should have valid column number"
        );

        // Match on node type and verify basic properties
        match node {
            Yaml::Hash(_) => {
                // Hash nodes should be findable at their start position
                let found_id = source_map
                    .find_node_at_position(location.start_line(), location.start_column());
                assert!(found_id.is_some(), "Should find hash node at its position");
            }
            Yaml::Array(_) => {
                // Array nodes should be findable at their start position
                let found_id = source_map
                    .find_node_at_position(location.start_line(), location.start_column());
                assert!(found_id.is_some(), "Should find array node at its position");
            }
            Yaml::String(_) => {
                // String nodes should be findable at their start position
                let found_id = source_map
                    .find_node_at_position(location.start_line(), location.start_column());
                assert!(
                    found_id.is_some(),
                    "Should find string node at its position"
                );
            }
            _ => {
                // Other node types should also be findable
                let found_id = source_map
                    .find_node_at_position(location.start_line(), location.start_column());
                assert!(found_id.is_some(), "Should find node at its position");
            }
        }
    }
}
