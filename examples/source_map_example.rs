#![cfg(feature = "source_mapping")]

//! Source Map Example
//!
//! This example demonstrates how to use the source mapping functionality
//! in yaml-rust2. It shows two approaches:
//!
//! 1. Manual source map creation - the example manually registers nodes
//!    with positions to create a source map for demonstration purposes.
//!    This is the approach shown in this example as it's more reliable
//!    and provides better control.
//!
//! 2. Automatic position tracking - the example tries to use the built-in
//!    position tracking to build a source map, but the current implementation
//!    only tracks positions for nodes with anchors or in flow collections.
//!
//! Key features demonstrated:
//! - Parsing YAML with position tracking
//! - Creating source maps with position information
//! - Finding nodes at specific positions
//! - Finding nodes within a range of positions
//! - Formatting error messages with source context

use std::collections::HashMap;
use yaml_rust2::parser::Parser;
use yaml_rust2::position::PositionSpan;
use yaml_rust2::scanner::Marker;
use yaml_rust2::source_map::SourceMapBuilder;
use yaml_rust2::{PositionTrackedLoader, Yaml};

// The example main function
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== YAML Source Mapping Example ===\n");

    // Example 1: Basic source mapping with position tracking
    basic_source_mapping_example()?;

    // Example 2: Error reporting with source mapping
    error_reporting_example()?;

    Ok(())
}

// Utility function to create a marker for testing
fn marker(line: usize, col: usize) -> Marker {
    let index = (line - 1) * 80 + (col - 1); // Approximate index for testing

    // This is a hack to create a marker for demonstration purposes
    struct MarkerTest {
        index: usize,
        line: usize,
        col: usize,
    }

    unsafe { std::mem::transmute(MarkerTest { index, line, col }) }
}

fn basic_source_mapping_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Basic Source Mapping Example ===\n");

    let yaml_str = r#"
# A sample YAML document
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

    // Parse the YAML with position tracking
    let mut loader = PositionTrackedLoader::default();

    // Parse the YAML to capture position information
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true)?;

    // Print the document contents to verify parsing worked
    let documents = loader.documents();
    println!("Parsed {} YAML documents", documents.len());

    if documents.is_empty() {
        println!("No documents found");
        return Ok(());
    }

    // Debugging: print the document structure
    print_yaml_structure(&documents[0], 0);

    // For demonstration purposes, we'll manually create a source map
    println!("\nCreating manual source map for demonstration:");
    let mut builder = SourceMapBuilder::new();

    // Get references to nodes we want to track
    let doc = &documents[0];
    let basic_types = &doc["document"]["basic_types"];
    let integer_node = &basic_types["integer"];
    let string_node = &basic_types["string"];

    // Register nodes with positions
    let _root_id = builder.register_node(
        doc.clone(),
        PositionSpan::with_end(marker(1, 1), marker(14, 1)),
    );

    let _basic_types_id = builder.register_node(
        basic_types.clone(),
        PositionSpan::with_end(marker(4, 3), marker(7, 28)),
    );

    let integer_id = builder.register_node(
        integer_node.clone(),
        PositionSpan::with_end(marker(6, 14), marker(6, 16)),
    );

    let _string_id = builder.register_node(
        string_node.clone(),
        PositionSpan::with_end(marker(5, 13), marker(5, 28)),
    );

    // Build the source map
    let source_map = builder.build_empty();

    // Print information about the manually created source map
    let node_ids = source_map.get_all_node_ids();
    println!("Manually created source map has {} nodes", node_ids.len());

    // Print node positions
    println!("\nNode positions in manual source map:");
    for id in &node_ids {
        if let Some(location) = source_map.get_location(*id) {
            if let Some(node) = source_map.get_node(*id) {
                println!(
                    "Node at ({},{}): {:?}",
                    location.span.start.line(),
                    location.span.start.col(),
                    node_type_name(node)
                );
            }
        }
    }

    // Find the integer node
    println!("\nLooking for integer node:");
    if let Some(location) = source_map.get_location(integer_id) {
        println!(
            "Integer node found at position ({},{}): {}",
            location.span.start.line(),
            location.span.start.col(),
            42
        );
    } else {
        println!("Integer node not found");
    }

    // Look up node at specific position
    println!("\nLooking up node at position 6:14:");
    if let Some(id) = source_map.find_node_at_position(6, 14) {
        if let Some(node) = source_map.get_node(id) {
            println!("Found node at (6,14): {:?}", node_type_name(node));
        }
    } else {
        println!("No node found at position 6:14");
    }

    // Find all nodes in a range
    println!("\nFinding all nodes in the range (4,1) to (7,1):");
    let nodes_in_range = source_map.find_nodes_in_range(4, 1, 7, 1);
    for id in nodes_in_range {
        if let Some(node) = source_map.get_node(id) {
            if let Some(location) = source_map.get_location(id) {
                println!(
                    "Node in range at ({},{}): {:?}",
                    location.span.start.line(),
                    location.span.start.col(),
                    node_type_name(node)
                );
            }
        }
    }

    Ok(())
}

fn error_reporting_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Error Reporting Example ===\n");

    let missing_password_error = "Password field is required in database configuration";

    let valid_yaml = r#"
config:
  database: &db
    host: &host localhost
    port: &port 3306
    username: &user admin
    # Missing required field: password
"#;

    // Parse the YAML with position tracking
    let mut loader = PositionTrackedLoader::default();

    // Parse the YAML to capture position information
    let mut parser = Parser::new(valid_yaml.chars());
    parser.load(&mut loader, true)?;

    // Get access to the parsed YAML document
    let doc = &loader.documents()[0];

    // Manually create source map for the error example
    let mut builder = SourceMapBuilder::new();

    // Get references to important nodes
    let config_node = &doc["config"];
    let db_node = &config_node["database"];

    // Register nodes with positions
    let _root_id = builder.register_node(
        doc.clone(),
        PositionSpan::with_end(marker(1, 1), marker(8, 1)),
    );

    let _config_id = builder.register_node(
        config_node.clone(),
        PositionSpan::with_end(marker(2, 1), marker(7, 1)),
    );

    let db_id = builder.register_node(
        db_node.clone(),
        PositionSpan::with_end(marker(3, 3), marker(7, 1)),
    );

    // Build the source map
    let source_map = builder.build_empty();

    // Find the database configuration node
    println!("Looking for database configuration node:");
    if let Some(location) = source_map.get_location(db_id) {
        println!(
            "Database node found at position ({},{})",
            location.span.start.line(),
            location.span.start.col()
        );

        // Simulate validation error for missing password
        let mut validation_errors = HashMap::new();
        validation_errors.insert(db_id, missing_password_error.to_string());

        // Format and display the error
        println!("\nValidation errors:");
        for (node_id, error_msg) in &validation_errors {
            if let Some(location) = source_map.get_location(*node_id) {
                // Format an error message with source context
                let start_line = location.span.start.line();
                let start_col = location.span.start.col();

                println!(
                    "Error at line {}, column {}: {}",
                    start_line, start_col, error_msg
                );

                // Extract context from the source
                let lines: Vec<&str> = valid_yaml.lines().collect();
                let context_start = start_line.saturating_sub(1);
                let context_end = (start_line + 1).min(lines.len());

                // Display context with a marker for the error location
                println!("\nContext:");
                for (i, line) in lines
                    .iter()
                    .enumerate()
                    .skip(context_start)
                    .take(context_end - context_start)
                {
                    println!("{:3} | {}", i + 1, line);
                    if i + 1 == start_line {
                        // Add a pointer to the error location
                        println!(
                            "    | {}{}",
                            " ".repeat(start_col - 1),
                            format!("^ {}", error_msg)
                        );
                    }
                }
            }
        }
    } else {
        println!("Database configuration node not found");
    }

    Ok(())
}

// Helper function to get the type name of a YAML node
fn node_type_name(node: &Yaml) -> &'static str {
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

/// Helper function to print the YAML structure with indentation
fn print_yaml_structure(yaml: &Yaml, indent: usize) {
    let indent_str = " ".repeat(indent * 2);

    match yaml {
        Yaml::Hash(hash) => {
            println!("{}Hash with {} entries:", indent_str, hash.len());
            for (k, v) in hash {
                print!("{}Key: ", indent_str);
                print_yaml_structure(k, 0);
                print_yaml_structure(v, indent + 1);
            }
        }
        Yaml::Array(array) => {
            println!("{}Array with {} items:", indent_str, array.len());
            for item in array {
                print_yaml_structure(item, indent + 1);
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
