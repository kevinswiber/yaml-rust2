//! Source mapping capabilities for YAML documents.
//!
//! This module provides structures and functions to map between YAML nodes and their
//! source positions in the original document. This is particularly useful for error
//! reporting, debugging, and tools that need to highlight specific sections of YAML
//! documents.

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Range;

use crate::position::PositionSpan;
use crate::scanner::Marker;
use crate::yaml::Yaml;

/// A unique identifier for a YAML node in the document.
///
/// This is used to create a stable reference to a node that can be used
/// for mapping between the node and its source position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) usize);

impl NodeId {
    /// Create a new node ID with the given index.
    #[must_use]
    pub fn new(id: usize) -> Self {
        NodeId(id)
    }

    /// Get the underlying ID value.
    #[must_use]
    pub fn value(&self) -> usize {
        self.0
    }
}

/// A source location range in the YAML document.
///
/// This structure represents a range of text in the original document,
/// with line and column information for both the start and end positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// The span containing start and end markers
    pub span: PositionSpan,
    /// The byte offset range in the original document
    pub byte_range: Option<Range<usize>>,
}

impl SourceLocation {
    /// Create a new source location from a position span.
    #[must_use]
    pub fn new(span: PositionSpan) -> Self {
        SourceLocation {
            span,
            byte_range: None,
        }
    }

    /// Create a new source location from a position span with byte offset information.
    #[must_use]
    pub fn with_byte_range(span: PositionSpan, range: Range<usize>) -> Self {
        SourceLocation {
            span,
            byte_range: Some(range),
        }
    }

    /// Get the start line number (1-based).
    #[must_use]
    pub fn start_line(&self) -> usize {
        self.span.start.line()
    }

    /// Get the start column number (1-based).
    #[must_use]
    pub fn start_column(&self) -> usize {
        self.span.start.col()
    }

    /// Get the end line number (1-based), if available.
    #[must_use]
    pub fn end_line(&self) -> Option<usize> {
        self.span.end.map(|m| m.line())
    }

    /// Get the end column number (1-based), if available.
    #[must_use]
    pub fn end_column(&self) -> Option<usize> {
        self.span.end.map(|m| m.col())
    }

    /// Check if this location contains the given position.
    ///
    /// # Arguments
    ///
    /// * `line` - The line number (1-based)
    /// * `column` - The column number (1-based)
    ///
    /// # Returns
    ///
    /// `true` if the position is within this location, `false` otherwise.
    #[must_use]
    pub fn contains_position(&self, line: usize, column: usize) -> bool {
        let start_line = self.start_line();
        let start_col = self.start_column();

        if let (Some(end_line), Some(end_col)) = (self.end_line(), self.end_column()) {
            // If we have both start and end positions
            if line < start_line || line > end_line {
                return false;
            }
            if line == start_line && column < start_col {
                return false;
            }
            if line == end_line && column > end_col {
                return false;
            }
            true
        } else {
            // If we only have a start position, consider it a point
            line == start_line && column == start_col
        }
    }

    /// Check if this location contains the given position.
    ///
    /// # Arguments
    ///
    /// * `marker` - The position marker
    ///
    /// # Returns
    ///
    /// `true` if the position is within this location, `false` otherwise.
    #[must_use]
    pub fn contains_marker(&self, marker: Marker) -> bool {
        self.contains_position(marker.line(), marker.col())
    }

    /// Check if this location contains the given byte offset.
    ///
    /// # Arguments
    ///
    /// * `offset` - The byte offset
    ///
    /// # Returns
    ///
    /// `true` if the offset is within this location's byte range, `false` otherwise.
    /// Always returns `false` if byte range information is not available.
    #[must_use]
    pub fn contains_byte_offset(&self, offset: usize) -> bool {
        if let Some(range) = &self.byte_range {
            range.contains(&offset)
        } else {
            false
        }
    }

    /// Format a location indicator string that highlights the source location
    ///
    /// # Arguments
    ///
    /// * `source` - The source text of the YAML document
    /// * `with_caret` - Whether to include a caret pointer at the error location
    ///
    /// # Returns
    ///
    /// A formatted string that shows the location in the source with:
    /// - The line containing the error
    /// - An optional caret pointing to the exact position
    /// - Line numbers for context
    #[must_use]
    pub fn format_location_indicator(&self, source: &str, with_caret: bool) -> String {
        let lines: Vec<&str> = source.lines().collect();

        // Get the line containing the error (adjust for 0-indexing)
        let line_number = self.start_line();
        let line_idx = line_number.saturating_sub(1);

        if line_idx >= lines.len() {
            return format!("<location out of range: line {}>", line_number);
        }

        let line_content = lines[line_idx];
        let column = self.start_column().saturating_sub(1); // Convert to 0-indexed

        let mut result = format!("{}: {}\n", line_number, line_content);

        if with_caret && column <= line_content.len() {
            let padding = " ".repeat(line_number.to_string().len() + 2 + column);
            result.push_str(&format!("{}^\n", padding));
        }

        result
    }

    /// Format an error message including the source code context for this location
    ///
    /// # Arguments
    ///
    /// * `message` - The error message to display
    /// * `source` - The source text of the YAML document
    ///
    /// # Returns
    ///
    /// A formatted error message that includes the original message, location,
    /// and relevant code snippet with a pointer
    #[must_use]
    pub fn format_error(&self, message: &str, source: &str) -> String {
        let location_str = format!("{}:{}", self.start_line(), self.start_column());
        let location_indicator = self.format_location_indicator(source, true);

        format!(
            "Error at {}: {}\n{}",
            location_str, message, location_indicator
        )
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.start_line(), self.start_column())?;
        if let (Some(end_line), Some(end_column)) = (self.end_line(), self.end_column()) {
            write!(f, "-{}:{}", end_line, end_column)?;
        }
        Ok(())
    }
}

/// A source map that provides bidirectional mapping between YAML nodes and their source locations.
///
/// The source map tracks the positions of all nodes in a YAML document and allows querying
/// by either node ID or position in the document.
#[derive(Debug, Clone)]
pub struct SourceMap<T = Yaml> {
    /// Maps node IDs to their source locations
    id_to_location: HashMap<NodeId, SourceLocation>,
    /// Maps node IDs to the actual nodes
    id_to_node: HashMap<NodeId, T>,
    /// The next available node ID
    next_id: usize,
    /// Phantom data for the node type
    _marker: PhantomData<T>,
}

impl<T> SourceMap<T>
where
    T: Clone + Eq + Hash,
{
    /// Create a new empty source map.
    #[must_use]
    pub fn new() -> Self {
        SourceMap {
            id_to_location: HashMap::new(),
            id_to_node: HashMap::new(),
            next_id: 1, // Start from 1 to avoid confusion with 0 (null)
            _marker: PhantomData,
        }
    }

    /// Register a node with its source location.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to register
    /// * `location` - The source location of the node
    ///
    /// # Returns
    ///
    /// The ID assigned to the node.
    pub fn register_node(&mut self, node: T, location: SourceLocation) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.id_to_location.insert(id, location);
        self.id_to_node.insert(id, node);
        id
    }

    /// Register a node with its source location, using a specific ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID to assign to the node
    /// * `node` - The node to register
    /// * `location` - The source location of the node
    ///
    /// # Returns
    ///
    /// `true` if the node was registered successfully, `false` if the ID was already in use.
    pub fn register_node_with_id(&mut self, id: NodeId, node: T, location: SourceLocation) -> bool {
        if self.id_to_location.contains_key(&id) {
            return false;
        }

        self.id_to_location.insert(id, location);
        self.id_to_node.insert(id, node);

        // Update next_id if necessary
        if id.0 >= self.next_id {
            self.next_id = id.0 + 1;
        }

        true
    }

    /// Get the source location of a node by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node
    ///
    /// # Returns
    ///
    /// The source location of the node, or `None` if not found.
    #[must_use]
    pub fn get_location(&self, id: NodeId) -> Option<&SourceLocation> {
        self.id_to_location.get(&id)
    }

    /// Get the node with the given ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node
    ///
    /// # Returns
    ///
    /// The node, or `None` if not found.
    #[must_use]
    pub fn get_node(&self, id: NodeId) -> Option<&T> {
        self.id_to_node.get(&id)
    }

    /// Find the most specific node ID that contains the given position.
    ///
    /// # Arguments
    ///
    /// * `line` - The line number (1-based)
    /// * `column` - The column number (1-based)
    ///
    /// # Returns
    ///
    /// The ID of the most specific node that contains the position, or `None` if not found.
    #[must_use]
    pub fn find_node_at_position(&self, line: usize, column: usize) -> Option<NodeId> {
        // Find all nodes that contain the position
        let mut matching_ids: Vec<NodeId> = self
            .id_to_location
            .iter()
            .filter_map(|(&id, loc)| {
                if loc.contains_position(line, column) {
                    Some((id, loc))
                } else {
                    None
                }
            })
            .map(|(id, _)| id)
            .collect();

        // If we found multiple matches, return the most specific one
        // (the one with the smallest area)
        if matching_ids.len() > 1 {
            matching_ids.sort_by_key(|id| {
                let loc = &self.id_to_location[id];
                let start_line = loc.start_line();
                let start_col = loc.start_column();
                let end_line = loc.end_line().unwrap_or(start_line);
                let end_col = loc.end_column().unwrap_or(start_col);

                // Calculate the "area" of the location (lines × columns)
                let lines = end_line - start_line + 1;
                let cols = if start_line == end_line {
                    end_col - start_col + 1
                } else {
                    // For multi-line spans, we'll use a simplified metric
                    // that considers the overall size
                    end_col + 80 * (lines - 1)
                };

                lines * cols
            });

            Some(matching_ids[0])
        } else {
            matching_ids.into_iter().next()
        }
    }

    /// Find the most specific node ID that contains the given marker.
    ///
    /// # Arguments
    ///
    /// * `marker` - The position marker
    ///
    /// # Returns
    ///
    /// The ID of the most specific node that contains the marker, or `None` if not found.
    #[must_use]
    pub fn find_node_at_marker(&self, marker: Marker) -> Option<NodeId> {
        self.find_node_at_position(marker.line(), marker.col())
    }

    /// Find the node ID that contains the given byte offset.
    ///
    /// # Arguments
    ///
    /// * `offset` - The byte offset
    ///
    /// # Returns
    ///
    /// The ID of the node that contains the offset, or `None` if not found.
    #[must_use]
    pub fn find_node_at_byte_offset(&self, offset: usize) -> Option<NodeId> {
        // Find all nodes that contain the offset
        let mut matching_ids: Vec<NodeId> = self
            .id_to_location
            .iter()
            .filter_map(|(&id, loc)| {
                if loc.contains_byte_offset(offset) {
                    Some((id, loc))
                } else {
                    None
                }
            })
            .map(|(id, _)| id)
            .collect();

        // If we found multiple matches, return the most specific one
        // (the one with the smallest byte range)
        if matching_ids.len() > 1 {
            matching_ids.sort_by_key(|id| {
                let loc = &self.id_to_location[id];
                if let Some(range) = &loc.byte_range {
                    range.end - range.start
                } else {
                    usize::MAX
                }
            });

            Some(matching_ids[0])
        } else {
            matching_ids.into_iter().next()
        }
    }

    /// Get all registered node IDs.
    ///
    /// # Returns
    ///
    /// A vector of all registered node IDs.
    #[must_use]
    pub fn get_all_node_ids(&self) -> Vec<NodeId> {
        self.id_to_node.keys().copied().collect()
    }

    /// Get the number of nodes in the source map.
    ///
    /// # Returns
    ///
    /// The number of nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.id_to_node.len()
    }

    /// Check if the source map is empty.
    ///
    /// # Returns
    ///
    /// `true` if the source map contains no nodes, `false` otherwise.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.id_to_node.is_empty()
    }

    /// Clear the source map, removing all nodes and their locations.
    pub fn clear(&mut self) {
        self.id_to_location.clear();
        self.id_to_node.clear();
        self.next_id = 1;
    }

    /// Find the nodes that are contained within a specific range of lines
    ///
    /// This method searches for nodes that have spans overlapping with the specified line range.
    ///
    /// # Arguments
    ///
    /// * `start_line` - The start line (1-indexed)
    /// * `start_col` - The start column (1-indexed)
    /// * `end_line` - The end line (1-indexed)
    /// * `end_col` - The end column (1-indexed)
    ///
    /// # Returns
    ///
    /// A vector of node IDs that are within the specified range
    #[must_use]
    pub fn find_nodes_in_range(
        &self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> Vec<NodeId> {
        let _start_idx = start_line.saturating_sub(1); // Convert to 0-indexed
        let _end_idx = end_line.saturating_sub(1); // Convert to 0-indexed

        // Find all nodes that have spans overlapping with the given range
        self.id_to_location
            .iter()
            .filter_map(|(&id, location)| {
                // Check if the location overlaps with the specified range
                let loc_start_line = location.start_line();
                let loc_start_column = location.start_column();
                let loc_end_line = location.end_line().unwrap_or(loc_start_line);
                let loc_end_column = location.end_column().unwrap_or(loc_start_column);

                // Location overlaps with range if:
                // 1. Location's end is after or at the range start
                // 2. Location's start is before or at the range end
                let location_ends_after_range_start = (loc_end_line > start_line)
                    || (loc_end_line == start_line && loc_end_column >= start_col);

                let location_starts_before_range_end = (loc_start_line < end_line)
                    || (loc_start_line == end_line && loc_start_column <= end_col);

                if location_ends_after_range_start && location_starts_before_range_end {
                    Some(id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find the deepest (most specific) node at a given position.
    ///
    /// This is similar to `find_node_at_position`, but explicitly looks for
    /// the smallest, most specific node that contains the position. This is
    /// useful for finding the most precise node when multiple nodes overlap.
    ///
    /// # Arguments
    ///
    /// * `line` - The line number (1-based)
    /// * `column` - The column number (1-based)
    ///
    /// # Returns
    ///
    /// The ID of the deepest node that contains the position, or `None` if not found.
    #[must_use]
    pub fn find_deepest_node_at_position(&self, line: usize, column: usize) -> Option<NodeId> {
        // Find all nodes that contain the position
        let matching_ids: Vec<NodeId> = self
            .id_to_location
            .iter()
            .filter_map(|(&id, loc)| {
                if loc.contains_position(line, column) {
                    Some((id, loc))
                } else {
                    None
                }
            })
            .map(|(id, _)| id)
            .collect();

        if matching_ids.is_empty() {
            return None;
        }

        // If we found multiple matches, return the most specific one
        // (the one with the smallest area)
        if matching_ids.len() > 1 {
            let mut smallest_id = matching_ids[0];
            let mut smallest_area = usize::MAX;

            for id in matching_ids {
                let loc = &self.id_to_location[&id];
                let start_line = loc.start_line();
                let start_col = loc.start_column();
                let end_line = loc.end_line().unwrap_or(start_line);
                let end_col = loc.end_column().unwrap_or(start_col);

                // Calculate the "area" of the location (lines × columns)
                let lines = end_line - start_line + 1;
                let cols = if start_line == end_line {
                    end_col - start_col + 1
                } else {
                    // For multi-line spans, we'll use a simplified metric
                    // that considers the overall size
                    end_col + 80 * (lines - 1)
                };

                let area = lines * cols;
                if area < smallest_area {
                    smallest_area = area;
                    smallest_id = id;
                }
            }

            Some(smallest_id)
        } else {
            Some(matching_ids[0])
        }
    }

    /// Highlight a node in the source document
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to highlight
    /// * `source` - The source text of the YAML document
    /// * `context_lines` - Number of context lines to show around the node (0 to show only the node lines)
    ///
    /// # Returns
    ///
    /// A formatted string showing the node's source with highlighting
    #[must_use]
    pub fn highlight_node(&self, id: NodeId, source: &str, context_lines: usize) -> String {
        let location = match self.get_location(id) {
            Some(loc) => loc,
            None => return String::from("Node not found in source map"),
        };

        let lines: Vec<&str> = source.lines().collect();

        // Get the start and end lines
        let start_line = location.start_line();
        let _start_idx = start_line.saturating_sub(1); // Convert to 0-indexed
        let end_line = location.end_line().unwrap_or(start_line);
        let _end_idx = end_line.saturating_sub(1); // Convert to 0-indexed

        // Calculate context range
        let context_start = start_line.saturating_sub(context_lines);
        let context_end = (end_line + context_lines).min(lines.len());

        let mut result = String::new();

        // Add line numbers and content with highlighting
        for i in context_start..=context_end {
            let idx = i.saturating_sub(1);
            if idx < lines.len() {
                let line_content = lines[idx];

                // Highlight the node lines
                if i >= start_line && i <= end_line {
                    result.push_str(&format!("> {}: {}\n", i, line_content));
                } else {
                    result.push_str(&format!("  {}: {}\n", i, line_content));
                }
            }
        }

        result
    }

    /// Format errors for nodes based on a map of error messages
    ///
    /// # Arguments
    ///
    /// * `errors` - A map of node IDs to error messages
    /// * `source` - The source text of the YAML document
    ///
    /// # Returns
    ///
    /// A vector of formatted error messages with source location information
    #[must_use]
    pub fn format_errors(&self, errors: &HashMap<NodeId, String>, source: &str) -> Vec<String> {
        let mut result = Vec::new();

        for (id, message) in errors {
            if let Some(location) = self.get_location(*id) {
                result.push(location.format_error(message, source));
            }
        }

        result
    }
}

impl<T> Default for SourceMap<T>
where
    T: Clone + Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A builder for constructing a YAML source map from a document.
#[derive(Debug)]
pub struct SourceMapBuilder {
    /// The source map being built
    source_map: SourceMap<Yaml>,
}

impl SourceMapBuilder {
    /// Create a new source map builder.
    #[must_use]
    pub fn new() -> Self {
        SourceMapBuilder {
            source_map: SourceMap::new(),
        }
    }

    /// Register a node with its source location.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to register
    /// * `span` - The position span of the node
    ///
    /// # Returns
    ///
    /// The ID assigned to the node.
    pub fn register_node(&mut self, node: Yaml, span: PositionSpan) -> NodeId {
        let location = SourceLocation::new(span);
        self.source_map.register_node(node, location)
    }

    /// Register a node with its source location and byte range.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to register
    /// * `span` - The position span of the node
    /// * `byte_range` - The byte offset range in the original document
    ///
    /// # Returns
    ///
    /// The ID assigned to the node.
    pub fn register_node_with_byte_range(
        &mut self,
        node: Yaml,
        span: PositionSpan,
        byte_range: Range<usize>,
    ) -> NodeId {
        let location = SourceLocation::with_byte_range(span, byte_range);
        self.source_map.register_node(node, location)
    }

    /// Build the source map from a YAML document.
    ///
    /// This method traverses the document and registers all nodes with their
    /// source locations. If the document was parsed with the position tracking
    /// feature enabled, this will produce a complete source map.
    ///
    /// # Arguments
    ///
    /// * `document` - The YAML document
    /// * `position_spans` - A map of nodes to their position spans
    ///
    /// # Returns
    ///
    /// The built source map.
    pub fn build(
        mut self,
        document: &Yaml,
        position_spans: &HashMap<*const Yaml, PositionSpan>,
    ) -> SourceMap<Yaml> {
        // Helper function to recursively register nodes
        fn register_nodes(
            builder: &mut SourceMapBuilder,
            node: &Yaml,
            position_spans: &HashMap<*const Yaml, PositionSpan>,
        ) -> Option<NodeId> {
            let node_ptr = node as *const Yaml;
            let span = position_spans.get(&node_ptr)?;

            let node_id = builder.register_node(node.clone(), *span);

            // Recursively register child nodes
            match node {
                Yaml::Array(array) => {
                    for item in array {
                        register_nodes(builder, item, position_spans);
                    }
                }
                Yaml::Hash(hash) => {
                    for (key, value) in hash {
                        register_nodes(builder, key, position_spans);
                        register_nodes(builder, value, position_spans);
                    }
                }
                _ => {}
            }

            Some(node_id)
        }

        register_nodes(&mut self, document, position_spans);
        self.source_map
    }

    /// Build an empty source map.
    ///
    /// This method returns the current state of the source map without further processing.
    ///
    /// # Returns
    ///
    /// The built source map.
    #[must_use]
    pub fn build_empty(self) -> SourceMap<Yaml> {
        self.source_map
    }
}

impl Default for SourceMapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// An extension trait for `PositionTrackedLoader` to enable building source maps.
pub trait SourceMapSupport {
    /// Build a source map from the loaded YAML documents.
    ///
    /// # Returns
    ///
    /// A vector of source maps, one for each document in the loader.
    #[must_use]
    fn build_source_maps(&self) -> Vec<SourceMap<Yaml>>;

    /// Build a source map for a specific document.
    ///
    /// # Arguments
    ///
    /// * `document_index` - The index of the document to build a source map for
    ///
    /// # Returns
    ///
    /// The source map for the specified document, or `None` if the index is out of bounds.
    #[must_use]
    fn build_source_map_for_document(&self, document_index: usize) -> Option<SourceMap<Yaml>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_contains_position() {
        let start = Marker::for_testing(1, 1);
        let end = Marker::for_testing(2, 5);
        let span = PositionSpan::with_end(start, end);
        let location = SourceLocation::new(span);

        // Points within the range
        assert!(location.contains_position(1, 1)); // Start point
        assert!(location.contains_position(1, 10)); // Middle of first line
        assert!(location.contains_position(2, 1)); // Start of second line
        assert!(location.contains_position(2, 5)); // End point

        // Points outside the range
        assert!(!location.contains_position(1, 0)); // Before start column
        assert!(!location.contains_position(0, 5)); // Before start line
        assert!(!location.contains_position(2, 6)); // After end column
        assert!(!location.contains_position(3, 1)); // After end line
    }

    #[test]
    fn test_source_map_find_node_at_position() {
        let mut source_map = SourceMap::<&str>::new();

        // Register some nodes
        let id1 = source_map.register_node(
            "root",
            SourceLocation::new(PositionSpan::with_end(
                Marker::for_testing(1, 1),
                Marker::for_testing(10, 1),
            )),
        );

        let id2 = source_map.register_node(
            "child1",
            SourceLocation::new(PositionSpan::with_end(
                Marker::for_testing(2, 1),
                Marker::for_testing(3, 10),
            )),
        );

        let id3 = source_map.register_node(
            "child2",
            SourceLocation::new(PositionSpan::with_end(
                Marker::for_testing(5, 1),
                Marker::for_testing(6, 10),
            )),
        );

        // Test finding nodes
        assert_eq!(source_map.find_node_at_position(1, 1), Some(id1)); // Root start
        assert_eq!(source_map.find_node_at_position(2, 5), Some(id2)); // Child1 middle
        assert_eq!(source_map.find_node_at_position(5, 5), Some(id3)); // Child2 middle
        assert_eq!(source_map.find_node_at_position(4, 1), Some(id1)); // Between children, root only
        assert_eq!(source_map.find_node_at_position(11, 1), None); // Outside all nodes
    }

    #[test]
    fn test_source_map_with_byte_offsets() {
        let mut source_map = SourceMap::<&str>::new();

        // Register nodes with byte ranges
        let id1 = source_map.register_node(
            "node1",
            SourceLocation::with_byte_range(
                PositionSpan::with_end(Marker::for_testing(1, 1), Marker::for_testing(1, 10)),
                0..10,
            ),
        );

        let id2 = source_map.register_node(
            "node2",
            SourceLocation::with_byte_range(
                PositionSpan::with_end(Marker::for_testing(2, 1), Marker::for_testing(2, 10)),
                10..20,
            ),
        );

        // Test finding nodes by byte offset
        assert_eq!(source_map.find_node_at_byte_offset(5), Some(id1));
        assert_eq!(source_map.find_node_at_byte_offset(15), Some(id2));
        assert_eq!(source_map.find_node_at_byte_offset(25), None);
    }
}
