#![cfg(feature = "source_mapping")]

//! Source Map Example
//!
//! This example demonstrates how to use the source mapping functionality
//! in yaml-rust2. It shows two approaches:
//!
//! 1. Automatic position tracking - using the enhanced position tracking system
//!    that tracks positions for all nodes, not just anchored ones.
//!
//! 2. Manual source map creation - manually registers nodes with positions
//!    to create a source map for demonstration or special cases.
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
use yaml_rust2::source_map::{SourceMap, SourceMapBuilder, SourceMapSupport};
use yaml_rust2::source_map_utils::YamlWithSourceMap;
use yaml_rust2::{PositionTrackedLoader, Yaml};

// The example main function
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== YAML Source Mapping Example ===\n");

    // Example 1: Automatic source mapping with position tracking
    automatic_source_mapping_example()?;

    // Example 2: Non-anchored node tracking
    non_anchored_node_tracking_example()?;

    // Example 3: Manual source mapping demonstration
    manual_source_mapping_example()?;

    // Example 4: Error reporting with source mapping
    error_reporting_example()?;

    // Example 5: Using the enhanced source mapping utilities API
    source_map_utils_example()?;

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

fn automatic_source_mapping_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Automatic Source Mapping Example ===\n");

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

    // Build source maps automatically from the loader
    println!("\nCreating automatic source map:");
    let source_maps = loader.build_source_maps();
    if source_maps.is_empty() {
        println!("No source maps generated");
        return Ok(());
    }

    let source_map = &source_maps[0];
    println!("\nAutomatically generated source map information:");
    print_source_map_info(source_map);

    // Find a specific node by navigating the document
    let doc = &documents[0];
    let integer_node = &doc["document"]["basic_types"]["integer"];

    // Find the node ID for the integer node
    let mut integer_id = None;
    for id in source_map.get_all_node_ids() {
        if let Some(node) = source_map.get_node(id) {
            if node == integer_node {
                integer_id = Some(id);
                break;
            }
        }
    }

    // Print information about the integer node
    if let Some(id) = integer_id {
        if let Some(location) = source_map.get_location(id) {
            println!(
                "\nInteger node found at position ({},{}): {:?}",
                location.span.start.line(),
                location.span.start.col(),
                integer_node
            );
        }
    } else {
        println!("\nInteger node not found in source map");
    }

    // Look up node at specific position (using the integer node position)
    if let Some(id) = integer_id {
        if let Some(location) = source_map.get_location(id) {
            let line = location.span.start.line();
            let col = location.span.start.col();
            println!("\nLooking up node at position {}:{}:", line, col);

            if let Some(found_id) = source_map.find_node_at_position(line, col) {
                if let Some(node) = source_map.get_node(found_id) {
                    println!(
                        "Found node at ({},{}): {:?}",
                        line,
                        col,
                        node_type_name(node)
                    );
                }
            } else {
                println!("No node found at position {}:{}", line, col);
            }
        }
    }

    Ok(())
}

fn non_anchored_node_tracking_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Non-Anchored Node Tracking Example ===\n");

    let yaml_str = r#"
# This example demonstrates tracking non-anchored nodes
config:
  server:
    host: localhost
    port: 8080
    debug: true
  paths:
    - /api/v1
    - /api/v2
    - /admin
  options:
    timeout: 30
    retry: 3
    settings:
      cache: true
      logging:
        level: info
"#;

    // Parse the YAML with position tracking
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true)?;

    // Get the document and build the source map
    let documents = loader.documents();
    println!("Parsed {} YAML documents", documents.len());

    if documents.is_empty() {
        println!("No documents found");
        return Ok(());
    }

    let doc = &documents[0];
    let source_maps = loader.build_source_maps();
    let source_map = &source_maps[0];

    // Print the document structure
    println!("\nDocument structure:");
    print_yaml_structure(doc, 0);

    // Print the source map information
    println!("\nSource map information for non-anchored nodes:");
    print_source_map_info(source_map);

    // Find specific non-anchored nodes
    let paths = find_node_at_path(doc, &["config", "paths"]);
    let host = find_node_at_path(doc, &["config", "server", "host"]);
    let logging_level =
        find_node_at_path(doc, &["config", "options", "settings", "logging", "level"]);

    // Find their position information
    println!("\nPosition information for non-anchored nodes:");
    if let Some(paths_node) = paths {
        print_node_position(source_map, paths_node, "paths");
    }

    if let Some(host_node) = host {
        print_node_position(source_map, host_node, "host");
    }

    if let Some(level_node) = logging_level {
        print_node_position(source_map, level_node, "logging level");
    }

    // Find a node at a specific position
    if let Some(host_node) = host {
        let host_id = find_node_id(source_map, host_node);
        if let Some(id) = host_id {
            if let Some(location) = source_map.get_location(id) {
                let line = location.span.start.line();
                let col = location.span.start.col();

                println!("\nLooking up node at position {}:{}:", line, col);
                if let Some(found_id) = source_map.find_node_at_position(line, col) {
                    if let Some(node) = source_map.get_node(found_id) {
                        println!("Found node at ({},{}): {:?}", line, col, node);
                    }
                }
            }
        }
    }

    Ok(())
}

fn manual_source_mapping_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Manual Source Mapping Example ===\n");

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
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true)?;
    let documents = loader.documents();

    if documents.is_empty() {
        println!("No documents found");
        return Ok(());
    }

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
  database:
    host: localhost
    port: 3306
    username: admin
    # Missing required field: password
"#;

    // Parse the YAML with position tracking
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(valid_yaml.chars());
    parser.load(&mut loader, true)?;

    // Build source maps automatically
    let source_maps = loader.build_source_maps();
    if source_maps.is_empty() {
        println!("No source maps generated");
        return Ok(());
    }

    let source_map = &source_maps[0];

    // Get access to the parsed YAML document
    let doc = &loader.documents()[0];
    let db_node = &doc["config"]["database"];

    // Find the node ID for the database node
    let mut db_id = None;
    for id in source_map.get_all_node_ids() {
        if let Some(node) = source_map.get_node(id) {
            if node == db_node {
                db_id = Some(id);
                break;
            }
        }
    }

    // Find the database configuration node
    println!("Looking for database configuration node:");
    if let Some(id) = db_id {
        if let Some(location) = source_map.get_location(id) {
            println!(
                "Database node found at position ({},{})",
                location.span.start.line(),
                location.span.start.col()
            );

            // Simulate validation error for missing password
            let mut validation_errors = HashMap::new();
            validation_errors.insert(id, missing_password_error.to_string());

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
        }
    } else {
        println!("Database configuration node not found");
    }

    Ok(())
}

fn source_map_utils_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Enhanced Source Mapping API ===");

    let yaml_str = r#"
database:
  host: localhost
  port: 5432
  credentials:
    username: admin
    password: secure123
server:
  host: 0.0.0.0
  port: 8080
  debug: true
"#;

    // Parse with automatic source mapping
    let yaml_with_map = YamlWithSourceMap::parse(yaml_str).unwrap();

    // Print document overview
    println!("Document Overview:");
    println!("  Root: {:?}", yaml_with_map.document);
    println!("  Source Map: {} nodes", yaml_with_map.source_map.len());

    // Find and validate database configuration
    if let Some(db_config) = yaml_with_map.find_node_by_path(&["database"]) {
        println!("\nFound database config: {:?}", db_config);

        // Get position for database config
        if let Some(location) = yaml_with_map.get_node_location(db_config) {
            let end_line = location.span.end.map_or(0, |m| m.line());
            let end_col = location.span.end.map_or(0, |m| m.col());

            println!(
                "  Position: ({}:{}) to ({}:{})",
                location.span.start.line(),
                location.span.start.col(),
                end_line,
                end_col
            );

            if location.span.end.is_none() {
                println!("  Note: End position is not set");
            }
        } else {
            println!("  Position: unknown position");
        }

        // Validate database configuration
        let validation_errors = validate_db_config(db_config);
        if !validation_errors.is_empty() {
            println!("\nValidation Errors:");
            for error in validation_errors {
                // Format error with source context
                let error_context = yaml_with_map.format_error(db_config, &error);
                println!("{}", error_context);
            }
        }
    }

    // Find and validate server configuration
    if let Some(server_config) = yaml_with_map.find_node_by_path(&["server"]) {
        println!("\nFound server config: {:?}", server_config);

        // Get position for server config
        if let Some(location) = yaml_with_map.get_node_location(server_config) {
            let end_line = location.span.end.map_or(0, |m| m.line());
            let end_col = location.span.end.map_or(0, |m| m.col());

            println!(
                "  Server position: ({}:{}) to ({}:{})",
                location.span.start.line(),
                location.span.start.col(),
                end_line,
                end_col
            );

            if location.span.end.is_none() {
                println!("  Note: End position is not set");
            }
        } else {
            println!("  Server position: unknown");
        }

        // Check each field in server config
        if let Yaml::Hash(ref hash) = server_config {
            println!("\nServer configuration fields with start and end positions:");
            for (key, value) in hash {
                if let Yaml::String(ref key_str) = key {
                    println!("  Field: {}", key_str);
                    if let Some(location) = yaml_with_map.get_node_location(value) {
                        let end_line = location.span.end.map_or(0, |m| m.line());
                        let end_col = location.span.end.map_or(0, |m| m.col());

                        println!(
                            "    Position: ({}:{}) to ({}:{})",
                            location.span.start.line(),
                            location.span.start.col(),
                            end_line,
                            end_col
                        );

                        if location.span.end.is_none() {
                            println!("    Note: End position is not set");
                        }
                    } else {
                        println!("    Position: unknown");
                    }
                }
            }
        }
    }

    // Demonstrate finding nodes by position
    println!("\nFinding nodes by position:");
    let node = yaml_with_map.find_node_at_position(11, 10); // Approximate position of server port
    if let Some(node) = node {
        println!("  Found node at position (11, 10): {:?}", node);
        if let Some(location) = yaml_with_map.get_node_location(node) {
            let end_line = location.span.end.map_or(0, |m| m.line());
            let end_col = location.span.end.map_or(0, |m| m.col());

            println!(
                "  Full position: ({}:{}) to ({}:{})",
                location.span.start.line(),
                location.span.start.col(),
                end_line,
                end_col
            );

            if location.span.end.is_none() {
                println!("  Note: End position is not set");
            }
        }
    }

    // Demonstrate finding nodes in range
    println!("\nFinding nodes in range:");
    let nodes_in_range = yaml_with_map.find_nodes_in_range(10, 1, 12, 20);
    println!(
        "  Found {} nodes in range (10, 1) - (12, 20)",
        nodes_in_range.len()
    );
    for node in nodes_in_range.iter().take(3) {
        println!("  - {:?}", node);
    }

    Ok(())
}

// Helper function to print source map information
fn print_source_map_info(source_map: &SourceMap<Yaml>) {
    let node_ids = source_map.get_all_node_ids();
    println!("Source map contains {} nodes", node_ids.len());

    // Print some nodes for demonstration
    println!("\nSample nodes from source map:");
    let mut count = 0;
    for id in node_ids {
        if count >= 10 {
            println!(
                "... and {} more nodes",
                source_map.get_all_node_ids().len() - 10
            );
            break;
        }

        if let Some(node) = source_map.get_node(id) {
            if let Some(location) = source_map.get_location(id) {
                println!(
                    "Node at ({},{}) - ({},{}): {:?}",
                    location.span.start.line(),
                    location.span.start.col(),
                    location.span.end.map_or(0, |m| m.line()),
                    location.span.end.map_or(0, |m| m.col()),
                    node_preview(node)
                );
                count += 1;
            }
        }
    }
}

// Helper function to find a node at a specific path
fn find_node_at_path<'a>(doc: &'a Yaml, path: &[&str]) -> Option<&'a Yaml> {
    let mut current = doc;
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
    Some(current)
}

// Helper function to find a node ID
fn find_node_id(
    source_map: &SourceMap<Yaml>,
    node: &Yaml,
) -> Option<yaml_rust2::source_map::NodeId> {
    for id in source_map.get_all_node_ids() {
        if let Some(map_node) = source_map.get_node(id) {
            if equal_yaml_content(map_node, node) {
                return Some(id);
            }
        }
    }
    None
}

// Helper function to check if two YAML nodes have the same content
fn equal_yaml_content(a: &Yaml, b: &Yaml) -> bool {
    match (a, b) {
        (Yaml::String(a), Yaml::String(b)) => a == b,
        (Yaml::Integer(a), Yaml::Integer(b)) => a == b,
        (Yaml::Real(a), Yaml::Real(b)) => a == b,
        (Yaml::Boolean(a), Yaml::Boolean(b)) => a == b,
        (Yaml::Null, Yaml::Null) => true,
        (Yaml::BadValue, Yaml::BadValue) => true,
        (Yaml::Alias(a), Yaml::Alias(b)) => a == b,
        (Yaml::Hash(a), Yaml::Hash(b)) => {
            // For deeper comparison, check size, then compare keys and values
            if a.len() != b.len() {
                return false;
            }

            // Try to compare by content
            for (key, value_a) in a {
                match b.get(key) {
                    Some(value_b) => {
                        if !equal_yaml_content(value_a, value_b) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        }
        (Yaml::Array(a), Yaml::Array(b)) => {
            // Check size first
            if a.len() != b.len() {
                return false;
            }

            // Compare elements one by one
            for (item_a, item_b) in a.iter().zip(b.iter()) {
                if !equal_yaml_content(item_a, item_b) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

// Helper function to print position information for a node
fn print_node_position(source_map: &SourceMap<Yaml>, node: &Yaml, name: &str) {
    if let Some(node_id) = find_node_id(source_map, node) {
        if let Some(location) = source_map.get_location(node_id) {
            println!(
                "{} node at ({},{}) - ({},{}): {:?}",
                name,
                location.span.start.line(),
                location.span.start.col(),
                location.span.end.map_or(0, |m| m.line()),
                location.span.end.map_or(0, |m| m.col()),
                node
            );
        } else {
            println!("{} node found in map but has no location", name);
        }
    } else {
        println!("{} node not found in source map", name);
    }
}

// Helper function to get a preview of a YAML node
fn node_preview(node: &Yaml) -> String {
    match node {
        Yaml::Real(r) => format!("Real({})", r),
        Yaml::Integer(i) => format!("Integer({})", i),
        Yaml::String(s) => {
            let preview = if s.len() > 20 {
                format!("{}...", &s[0..17])
            } else {
                s.clone()
            };
            format!("String({})", preview)
        }
        Yaml::Boolean(b) => format!("Boolean({})", b),
        Yaml::Array(a) => format!("Array({})", a.len()),
        Yaml::Hash(h) => format!("Hash({})", h.len()),
        Yaml::Alias(id) => format!("Alias({})", id),
        Yaml::Null => "Null".to_string(),
        Yaml::BadValue => "BadValue".to_string(),
    }
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

/// Validates database configuration and returns any errors found
fn validate_db_config(db_config: &Yaml) -> Vec<String> {
    let mut errors = Vec::new();

    // Check for required fields
    if let Yaml::Hash(ref hash) = db_config {
        // Check for required host field
        if !hash
            .iter()
            .any(|(k, _)| matches!(k, Yaml::String(s) if s == "host"))
        {
            errors.push("Database configuration is missing required 'host' field".to_string());
        }

        // Check for required port field
        if !hash
            .iter()
            .any(|(k, _)| matches!(k, Yaml::String(s) if s == "port"))
        {
            errors.push("Database configuration is missing required 'port' field".to_string());
        }

        // Check for required username in credentials
        if let Some(credentials) = hash
            .iter()
            .find(|(k, _)| matches!(k, Yaml::String(s) if s == "credentials"))
            .map(|(_, v)| v)
        {
            if let Yaml::Hash(ref cred_hash) = credentials {
                if !cred_hash
                    .iter()
                    .any(|(k, _)| matches!(k, Yaml::String(s) if s == "username"))
                {
                    errors.push(
                        "Database credentials is missing required 'username' field".to_string(),
                    );
                }
            }
        } else {
            errors
                .push("Database configuration is missing required 'credentials' field".to_string());
        }
    }

    errors
}
