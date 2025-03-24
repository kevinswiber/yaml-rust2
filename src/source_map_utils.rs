//! Utility functions and high-level API for working with YAML source maps
//!
//! This module provides a more user-friendly API for working with source maps
//! than the core `SourceMap` struct. It includes functions for common tasks
//! like finding nodes by path, getting line/column information for nodes,
//! formatting error messages, and more.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::parser::Parser;
use crate::source_map::{SourceLocation, SourceMap, SourceMapSupport};
use crate::NodeId;
use crate::PositionTrackedLoader;
use crate::Yaml;

/// A utility struct that wraps a Yaml document and its source map.
pub struct YamlWithSourceMap {
    /// The YAML document
    pub document: Yaml,
    /// The source map for the document
    pub source_map: SourceMap<Yaml>,
    /// The source string
    source: String,
}

impl YamlWithSourceMap {
    /// Parse a YAML string and build a source map for it.
    ///
    /// # Arguments
    ///
    /// * `yaml_str` - The YAML string to parse
    ///
    /// # Returns
    ///
    /// A `YamlWithSourceMap` containing the parsed document and its source map.
    ///
    /// # Errors
    ///
    /// Returns an error if the YAML could not be parsed.
    pub fn parse(yaml_str: &str) -> Result<Self, String> {
        let mut loader = PositionTrackedLoader::default();
        let mut parser = Parser::new(yaml_str.chars());

        // Parse the YAML to capture position information
        parser
            .load(&mut loader, true)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;

        let documents = loader.documents();
        if documents.is_empty() {
            return Err("No YAML documents found".to_string());
        }

        // Build source maps
        let source_maps = loader.build_source_maps();
        if source_maps.is_empty() {
            return Err("Failed to build source map".to_string());
        }

        Ok(YamlWithSourceMap {
            document: documents[0].clone(),
            source_map: source_maps[0].clone(),
            source: yaml_str.to_string(),
        })
    }

    /// Parse a YAML file and build a source map for it.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path to the YAML file
    ///
    /// # Returns
    ///
    /// A `YamlWithSourceMap` containing the parsed document and its source map.
    ///
    /// # Errors
    ///
    /// Returns an error if the file could not be read or the YAML could not be parsed.
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, String> {
        let content =
            fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;
        Self::parse(&content)
    }

    /// Get the source text
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Find a node in the document by path.
    ///
    /// # Arguments
    ///
    /// * `path` - A slice of strings representing the path to the node
    ///
    /// # Returns
    ///
    /// The node at the given path, or `None` if not found.
    pub fn find_node_by_path(&self, path: &[&str]) -> Option<&Yaml> {
        let mut current = &self.document;
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

    /// Get the location of a node in the source.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to locate
    ///
    /// # Returns
    ///
    /// The location of the node, or `None` if not found.
    pub fn get_node_location(&self, node: &Yaml) -> Option<&SourceLocation> {
        let id = self.find_node_id(node)?;
        self.source_map.get_location(id)
    }

    /// Find a node at a specific line and column.
    ///
    /// # Arguments
    ///
    /// * `line` - The line number (1-based)
    /// * `column` - The column number (1-based)
    ///
    /// # Returns
    ///
    /// The node at the given position, or `None` if not found.
    pub fn find_node_at_position(&self, line: usize, column: usize) -> Option<&Yaml> {
        let id = self.source_map.find_node_at_position(line, column)?;
        self.source_map.get_node(id)
    }

    /// Find all nodes in a range of positions.
    ///
    /// # Arguments
    ///
    /// * `start_line` - The starting line number (1-based)
    /// * `start_col` - The starting column number (1-based)
    /// * `end_line` - The ending line number (1-based)
    /// * `end_col` - The ending column number (1-based)
    ///
    /// # Returns
    ///
    /// A vector of nodes in the given range.
    pub fn find_nodes_in_range(
        &self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> Vec<&Yaml> {
        let ids = self
            .source_map
            .find_nodes_in_range(start_line, start_col, end_line, end_col);
        ids.into_iter()
            .filter_map(|id| self.source_map.get_node(id))
            .collect()
    }

    /// Format an error message with source context.
    ///
    /// # Arguments
    ///
    /// * `node` - The node associated with the error
    /// * `message` - The error message
    ///
    /// # Returns
    ///
    /// A formatted error string with source context, or just the message if the node is not found.
    pub fn format_error(&self, node: &Yaml, message: &str) -> String {
        if let Some(loc) = self.get_node_location(node) {
            loc.format_error(message, &self.source)
        } else {
            format!("Error: {}", message)
        }
    }

    /// Create an error report for multiple nodes.
    ///
    /// # Arguments
    ///
    /// * `errors` - A map from nodes to error messages
    ///
    /// # Returns
    ///
    /// A vector of formatted error strings.
    pub fn format_errors(&self, errors: &HashMap<&Yaml, String>) -> Vec<String> {
        let mut result = Vec::new();
        let mut node_id_errors = HashMap::new();

        // Convert from &Yaml -> String to NodeId -> String
        for (node, message) in errors {
            if let Some(id) = self.find_node_id(node) {
                node_id_errors.insert(id, message.clone());
            } else {
                result.push(format!("Error: {}", message));
            }
        }

        // Format errors for nodes that were found
        if !node_id_errors.is_empty() {
            result.extend(self.source_map.format_errors(&node_id_errors, &self.source));
        }

        result
    }

    /// Highlight a node in the source.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to highlight
    /// * `context_lines` - The number of context lines to include (default: 2)
    ///
    /// # Returns
    ///
    /// A formatted string with the node highlighted, or an empty string if the node is not found.
    pub fn highlight_node(&self, node: &Yaml, context_lines: usize) -> String {
        if let Some(id) = self.find_node_id(node) {
            self.source_map
                .highlight_node(id, &self.source, context_lines)
        } else {
            String::new()
        }
    }

    /// Get a user-friendly string representation of a node's position.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to locate
    ///
    /// # Returns
    ///
    /// A string like "line 5, column 10", or "unknown position" if the node is not found.
    pub fn node_position_string(&self, node: &Yaml) -> String {
        if let Some(loc) = self.get_node_location(node) {
            let start_line = loc.start_line();
            let start_col = loc.start_column();
            let end_line = loc.end_line();
            let end_col = loc.end_column();

            if let (Some(end_line), Some(end_col)) = (end_line, end_col) {
                if start_line == end_line {
                    if start_col == end_col {
                        format!("line {}, column {}", start_line, start_col)
                    } else {
                        format!("line {}, columns {}-{}", start_line, start_col, end_col)
                    }
                } else {
                    format!(
                        "lines {}-{}, columns {}-{}",
                        start_line, end_line, start_col, end_col
                    )
                }
            } else {
                format!("line {}, column {}", start_line, start_col)
            }
        } else {
            "unknown position".to_string()
        }
    }

    // Helper method to find a node ID by node reference
    fn find_node_id(&self, node: &Yaml) -> Option<NodeId> {
        for id in self.source_map.get_all_node_ids() {
            if let Some(map_node) = self.source_map.get_node(id) {
                if equal_yaml_content(map_node, node) {
                    return Some(id);
                }
            }
        }
        None
    }
}

/// A simple utility to check if two YAML nodes have the same content.
///
/// This is used internally to find node IDs by content, since we can't
/// rely on reference equality when working with cloned nodes.
///
/// # Arguments
///
/// * `a` - The first node
/// * `b` - The second node
///
/// # Returns
///
/// `true` if the nodes have the same content, `false` otherwise.
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

/// A utility to get a preview of a YAML node's content as a string.
///
/// This is useful for debugging and error messages.
///
/// # Arguments
///
/// * `node` - The YAML node
/// * `max_length` - The maximum length of the preview (default: 40)
///
/// # Returns
///
/// A string representation of the node's content.
pub fn node_preview(node: &Yaml, max_length: usize) -> String {
    match node {
        Yaml::Real(r) => format!("Real({})", r),
        Yaml::Integer(i) => format!("Integer({})", i),
        Yaml::String(s) => {
            let preview = if s.len() > max_length {
                format!("{}...", &s[0..max_length.saturating_sub(3)])
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

/// A utility to get the type name of a YAML node.
///
/// # Arguments
///
/// * `node` - The YAML node
///
/// # Returns
///
/// A string representing the type of the node.
pub fn node_type_name(node: &Yaml) -> &'static str {
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

/// Parse YAML from a string and build a source map for it.
///
/// This is a convenience function equivalent to `YamlWithSourceMap::parse`.
///
/// # Arguments
///
/// * `yaml_str` - The YAML string to parse
///
/// # Returns
///
/// A `YamlWithSourceMap` containing the parsed document and its source map.
///
/// # Errors
///
/// Returns an error if the YAML could not be parsed.
pub fn parse_yaml_with_source_map(yaml_str: &str) -> Result<YamlWithSourceMap, String> {
    YamlWithSourceMap::parse(yaml_str)
}

/// Parse YAML from a file and build a source map for it.
///
/// This is a convenience function equivalent to `YamlWithSourceMap::from_file`.
///
/// # Arguments
///
/// * `file_path` - The path to the YAML file
///
/// # Returns
///
/// A `YamlWithSourceMap` containing the parsed document and its source map.
///
/// # Errors
///
/// Returns an error if the file could not be read or the YAML could not be parsed.
pub fn parse_yaml_file_with_source_map<P: AsRef<Path>>(
    file_path: P,
) -> Result<YamlWithSourceMap, String> {
    YamlWithSourceMap::from_file(file_path)
}
