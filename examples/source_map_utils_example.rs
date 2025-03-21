#![cfg(feature = "source_mapping")]

//! Example demonstrating the use of the enhanced source mapping utilities.
//!
//! This example shows how to use the new `YamlWithSourceMap` API for working
//! with YAML documents and their source maps. The API provides a more
//! user-friendly interface than the core `SourceMap` API, making it easier
//! to find nodes by path, get position information, format error messages, etc.

use std::collections::HashMap;
use yaml_rust2::{node_preview, node_type_name, parse_yaml_with_source_map, Yaml};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== YAML Source Map Utilities Example ===\n");

    // Parse a YAML document with source mapping
    let yaml_str = r#"
# Configuration example
server:
  host: localhost
  port: 8080
  debug: true
database:
  host: db.example.com
  port: 5432
  username: admin
  # password: secret  # commented out for security
logging:
  level: info
  file: /var/log/app.log
features:
  - authentication
  - authorization
  - api
  - dashboard
"#;

    // Parse the YAML and build a source map using the new API
    let yaml_with_map = parse_yaml_with_source_map(yaml_str)?;

    // 1. Basic information about the document
    println!("=== Document Overview ===");
    print_yaml_structure(&yaml_with_map.document, 0);

    // 2. Find nodes by path
    println!("\n=== Finding Nodes by Path ===");

    // Find the server host node
    let server_host = yaml_with_map.find_node_by_path(&["server", "host"]);
    if let Some(node) = server_host {
        println!("Server host: {:?}", node);

        // Get position information for the node
        println!("Position: {}", yaml_with_map.node_position_string(node));

        // Highlight the node in the source
        println!("\nNode highlighted in source:");
        println!("{}", yaml_with_map.highlight_node(node, 2));
    }

    // Find the features array
    let features = yaml_with_map.find_node_by_path(&["features"]);
    if let Some(node) = features {
        println!("\nFeatures array: {}", node_preview(node, 50));
        println!("Position: {}", yaml_with_map.node_position_string(node));
    }

    // 3. Find nodes by position
    println!("\n=== Finding Nodes by Position ===");

    // Find the node at line 8, column 3 (should be db.example.com)
    let line = 8;
    let col = 8;
    if let Some(node) = yaml_with_map.find_node_at_position(line, col) {
        println!(
            "Node at ({}, {}): {:?} ({})",
            line,
            col,
            node,
            node_type_name(node)
        );
    }

    // 4. Validate the document and report errors with position information
    println!("\n=== Validation with Error Reporting ===");

    // Collect validation errors
    let mut errors = HashMap::new();

    // Check if database password is present
    let db_node = yaml_with_map.find_node_by_path(&["database"]);
    if let Some(db) = db_node {
        if db.is_hash() {
            // Check if password key exists in the database section
            let has_password = yaml_with_map
                .find_node_by_path(&["database", "password"])
                .is_some();
            if !has_password {
                errors.insert(
                    db,
                    "Database configuration is missing a password field".to_string(),
                );
            }
        }
    }

    // Check if logging level is valid
    let log_level = yaml_with_map.find_node_by_path(&["logging", "level"]);
    if let Some(level) = log_level {
        if let Yaml::String(value) = level {
            if !["debug", "info", "warn", "error"].contains(&value.as_str()) {
                errors.insert(
                    level,
                    format!(
                        "Invalid log level '{}'. Valid values are: debug, info, warn, error",
                        value
                    ),
                );
            }
        }
    }

    // Format and display errors with source context
    if !errors.is_empty() {
        println!("Validation errors found:");
        for error_msg in yaml_with_map.format_errors(&errors) {
            println!("{}\n", error_msg);
        }
    } else {
        println!("No validation errors found");
    }

    // 5. Find all nodes in a specific range
    println!("\n=== Finding Nodes in a Range ===");
    let nodes_in_range = yaml_with_map.find_nodes_in_range(3, 1, 6, 20);
    println!(
        "Found {} nodes in range (3,1) to (6,20):",
        nodes_in_range.len()
    );
    for (i, node) in nodes_in_range.iter().enumerate() {
        println!(
            "{}: {} at {}",
            i + 1,
            node_preview(node, 30),
            yaml_with_map.node_position_string(node)
        );
    }

    // 6. Generate a complete report for the document
    println!("\n=== Document Analysis Report ===");

    // Count node types
    let mut type_counts = HashMap::new();
    count_node_types(&yaml_with_map.document, &mut type_counts);

    println!("Document statistics:");
    println!(
        "  - Total top-level keys: {}",
        match &yaml_with_map.document {
            Yaml::Hash(h) => h.len(),
            _ => 0,
        }
    );

    println!("  - Node type distribution:");
    for (type_name, count) in type_counts {
        println!("    - {}: {}", type_name, count);
    }

    // Find the deepest nested node
    if let Some((path, depth)) = find_deepest_path(&yaml_with_map.document) {
        println!("  - Deepest nested node:");
        println!("    - Path: {}", path.join(" > "));
        println!("    - Depth: {}", depth);

        // Find the actual node to get its position
        if let Some(node) = find_node_by_string_path(&yaml_with_map.document, &path) {
            println!(
                "    - Position: {}",
                yaml_with_map.node_position_string(node)
            );
        }
    }

    Ok(())
}

// Helper function to print the YAML structure with indentation
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

// Helper function to count node types in a YAML document
fn count_node_types(node: &Yaml, counts: &mut HashMap<&'static str, usize>) {
    let type_name = node_type_name(node);
    *counts.entry(type_name).or_insert(0) += 1;

    match node {
        Yaml::Hash(hash) => {
            for (k, v) in hash {
                count_node_types(k, counts);
                count_node_types(v, counts);
            }
        }
        Yaml::Array(array) => {
            for item in array {
                count_node_types(item, counts);
            }
        }
        _ => {}
    }
}

// Helper function to find the deepest nested path in a YAML document
fn find_deepest_path(node: &Yaml) -> Option<(Vec<String>, usize)> {
    match node {
        Yaml::Hash(hash) => {
            let mut deepest_path = None;
            let mut max_depth = 0;

            for (k, v) in hash {
                let key_str = match k {
                    Yaml::String(s) => s.clone(),
                    _ => format!("{:?}", k),
                };

                if let Some((mut path, depth)) = find_deepest_path(v) {
                    path.insert(0, key_str);
                    if depth + 1 > max_depth {
                        max_depth = depth + 1;
                        deepest_path = Some((path, max_depth));
                    }
                } else {
                    // Leaf node
                    if 1 > max_depth {
                        max_depth = 1;
                        deepest_path = Some((vec![key_str], 1));
                    }
                }
            }

            deepest_path
        }
        Yaml::Array(array) => {
            let mut deepest_path = None;
            let mut max_depth = 0;

            for (i, item) in array.iter().enumerate() {
                if let Some((mut path, depth)) = find_deepest_path(item) {
                    path.insert(0, i.to_string());
                    if depth + 1 > max_depth {
                        max_depth = depth + 1;
                        deepest_path = Some((path, max_depth));
                    }
                } else {
                    // Leaf node
                    if 1 > max_depth {
                        max_depth = 1;
                        deepest_path = Some((vec![i.to_string()], 1));
                    }
                }
            }

            deepest_path
        }
        _ => None, // Leaf node
    }
}

// Helper function to find a node by string path
fn find_node_by_string_path<'a>(node: &'a Yaml, path: &[String]) -> Option<&'a Yaml> {
    if path.is_empty() {
        return Some(node);
    }

    match node {
        Yaml::Hash(hash) => {
            for (k, v) in hash {
                if let Yaml::String(key) = k {
                    if key == &path[0] {
                        return find_node_by_string_path(v, &path[1..]);
                    }
                }
            }
            None
        }
        Yaml::Array(array) => {
            if let Ok(index) = path[0].parse::<usize>() {
                array
                    .get(index)
                    .and_then(|item| find_node_by_string_path(item, &path[1..]))
            } else {
                None
            }
        }
        _ => None,
    }
}
