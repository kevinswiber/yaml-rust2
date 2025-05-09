//! Position tracking for YAML constructs.
//!
//! This module provides structures for tracking the positions of YAML constructs
//! in the source document, including both start and end positions.

use std::fmt::Display;

use crate::error::ScanError;
use crate::parser::Event;
use crate::parser::Tag;
use crate::scanner::{Marker, TMappingStyle, TScalarStyle};
use crate::style::TSequenceStyle;
use crate::yaml::Yaml;

/// A unique identifier for a YAML node in the document.
///
/// This is used to create a stable reference to a node that can be used
/// for mapping between the node and its source position.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
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

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct AnchorId(pub(crate) usize);

impl AnchorId {
    /// Create a new node ID with the given index.
    #[must_use]
    pub fn new(id: usize) -> Self {
        AnchorId(id)
    }

    /// Get the underlying ID value.
    #[must_use]
    pub fn value(&self) -> usize {
        self.0
    }
}

impl Display for AnchorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A start and end position for a YAML construct.
///
/// This structure is used to track the exact position of YAML constructs
/// in the source document, with particular attention to maintaining accurate
/// positions for flow collections like mappings and sequences.
///
/// For flow collections, the positions will point to the opening and closing
/// delimiters (`{`, `}` for mappings, `[`, `]` for sequences).
#[derive(Clone, Copy, PartialEq, Debug, Eq)]
pub struct PositionSpan {
    /// The start position of the construct
    pub start: Marker,
    /// The end position of the construct (if known)
    pub end: Option<Marker>,
}

impl PositionSpan {
    /// Create a new position span with only a start position.
    ///
    /// This is typically used when beginning to parse a construct
    /// where the end position is not yet known.
    ///
    /// # Example
    /// ```ignore
    /// use yaml_rust2::position::PositionSpan;
    /// use yaml_rust2::scanner::Marker;
    ///
    /// // Assuming we have a start_marker from somewhere
    /// let span = PositionSpan::new(start_marker);
    /// ```
    #[must_use]
    pub fn new(start: Marker) -> Self {
        PositionSpan { start, end: None }
    }

    /// Create a new position span with both start and end positions.
    ///
    /// This is typically used when both positions are known,
    /// such as when completing the parsing of a construct.
    ///
    /// # Example
    /// ```ignore
    /// use yaml_rust2::position::PositionSpan;
    /// use yaml_rust2::scanner::Marker;
    ///
    /// // Assuming we have start_marker and end_marker from somewhere
    /// let span = PositionSpan::with_end(start_marker, end_marker);
    /// ```
    #[must_use]
    pub fn with_end(start: Marker, end: Marker) -> Self {
        PositionSpan {
            start,
            end: Some(end),
        }
    }

    /// Set the end position for this span.
    ///
    /// This is used when the end position becomes available after
    /// the span was initially created with just a start position.
    ///
    /// # Example
    /// ```ignore
    /// use yaml_rust2::position::PositionSpan;
    /// use yaml_rust2::scanner::Marker;
    ///
    /// // Assuming we have start_marker and end_marker from somewhere
    /// let mut span = PositionSpan::new(start_marker);
    /// span.set_end(end_marker);
    /// ```
    pub fn set_end(&mut self, end: Marker) {
        self.end = Some(end);
    }
}

/// A convenience alias for a `Result` of a parser event with position information.
pub type PositionedParseResult = Result<(Event, PositionSpan), ScanError>;

/// A structure to track open constructs and their positions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionTracker {
    /// A stack of positions for open constructs
    position_stack: Vec<(NodeId, Marker)>,
    /// Map of anchor ID to position
    anchor_positions: std::collections::HashMap<AnchorId, Marker>,
    /// Map of anchor ID to node content
    anchor_nodes: std::collections::HashMap<AnchorId, Yaml>,
    /// Map of anchor ID to node ID for better resolution
    anchor_node_ids: std::collections::HashMap<AnchorId, NodeId>,
    /// Map of node pointers to positions (for all nodes, not just anchors)
    node_positions: std::collections::HashMap<NodeId, PositionSpan>,
    /// Map of node content hash to node ID
    node_content_hash_map: std::collections::HashMap<u64, NodeId>,
    /// Map of node path to node ID for tracking non-anchored nodes
    node_path_map: std::collections::HashMap<String, NodeId>,
    /// Map of value paths to node IDs (for values, not keys)
    value_path_map: std::collections::HashMap<String, NodeId>,
    /// Counter for generating unique node IDs
    next_node_id: NodeId,
    /// Stack of path components for tracking the current path
    path_stack: Vec<String>,
    /// The current key being processed (for mapping entries)
    current_key: Option<String>,
    /// Map of node ID to node instance - stores ALL nodes for consistent identity
    all_nodes: std::collections::HashMap<NodeId, Yaml>,
    /// Map of node ID to style information
    node_styles: std::collections::HashMap<NodeId, NodeStyle>,
    /// Flag to indicate if we're processing a value (as opposed to a key)
    processing_value: bool,
    /// Parent node ID stack to track hierarchical relationships
    parent_stack: Vec<NodeId>,
    /// Key-value relationship map to track which keys are associated with which values
    key_value_map: std::collections::HashMap<NodeId, NodeId>,
    /// Context tracking for special collection types (mapping/sequence)
    context_stack: Vec<ContextType>,
}

/// Enum to represent the style of a node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStyle {
    Scalar(TScalarStyle),
    Sequence(TSequenceStyle),
    Mapping(TMappingStyle),
}

/// The type of YAML context currently being processed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextType {
    /// Processing a mapping
    Mapping(TMappingStyle),
    /// Processing a sequence
    Sequence(TSequenceStyle),
    /// Processing a document root
    Document,
}

impl PositionTracker {
    /// Create a new position tracker
    #[must_use]
    pub fn new() -> Self {
        PositionTracker {
            position_stack: Vec::new(),
            anchor_positions: std::collections::HashMap::new(),
            anchor_nodes: std::collections::HashMap::new(),
            anchor_node_ids: std::collections::HashMap::new(),
            node_positions: std::collections::HashMap::new(),
            node_content_hash_map: std::collections::HashMap::new(),
            node_path_map: std::collections::HashMap::new(),
            value_path_map: std::collections::HashMap::new(),
            next_node_id: NodeId::new(1), // Start from 1
            path_stack: Vec::new(),
            current_key: None,
            all_nodes: std::collections::HashMap::new(),
            node_styles: std::collections::HashMap::new(),
            processing_value: false,
            parent_stack: Vec::new(),
            key_value_map: std::collections::HashMap::new(),
            context_stack: vec![ContextType::Document],
        }
    }

    /// Get the current path in the YAML document
    ///
    /// This method returns the current path based on the path stack
    /// and the current key being processed.
    ///
    /// # Returns
    ///
    /// The current path as a string, or None if we're at the root
    #[must_use]
    pub fn get_current_path(&self) -> Option<String> {
        if self.path_stack.is_empty() {
            // If we're at the root, return "root"
            Some("root".to_string())
        } else {
            // Otherwise, join the path components
            Some(self.path_stack.join("."))
        }
    }

    /// Push a component onto the path stack
    ///
    /// # Arguments
    ///
    /// * `component` - The path component to push
    pub fn push_path(&mut self, component: String) {
        self.path_stack.push(component);
    }

    /// Pop a component from the path stack
    ///
    /// # Returns
    ///
    /// The popped component, or None if the stack is empty
    pub fn pop_path(&mut self) -> Option<String> {
        self.path_stack.pop()
    }

    /// Set the current key being processed
    ///
    /// # Arguments
    ///
    /// * `key` - The key being processed
    pub fn set_current_key(&mut self, key: String) {
        self.current_key = Some(key);
    }

    /// Clear the current key
    pub fn clear_current_key(&mut self) {
        self.current_key = None;
    }

    /// Push a new position to the stack
    ///
    /// This is used when starting a new construct (such as a flow mapping or sequence)
    /// where we want to track the position of the opening delimiter.
    ///
    /// The `id` parameter is used to identify the construct type (e.g., 0 for flow mappings,
    /// 1 for flow sequences).
    pub fn push(&mut self, id: NodeId, position: Marker) {
        self.position_stack.push((id, position));
    }

    /// Pop a position from the stack
    ///
    /// This is used when ending a construct (such as a flow mapping or sequence)
    /// where we want to retrieve the position of the opening delimiter.
    ///
    /// Returns `None` if the stack is empty.
    #[must_use]
    pub fn pop(&mut self) -> Option<(NodeId, Marker)> {
        self.position_stack.pop()
    }

    /// Peek at the top position on the stack
    ///
    /// This is used to check the position of the most recently opened construct
    /// without removing it from the stack.
    ///
    /// Returns `None` if the stack is empty.
    #[must_use]
    pub fn peek(&self) -> Option<&(NodeId, Marker)> {
        self.position_stack.last()
    }

    /// Check if the stack is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.position_stack.is_empty()
    }

    /// Get the size of the stack
    #[must_use]
    pub fn len(&self) -> usize {
        self.position_stack.len()
    }

    /// Track an anchor position
    ///
    /// This is used to remember the position of an anchor declaration for future reference
    /// and also associate the anchor with a node ID for better resolution
    pub fn track_anchor(&mut self, anchor_id: AnchorId, position: Marker) {
        // Track the anchor position
        self.anchor_positions.insert(anchor_id, position);

        // Also create a node ID for this anchor and track its position
        let node_id = self.track_node_position(position);

        // Associate the anchor ID with this node ID for better resolution
        self.anchor_node_ids.insert(anchor_id, node_id);
    }

    /// Find a node ID by its anchor ID
    ///
    /// This method retrieves the node ID associated with a given anchor ID,
    /// which can be used to find the actual node in a source map.
    ///
    /// # Arguments
    ///
    /// * `anchor_id` - The ID of the anchor to look up
    ///
    /// # Returns
    ///
    /// The node ID associated with the anchor, if found
    #[must_use]
    pub fn find_node_id_by_anchor(&self, anchor_id: AnchorId) -> Option<NodeId> {
        self.anchor_node_ids.get(&anchor_id).copied()
    }

    /// Track a node by its path in the YAML document
    ///
    /// This method tracks a node by its path in the YAML document.
    /// It associates the path with a node ID for better resolution.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the node in the YAML document
    /// * `node` - The node to track
    /// * `position` - The position of the node in the source
    ///
    /// # Returns
    ///
    /// * `usize` - The ID assigned to this node
    pub fn track_node_by_path(&mut self, path: &str, node: &Yaml, position: Marker) -> NodeId {
        // Check if we already have a node ID for this node
        let existing_id = self.find_node_id(node);

        // If we have an existing ID, use it to maintain pointer equality
        let node_id = if let Some(id) = existing_id {
            // Update the position for the existing node if needed
            if !self.node_positions.contains_key(&id) {
                let span = PositionSpan {
                    start: position,
                    end: Some(position), // Default to same position for start/end
                };
                self.node_positions.insert(id, span);
            }
            id
        } else {
            // First track the node position if it's a new node
            let id = self.track_node_position(position);

            // Store the node instance for consistent identity
            self.all_nodes.insert(id, node.clone());

            // Calculate and store the content hash
            let hash = Self::calculate_node_hash(node);
            self.node_content_hash_map.insert(hash, id);

            id
        };

        // Always associate the path with the node ID (even if the node already exists)
        // This ensures that nodes can be found by multiple paths if needed
        self.node_path_map.insert(path.to_string(), node_id);

        node_id
    }

    /// Find a node ID by its path in the YAML document
    ///
    /// This method finds a node ID by its path in the YAML document.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the node in the YAML document
    ///
    /// # Returns
    ///
    /// * `Some(usize)` - The node ID if found
    /// * `None` - If no node ID is associated with this path
    #[must_use]
    pub fn find_node_id_by_path(&self, path: &str) -> Option<NodeId> {
        self.node_path_map.get(path).copied()
    }

    /// Get the position of an anchor
    ///
    /// Returns the position span where the anchor was declared
    #[must_use]
    pub fn get_anchor_position(&self, anchor_id: AnchorId) -> Option<PositionSpan> {
        self.anchor_positions
            .get(&anchor_id)
            .map(|&pos| PositionSpan::new(pos))
    }

    /// Get all nodes tracked by path
    ///
    /// Returns a vector of tuples containing the path, node ID, and node
    #[must_use]
    pub fn get_nodes_by_path(&self) -> Vec<(String, NodeId, Yaml)> {
        let mut result = Vec::new();
        for (path, &node_id) in self.node_path_map.iter() {
            if let Some(node) = self.all_nodes.get(&node_id) {
                result.push((path.clone(), node_id, node.clone()));
            }
        }
        result
    }

    /// Get all anchor nodes
    ///
    /// Returns a vector of tuples containing the anchor ID and node
    #[must_use]
    pub fn get_anchor_nodes(&self) -> Vec<(AnchorId, Yaml)> {
        let mut result = Vec::new();
        for (&anchor_id, node) in self.anchor_nodes.iter() {
            result.push((anchor_id, node.clone()));
        }
        result
    }

    /// Store a node for an anchor
    ///
    /// This is used to remember both the position and content of an anchor
    /// for future reference when resolving aliases.
    ///
    /// # Arguments
    ///
    /// * `anchor_id` - The unique ID of the anchor
    /// * `node` - The YAML node content to store (can be a scalar, sequence, or mapping)
    ///
    /// This method is particularly useful when tracking complex anchors like sequences and mappings,

    /// Find a node ID by its content hash
    ///
    /// This method calculates the hash of the provided node and tries to find a matching node ID
    /// in the position tracker's node content hash map.
    ///
    /// # Arguments
    ///
    /// * `node` - The YAML node to find by content
    ///
    /// # Returns
    ///
    /// The node ID if a matching node was found, or None otherwise
    #[must_use]
    pub fn get_node_hash_by_content(&self, node: &Yaml) -> Option<NodeId> {
        let hash = Self::calculate_node_hash(node);
        self.node_content_hash_map.get(&hash).copied()
    }
    /// as it allows for complete node resolution when encountering aliases.
    pub fn store_anchor_node(&mut self, anchor_id: AnchorId, node: Yaml) {
        // Store the node in both the anchor_nodes map and the all_nodes map
        // This ensures that the same node instance is used consistently
        let _node_id = self.store_node(node.clone());
        self.anchor_nodes.insert(anchor_id, node);
    }

    /// Store a node in the all_nodes map to ensure consistent node instances
    ///
    /// This method stores a node in the all_nodes map and returns its ID.
    /// If the node already exists in the map (by content equality), it returns the existing ID.
    ///
    /// # Arguments
    ///
    /// * `node` - The YAML node to store
    ///
    /// # Returns
    ///
    /// The ID of the stored node
    pub fn store_node(&mut self, node: Yaml) -> NodeId {
        // Calculate a hash for the node's content
        let hash = Self::calculate_node_hash(&node);

        // Check if we already have a node with this hash
        if let Some(&existing_id) = self.node_content_hash_map.get(&hash) {
            // If we do, check if the content is actually equal
            // This is to handle hash collisions
            if let Some(existing_node) = self.all_nodes.get(&existing_id) {
                if existing_node == &node {
                    // We already have this node, return its ID
                    return existing_id;
                }
            }
        }

        // If we don't have this node yet, create a new ID and store it
        let node_id = self.next_node_id;
        self.next_node_id = NodeId::new(self.next_node_id.value() + 1);

        // Store the node in the all_nodes map
        self.all_nodes.insert(node_id, node);

        // Store the hash mapping for future lookups
        self.node_content_hash_map.insert(hash, node_id);

        node_id
    }

    /// Get the node associated with an anchor
    ///
    /// Returns the node that was stored for the given anchor ID,
    /// or None if no node was stored.
    ///
    /// # Arguments
    ///
    /// * `anchor_id` - The unique ID of the anchor to look up
    ///
    /// # Returns
    ///
    /// * `Some(&Yaml)` - Reference to the stored node if found
    /// * `None` - If no node exists for this anchor ID
    ///
    /// This method works for all node types (scalar, sequence, mapping)
    /// and is used when resolving aliases in the YAML document.
    #[must_use]
    pub fn get_anchor_node(&self, anchor_id: AnchorId) -> Option<&Yaml> {
        self.anchor_nodes.get(&anchor_id)
    }

    /// Retrieve an anchor node as a Yaml value
    ///
    /// This method is a bridge between the position tracking system and
    /// YAML document processing. It retrieves the node associated with
    /// a given anchor ID and returns it as a Yaml value.
    ///
    /// # Arguments
    ///
    /// * `anchor_id` - The unique ID of the anchor to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(Yaml)` - The Yaml node associated with the anchor
    /// * `None` - If no node exists for this anchor ID
    ///
    /// This method is particularly useful when integrating with code that
    /// uses the anchor_map directly, as it provides a compatible interface.
    #[must_use]
    pub fn get_anchor_yaml(&self, anchor_id: AnchorId) -> Option<Yaml> {
        self.get_anchor_node(anchor_id).cloned()
    }

    /// Get a node by its ID from the all_nodes map
    ///
    /// This method retrieves a node from the all_nodes map by its ID.
    /// It's used to ensure consistent node instances throughout the parsing process.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&Yaml)` - Reference to the stored node if found
    /// * `None` - If no node exists for this ID
    #[must_use]
    pub fn get_node(&self, node_id: NodeId) -> Option<&Yaml> {
        self.all_nodes.get(&node_id)
    }

    /// Get a node by its ID from the all_nodes map as a Yaml value
    ///
    /// This method retrieves a node from the all_nodes map by its ID and returns it as a Yaml value.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(Yaml)` - The Yaml node if found
    /// * `None` - If no node exists for this ID
    #[must_use]
    pub fn get_node_yaml(&self, node_id: NodeId) -> Option<Yaml> {
        self.get_node(node_id).cloned()
    }

    /// Find a node ID by its content
    ///
    /// This method finds a node ID by its content using content equality.
    /// It's used to ensure consistent node instances throughout the parsing process.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to find
    ///
    /// # Returns
    ///
    /// * `Some(usize)` - The ID of the node if found
    /// * `None` - If no node with this content exists
    #[must_use]
    pub fn find_node_id(&self, node: &Yaml) -> Option<NodeId> {
        // Calculate a hash for the node's content
        let hash = Self::calculate_node_hash(node);

        // Check if we have a node with this hash
        if let Some(&node_id) = self.node_content_hash_map.get(&hash) {
            // If we do, check if the content is actually equal
            // This is to handle hash collisions
            if let Some(stored_node) = self.all_nodes.get(&node_id) {
                if stored_node == node {
                    // We found the node, return its ID
                    return Some(node_id);
                }
            }
        }

        // If we didn't find the node, check all nodes
        // This is a fallback for cases where the hash doesn't match
        for (&id, stored_node) in &self.all_nodes {
            if stored_node == node {
                return Some(id);
            }
        }

        None
    }

    /// Track a node's position regardless of whether it has an anchor
    ///
    /// This method stores the position of any node, providing more comprehensive
    /// position tracking beyond just anchors.
    ///
    /// # Returns
    ///
    /// A unique ID for the tracked node
    pub fn track_node_position(&mut self, position: Marker) -> NodeId {
        let node_id = self.next_node_id;
        self.next_node_id = NodeId::new(self.next_node_id.value() + 1);
        // Create a PositionSpan with just the start position
        self.node_positions
            .insert(node_id, PositionSpan::new(position));
        node_id
    }

    /// Get the position of a node by its ID
    ///
    /// # Returns
    ///
    /// The position span of the node, or None if not found
    #[must_use]
    pub fn get_node_position(&self, node_id: NodeId) -> Option<PositionSpan> {
        self.node_positions.get(&node_id).copied()
    }

    /// Get all non-anchor node positions
    ///
    /// # Returns
    ///
    /// An iterator over (node_id, position_span) pairs
    #[must_use]
    pub fn get_all_node_positions(&self) -> impl Iterator<Item = (NodeId, PositionSpan)> + '_ {
        self.node_positions.iter().map(|(&id, &pos)| (id, pos))
    }

    /// Determines if a node is a flow sequence based on characteristics
    /// Flow sequences are denoted by surrounding `[` and `]` characters
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to check
    ///
    /// # Returns
    ///
    /// `true` if the node is determined to be a flow sequence, `false` otherwise
    pub fn is_flow_sequence(&self, node_id: NodeId) -> bool {
        let id = node_id;

        // First check if we're tracking this node directly
        if let Some(node) = self.all_nodes.get(&id) {
            // Flow sequences must be arrays
            if let Yaml::Array(items) = node {
                // Check position characteristics
                if let Some(position) = self.node_positions.get(&id) {
                    // Primary characteristic: start column is not at beginning of line (typically)
                    // Flow sequences rarely start at column 0 in typical YAML
                    if position.start.col() > 0 {
                        return true;
                    }

                    // If we have an end position, we can make a better determination
                    if let Some(end) = position.end {
                        // If the array is small (few items) and the span is narrow,
                        // it's likely a flow sequence
                        let is_small_array = items.len() <= 5;
                        let same_line = end.line() == position.start.line();
                        let narrow_span = end.col() - position.start.col() < 60;

                        if is_small_array && (same_line || narrow_span) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Determines if a node is a flow mapping based on characteristics
    /// Flow mappings are denoted by surrounding `{` and `}` characters
    /// They can span multiple lines but typically maintain a more compact structure
    /// than block mappings.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to check
    ///
    /// # Returns
    ///
    /// `true` if the node is determined to be a flow mapping, `false` otherwise
    pub fn is_flow_mapping(&self, node_id: NodeId) -> bool {
        let id = node_id;

        // First check if we're tracking this node directly
        if let Some(node) = self.all_nodes.get(&id) {
            // Flow mappings must be hash maps
            if let Yaml::Hash(items) = node {
                // Check position characteristics
                if let Some(position) = self.node_positions.get(&id) {
                    // Primary characteristic: start column is not at beginning of line (typically)
                    // Flow mappings rarely start at column 0 in typical YAML
                    if position.start.col() > 0 {
                        return true;
                    }

                    // If we have an end position, we can make a better determination
                    if let Some(end) = position.end {
                        // Several heuristics that suggest flow mapping:

                        // 1. Small mapping (few key-value pairs)
                        let is_small_mapping = items.len() <= 5;

                        // 2. Position span characteristics
                        let line_span = end.line() - position.start.line();

                        // Small mappings that span only a few lines are likely flow mappings
                        // (Block mappings typically span many more lines due to indentation)
                        if is_small_mapping && line_span <= 5 {
                            return true;
                        }

                        // 3. Compact structure (less vertical space than block mappings)
                        // If the mapping has multiple items but spans fewer lines than items,
                        // it's likely using flow style with multiple items per line
                        if items.len() > 2 && line_span + 1 < items.len() {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Track a node by its content and position
    ///
    /// This method associates a content hash with a node position,
    /// enabling more reliable position lookups for non-anchored nodes.
    ///
    /// # Arguments
    ///
    /// * `content_hash` - A hash of the node's content
    /// * `position` - The position of the node
    /// * `node` - Optional node to store. If provided, ensures the node is stored in all_nodes
    ///
    /// # Returns
    ///
    /// A unique ID for the tracked node
    pub fn track_node_with_hash(
        &mut self,
        content_hash: u64,
        position: Marker,
        node: Option<Yaml>,
    ) -> NodeId {
        let node_id = self.track_node_position(position);
        self.node_content_hash_map.insert(content_hash, node_id);

        // For scalar nodes, set an end position immediately
        // This ensures that all scalar nodes have both start and end positions
        if let Some(ref n) = node {
            // Check if the node is a scalar type
            let is_scalar = match n {
                Yaml::String(_)
                | Yaml::Integer(_)
                | Yaml::Real(_)
                | Yaml::Boolean(_)
                | Yaml::Null => true,
                _ => false,
            };

            if is_scalar {
                // For scalar nodes, estimate the end position based on content length
                let content_len = match n {
                    Yaml::String(s) => s.len(),
                    Yaml::Integer(_) => 5, // Reasonable estimate for most integers
                    Yaml::Real(_) => 8,    // Reasonable estimate for most floats
                    Yaml::Boolean(_) => 5, // 'true' or 'false'
                    Yaml::Null => 4,       // 'null'
                    _ => 0,
                };

                // Create an end position that's the same line but a few columns over
                let end_position = Marker::new(
                    position.index,
                    position.line,
                    position.col + content_len.min(100), // Cap at 100 to avoid overflow
                );

                // Set the end position for this node
                if let Some(span) = self.node_positions.get_mut(&node_id) {
                    span.set_end(end_position);
                }
            }
        }

        // If a node was provided, store it in all_nodes
        if let Some(n) = node {
            self.all_nodes.insert(node_id, n);
        }

        // If this is a flow-style mapping or sequence, we need to track it specially
        // to ensure we capture the opening and closing braces/brackets
        if let Some(current_path) = self.get_current_path() {
            if current_path == "root" {
                // We're at the root, so this might be a top-level flow mapping or sequence
                if let Some(key) = &self.current_key {
                    // This is a flow mapping or sequence under a key at the root level
                    // For example: "root: { key1: value1 }"
                    let path = format!("{}", key);
                    self.node_path_map.insert(path, node_id);
                }
            } else if let Some(key) = &self.current_key {
                // This is a flow mapping or sequence under a key in a nested structure
                // For example: "nested: { level1: { inner1: val1 } }"
                let path = format!("{}.{}", current_path, key);
                self.node_path_map.insert(path, node_id);
            }
        }

        node_id
    }

    /// Get the position of a node by its content hash
    ///
    /// # Arguments
    ///
    /// * `content_hash` - A hash of the node's content
    ///
    /// # Returns
    ///
    /// The position span of the node, or None if not found
    #[must_use]
    pub fn get_position_by_hash(&self, content_hash: u64) -> Option<PositionSpan> {
        self.node_content_hash_map
            .get(&content_hash)
            .and_then(|node_id| self.get_node_position(*node_id))
    }

    /// Get a node by its content hash
    ///
    /// This method retrieves a node from the all_nodes map by its content hash.
    /// It's used to ensure consistent node instances throughout the parsing process.
    ///
    /// # Arguments
    ///
    /// * `content_hash` - The hash of the node's content
    ///
    /// # Returns
    ///
    /// * `Some(Yaml)` - The Yaml node if found
    /// * `None` - If no node with this content hash exists
    #[must_use]
    pub fn get_node_by_hash(&self, content_hash: u64) -> Option<Yaml> {
        self.node_content_hash_map
            .get(&content_hash)
            .and_then(|node_id| self.get_node_yaml(*node_id))
    }

    /// Get the position of a node by its path
    ///
    /// This method looks up a node's position using its path in the document.
    /// This is particularly useful for retrieving positions of flow-style mappings
    /// and sequences that are identified by their path in the document structure.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the node (e.g., "root.key1")
    ///
    /// # Returns
    ///
    /// The position span of the node, or None if not found
    #[must_use]
    pub fn get_node_position_by_path(&self, path: &str) -> Option<PositionSpan> {
        // First try to find the node ID directly by path
        if let Some(&node_id) = self.node_path_map.get(path) {
            return self.get_node_position(node_id);
        }
        
        // If we didn't find it directly, try as a value path
        let value_path = format!("{}:value", path);
        if let Some(&node_id) = self.value_path_map.get(&value_path) {
            return self.get_node_position(node_id);
        }
        
        // Try looking up by mapping path for flow-style mappings
        // First check if this is a key with an associated mapping
        let key = path.split('.').last().unwrap_or(path);
        
        // Look for flow mapping at this key's position
        for (map_path, node_id) in self.node_path_map.iter() {
            // Check if this is a mapping_XX path
            if map_path.starts_with("mapping_") {
                if let Some(pos) = self.get_node_position(*node_id) {
                    if let Some(style) = self.get_mapping_style(*node_id) {
                        if style == TMappingStyle::Flow {
                            // Find the key node for this mapping
                            for (key_path, &key_id) in &self.node_path_map {
                                if *key_path == key {
                                    // Check if the key's position is right before the mapping
                                    if let Some(key_pos) = self.get_node_position(key_id) {
                                        // Key should be at same line, earlier column
                                        if key_pos.start.line() == pos.start.line() {
                                            if key_pos.start.col() < pos.start.col() {
                                                // This is likely the flow mapping right after this key
                                                println!("Found flow mapping at line {}, col {} for key '{}'", 
                                                       pos.start.line(), pos.start.col(), key);
                                                return Some(pos);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Try more aggressively - search for any flow mapping following a key with this name
        // This is especially important in early versions of the scanner
        for (key_path, &key_id) in &self.node_path_map {
            if *key_path == key {
                if let Some(key_pos) = self.get_node_position(key_id) {
                    // Now look for a mapping right after this key
                    for (map_path, node_id) in self.node_path_map.iter() {
                        if map_path.starts_with("mapping_") {
                            if let Some(style) = self.get_mapping_style(*node_id) {
                                if style == TMappingStyle::Flow {
                                    if let Some(map_pos) = self.get_node_position(*node_id) {
                                        // Mapping should be at same line, within a few columns of the key
                                        if map_pos.start.line() == key_pos.start.line() {
                                            let col_diff = map_pos.start.col() - key_pos.start.col();
                                            if col_diff > 0 && col_diff < 20 {  // Within reasonable distance
                                                println!("Found flow mapping by position at line {}, col {} for key '{}'", 
                                                       map_pos.start.line(), map_pos.start.col(), key);
                                                return Some(map_pos);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // General fallback approach - look for any flow mapping on the requested line
        // This handles cases where we're looking for a flow mapping by a key name,
        // but we don't have enough context to match it directly
        for (map_path, node_id) in self.node_path_map.iter() {
            if map_path.starts_with("mapping_") {
                if let Some(style) = self.get_mapping_style(*node_id) {
                    if style == TMappingStyle::Flow {
                        if let Some(pos) = self.get_node_position(*node_id) {
                            // Check if any part of the path is or contains the key
                            if key.contains(key) || map_path.contains(key) {
                                println!("Found flow mapping by name similarity at line {}, col {}", 
                                       pos.start.line(), pos.start.col());
                                return Some(pos);
                            }
                            
                            // Also check value paths
                            for (value_path, &value_node_id) in &self.value_path_map {
                                if value_path.contains(key) && value_node_id == *node_id {
                                    println!("Found flow mapping through value path match at line {}, col {}", 
                                           pos.start.line(), pos.start.col());
                                    return Some(pos);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        None
    }

    /// Calculate a simple hash for a YAML node
    ///
    /// This method generates a hash for a node based on its content.
    /// Note: This is a basic implementation and could be improved for better
    /// collision avoidance, especially for complex structures.
    ///
    /// # Arguments
    ///
    /// * `node` - The YAML node to hash
    ///
    /// # Returns
    ///
    /// A content hash that can be used to identify similar nodes
    #[must_use]
    pub fn calculate_node_hash(node: &Yaml) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        match node {
            Yaml::String(s) => {
                "string".hash(&mut hasher);
                s.hash(&mut hasher);
            }
            Yaml::Integer(i) => {
                "integer".hash(&mut hasher);
                i.hash(&mut hasher);
            }
            Yaml::Real(r) => {
                "real".hash(&mut hasher);
                r.hash(&mut hasher);
            }
            Yaml::Boolean(b) => {
                "boolean".hash(&mut hasher);
                b.hash(&mut hasher);
            }
            Yaml::Array(a) => {
                "array".hash(&mut hasher);
                a.len().hash(&mut hasher);
                if !a.is_empty() {
                    // Add the type of the first element for better differentiation
                    match &a[0] {
                        Yaml::String(_) => "string_array".hash(&mut hasher),
                        Yaml::Integer(_) => "int_array".hash(&mut hasher),
                        Yaml::Hash(_) => "hash_array".hash(&mut hasher),
                        _ => "mixed_array".hash(&mut hasher),
                    }
                }
            }
            Yaml::Hash(h) => {
                "hash".hash(&mut hasher);
                h.len().hash(&mut hasher);

                // Try to hash some key names for better identification
                let mut keys = h.keys().collect::<Vec<_>>();
                keys.sort_by(|a, b| {
                    if let (Yaml::String(a_str), Yaml::String(b_str)) = (a, b) {
                        a_str.cmp(b_str)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                });

                // Hash up to 3 keys for identification
                for key in keys.iter().take(3) {
                    if let Yaml::String(key_str) = key {
                        key_str.hash(&mut hasher);
                    }
                }
            }
            Yaml::Alias(id) => {
                "alias".hash(&mut hasher);
                id.hash(&mut hasher);
            }
            Yaml::BadValue => {
                "badvalue".hash(&mut hasher);
            }
            Yaml::Null => {
                "null".hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Convert a scalar value to the appropriate Yaml type based on style and tag
    ///
    /// This helper method determines the appropriate Yaml type for a scalar value
    /// considering the style, tag, and content of the scalar.
    fn convert_scalar_value(value: &str, style: &TScalarStyle, tag: &Option<Tag>) -> Yaml {
        if *style != TScalarStyle::Plain {
            // Non-plain scalars are always stored as strings
            return Yaml::String(String::from(value));
        }

        // Check if there's a tag
        if let Some(Tag {
            ref handle,
            ref suffix,
        }) = tag
        {
            if handle == "tag:yaml.org,2002:" {
                match suffix.as_ref() {
                    "bool" => {
                        if let Ok(v) = value.parse::<bool>() {
                            return Yaml::Boolean(v);
                        } else {
                            return Yaml::BadValue;
                        }
                    }
                    "int" => {
                        if let Ok(v) = value.parse::<i64>() {
                            return Yaml::Integer(v);
                        } else {
                            return Yaml::BadValue;
                        }
                    }
                    "float" => return Yaml::Real(String::from(value)),
                    "null" => {
                        if value == "~" || value == "null" {
                            return Yaml::Null;
                        } else {
                            return Yaml::BadValue;
                        }
                    }
                    _ => return Yaml::String(String::from(value)),
                }
            } else {
                return Yaml::String(String::from(value));
            }
        }

        // Try to convert the value based on its content (plain scalar)
        if value == "~" || value == "null" {
            Yaml::Null
        } else if value == "true" {
            Yaml::Boolean(true)
        } else if value == "false" {
            Yaml::Boolean(false)
        } else if let Ok(i) = value.parse::<i64>() {
            Yaml::Integer(i)
        } else if value == ".inf"
            || value == ".Inf"
            || value == ".INF"
            || value == "+.inf"
            || value == "+.Inf"
            || value == "+.INF"
            || value == "-.inf"
            || value == "-.Inf"
            || value == "-.INF"
            || value == ".nan"
            || value == "NaN"
            || value == ".NAN"
            || value.parse::<f64>().is_ok()
        {
            Yaml::Real(String::from(value))
        } else {
            Yaml::String(String::from(value))
        }
    }

    /// Process a YAML event and track its position
    ///
    /// This method has been enhanced to track positions for all nodes,
    /// not just those with anchors, and to properly differentiate between
    /// keys and values in the YAML structure.
    ///
    /// # Arguments
    ///
    /// * `event` - The YAML event to process
    /// * `mark` - The position marker for the event
    ///
    /// # Returns
    ///
    /// A position span for the event
    pub fn process_event(&mut self, ev: &Event, mark: Marker) -> PositionSpan {
        let mut span = PositionSpan {
            start: mark,
            end: None,
        };
        
        match ev {
            Event::MappingStart(anchor_id, _, style) => {
                let node_id = self.next_node_id();
                self.store_mapping_style(node_id, *style);
                self.push(node_id, mark);
                
                // Push the context type to track that we're in a mapping
                self.context_stack.push(ContextType::Mapping(*style));

                // If this is also an anchor, track it
                if *anchor_id > AnchorId::new(0) {
                    self.track_anchor(*anchor_id, mark);
                    // For now, we can't store the node content as we don't have the full mapping yet
                    // We'll store an empty mapping that will be populated later
                    let empty_map = Yaml::Hash(crate::yaml::Hash::new());
                    self.store_anchor_node(*anchor_id, empty_map);
                }

                // Track this node position regardless of anchor
                // We also track the content hash for empty maps
                let empty_map = Yaml::Hash(crate::yaml::Hash::new());
                let hash = Self::calculate_node_hash(&empty_map);
                let node_id = self.track_node_with_hash(hash, mark, Some(empty_map));

                // Store the node ID in the node path map for easier lookup
                // This is especially important for block-style mappings
                let path = format!("mapping_{}", node_id);
                self.node_path_map.insert(path.clone(), node_id);

                // Special handling for flow-style mappings
                if *style == TMappingStyle::Flow {
                    // For flow-style mappings, we need to ensure the position is correctly tracked
                    if let Some(key) = &self.current_key {
                        // This is a flow-style mapping under a key
                        // Debug statement for visibility
                        println!("Processing flow mapping for key '{}' at line {}, col {}", 
                                key, mark.line(), mark.col());
                        
                        // Store in the value path map to distinguish it from the key
                        let value_path = format!("{}:value", key);
                        self.value_path_map.insert(value_path, node_id);
                        
                        // Also store in the regular path map for backward compatibility
                        self.node_path_map.insert(key.clone(), node_id);

                        // If we have a parent node, establish the key-value relationship
                        if let Some(&parent_id) = self.parent_stack.last() {
                            // Find the key node ID
                            if let Some(key_node_id) = self.find_node_id_by_path(key) {
                                // Store the relationship
                                self.key_value_map.insert(key_node_id, node_id);
                                
                                println!("Established key-value relationship: {} -> {}", key_node_id, node_id);
                            }
                        }

                        // Also store with the current path if available
                        if let Some(current_path) = self.get_current_path() {
                            let full_path = format!("{}.{}", current_path, key);
                            self.node_path_map.insert(full_path.clone(), node_id);
                            
                            // Also store the value path for this full path
                            let full_value_path = format!("{}:value", full_path);
                            self.value_path_map.insert(full_value_path, node_id);
                            
                            println!("Mapped flow mapping to path: {} at line {}, col {}", 
                                    full_path, mark.line(), mark.col());
                        }

                        // Push the key onto the path stack for nested elements
                        self.push_path(key.clone());
                    } else if let Some(current_path) = self.get_current_path() {
                        // We're at the root or in a sequence with a flow-style mapping
                        self.node_path_map.insert(current_path.clone(), node_id);
                        
                        println!("Mapped root flow mapping to path: {} at line {}, col {}", 
                                current_path, mark.line(), mark.col());
                    }
                } else {
                    // Standard handling for block-style mappings
                    if let Some(current_path) = self.get_current_path() {
                        // For mappings in the root or as values in other mappings
                        if let Some(key) = &self.current_key {
                            // We're inside a mapping and this is a nested mapping under a key
                            let full_path = format!("{}.{}", current_path, key);
                            self.node_path_map.insert(full_path.clone(), node_id);
                            
                            // Also store as a value specifically
                            let value_path = format!("{}:value", key);
                            self.value_path_map.insert(value_path, node_id);
                            
                            println!("Mapped block mapping to path: {} at line {}, col {}", 
                                    full_path, mark.line(), mark.col());
                            
                            // Push the key onto the path stack for nested elements
                            self.push_path(key.clone());
                        } else {
                            // We're at the root or in a sequence
                            self.node_path_map.insert(current_path.clone(), node_id);
                        }
                    }
                }

                // Push this node ID onto the parent stack
                self.parent_stack.push(node_id);

                // Clear the current key as we've processed it
                self.clear_current_key();
                
                // Reset processing_value flag
                self.processing_value = false;

                span
            }
            Event::MappingEnd => {
                // For mappings, if we have a start position on the stack,
                // create a position span with both start and end positions
                if let Some((id, start_mark)) = self.pop() {
                    let end_mark = self.calculate_end_position(id, mark);
                    // Create a complete span with start and end positions
                    let complete_span = PositionSpan::with_end(start_mark, end_mark);

                    // Update the position span for this node ID
                    if let Some(pos) = self.node_positions.get_mut(&id) {
                        pos.end = Some(end_mark);
                        
                        // Print debug information
                        println!("Updated end position for hash node {} to ({},{})",
                            id, end_mark.line(), end_mark.col());
                    }

                    // Pop from the context stack
                    if let Some(ContextType::Mapping(_)) = self.context_stack.last() {
                        self.context_stack.pop();
                    }

                    // Pop from the parent stack
                    if !self.parent_stack.is_empty() {
                        self.parent_stack.pop();
                    }

                    // Pop the path component as we're exiting the mapping
                    self.pop_path();

                    complete_span
                } else {
                    // Otherwise, just return the current position
                    span
                }
            }
            Event::SequenceStart(anchor_id, _tag, style) => {
                let node_id = self.next_node_id();
                self.store_sequence_style(node_id, *style);
                self.push(node_id, mark);
                
                // Push the context type to track that we're in a sequence
                self.context_stack.push(ContextType::Sequence(*style));

                // If this is also an anchor, track it
                if *anchor_id > AnchorId::new(0) {
                    self.track_anchor(*anchor_id, mark);
                    // For now, we can't store the node content as we don't have the full sequence yet
                    // We'll store an empty sequence that will be populated later
                    let empty_seq = Yaml::Array(Vec::new());
                    self.store_anchor_node(*anchor_id, empty_seq);
                }

                // Track this node position regardless of anchor
                // We also track the content hash for empty sequences
                let empty_seq = Yaml::Array(Vec::new());
                let hash = Self::calculate_node_hash(&empty_seq);
                let node_id = self.track_node_with_hash(hash, mark, Some(empty_seq));

                // Store the node ID in the node path map for easier lookup
                // This is especially important for block-style sequences
                let path = format!("sequence_{}", node_id);
                self.node_path_map.insert(path.clone(), node_id);

                // If we have a path on the stack and a current key, use them to track this sequence
                if let Some(current_path) = self.get_current_path() {
                    // For sequences in the root or as values in other mappings
                    if let Some(key) = &self.current_key {
                        // We're inside a mapping and this is a sequence under a key
                        let full_path = format!("{}.{}", current_path, key);
                        self.node_path_map.insert(full_path.clone(), node_id);
                        
                        // Also store as a value specifically
                        let value_path = format!("{}:value", key);
                        self.value_path_map.insert(value_path, node_id);
                        
                        println!("Mapped sequence to path: {} at line {}, col {}", 
                                full_path, mark.line(), mark.col());
                        
                        // Push the key onto the path stack for nested elements
                        self.push_path(key.clone());
                    } else {
                        // We're at the root or in another sequence
                        self.node_path_map.insert(current_path.clone(), node_id);
                    }
                }

                // Clear the current key as we've processed it
                self.clear_current_key();
                
                // Reset processing_value flag
                self.processing_value = false;

                // Push this node ID onto the parent stack
                self.parent_stack.push(node_id);

                span
            }
            Event::SequenceEnd => {
                // If the item on top of the stack is a sequence,
                // return a span from its start position to the current position
                if let Some((id, start_mark)) = self.pop() {
                    let end_mark = self.calculate_end_position(id, mark);
                    let complete_span = PositionSpan::with_end(start_mark, end_mark);

                    // Update the position span for this node ID
                    if let Some(pos) = self.node_positions.get_mut(&id) {
                        pos.end = Some(end_mark);
                        
                        // Print debug information
                        println!("Updated end position for array node {} to ({},{})",
                            id, end_mark.line(), end_mark.col());
                    }

                    // Pop from the context stack
                    if let Some(ContextType::Sequence(_)) = self.context_stack.last() {
                        self.context_stack.pop();
                    }

                    // Pop from the parent stack
                    if !self.parent_stack.is_empty() {
                        self.parent_stack.pop();
                    }

                    // Pop the path component as we're exiting the sequence
                    self.pop_path();

                    complete_span
                } else {
                    // Otherwise, just return the current position
                    span
                }
            }
            Event::Scalar(value, style, anchor_id, tag) => {
                // For scalar values, compute end position based on content length
                // This is an approximation that works for simple cases
                let lines = value.split('\n').collect::<Vec<_>>();
                if lines.len() == 1 {
                    // Single line scalar - end is start + length
                    span.end = Some(Marker::new(
                        mark.index() + value.len(),
                        mark.line(),
                        mark.col() + value.len(),
                    ));
                } else {
                    // Multi-line scalar - need to compute based on last line
                    let last_line = lines.last().unwrap();
                    span.end = Some(Marker::new(
                        mark.index() + value.len(),
                        mark.line() + lines.len() - 1,
                        last_line.len(),
                    ));
                }

                // Convert the scalar value to the appropriate Yaml type
                let node = Self::convert_scalar_value(value, style, tag);

                // Generate a hash for this scalar node
                let hash = Self::calculate_node_hash(&node);

                // Track this node with its content hash
                let node_id = self.track_node_with_hash(hash, mark, Some(node.clone()));

                // If this is an anchor, track it
                if *anchor_id > AnchorId::new(0) {
                    // For scalars with anchors, track the anchor position
                    self.track_anchor(*anchor_id, mark);
                    // Store the converted node
                    self.store_anchor_node(*anchor_id, node);
                }

                // Determine if we're processing a key or a value based on context
                let is_key = if let Some(context) = self.context_stack.last() {
                    match context {
                        ContextType::Mapping(_) => {
                            // In a mapping, every other scalar is a key
                            self.current_key.is_none() && !self.processing_value
                        }
                        _ => false, // In sequences or at document root, no keys
                    }
                } else {
                    false
                };

                // Debug info for tracking key/value pairs
                println!("Processing scalar '{}' at line {}, col {} (is_key={})", 
                        value, mark.line(), mark.col(), is_key);

                // Check if this is a key in a mapping
                if let Some(current_path) = self.get_current_path() {
                    if is_key {
                        // This is a key - store it for the next scalar (which will be the value)
                        self.set_current_key(value.clone());
                        self.processing_value = true;

                        // Track the key's position
                        let key_path = format!("{}.{}", current_path, value);
                        self.node_path_map.insert(key_path.clone(), node_id);
                        
                        println!("Registered key '{}' at path: {}", value, key_path);
                    } else {
                        // This is a value for a previously seen key or a scalar in a sequence
                        if let Some(key) = &self.current_key {
                            // This is a value for a specific key
                            let value_path = format!("{}:value", key);
                            self.value_path_map.insert(value_path.clone(), node_id);
                            
                            println!("Registered scalar value '{}' for key '{}' at path: {}", 
                                    value, key, value_path);
                            
                            // Also add to the traditional path map for compatibility
                            let full_path = format!("{}.{}", current_path, key);
                            self.node_path_map.insert(full_path.clone(), node_id);
                            
                            // Add the value path for the full path too
                            let full_value_path = format!("{}:value", full_path);
                            self.value_path_map.insert(full_value_path, node_id);
                            
                            // Store the key-value relationship
                            if let Some(key_node_id) = self.find_node_id_by_path(key) {
                                self.key_value_map.insert(key_node_id, node_id);
                                println!("Linked key-value relationship: {} -> {}", key_node_id, node_id);
                            }
                            
                            // Reset processing flags
                            self.processing_value = false;
                            self.clear_current_key();
                        } else {
                            // This is a scalar in a sequence or at the root
                            self.node_path_map.insert(current_path.clone(), node_id);
                        }
                    }
                } else if self.path_stack.is_empty() {
                    // We're at the root and this is a scalar
                    self.node_path_map.insert("root".to_string(), node_id);
                }

                span
            }
            Event::Alias(_anchor_id) => {
                // For aliases, we'll create a span with just the current position
                // We could also look up the anchor position if needed
                let span = PositionSpan::new(mark);
                // We don't need to handle the node resolution here, as that's done
                // in the loader. We just need to track the position.
                // Track this node position regardless of anchor
                self.track_node_position(mark);
                span
            }
            _ => {
                // For all other events, just return a span with the current position
                span
            }
        }
    }

    /// Find an anchor ID for a given node
    ///
    /// This performs a reverse lookup, finding the anchor ID
    /// associated with a specific node.
    ///
    /// # Arguments
    ///
    /// * `node` - The YAML node to look up
    ///
    /// # Returns
    ///
    /// * `Some(usize)` - The anchor ID if found
    /// * `None` - If no anchor is associated with this node
    #[must_use]
    pub fn find_anchor_id(&self, node: &Yaml) -> Option<AnchorId> {
        // First try direct equality
        for (id, anchor_node) in &self.anchor_nodes {
            if anchor_node == node {
                return Some(*id);
            }
        }

        // If that fails, try content-based equality using the hash
        let node_hash = Self::calculate_node_hash(node);
        for (id, anchor_node) in &self.anchor_nodes {
            let anchor_hash = Self::calculate_node_hash(anchor_node);
            if anchor_hash == node_hash {
                return Some(*id);
            }
        }

        None
    }

    pub fn next_node_id(&mut self) -> NodeId {
        self.next_node_id = NodeId::new(self.next_node_id.value() + 1);
        self.next_node_id
    }

    /// Track a node by its path in the YAML document
    ///
    /// This method stores the position of a node identified by its path,
    /// which provides a way to track non-anchored nodes.
    ///
    /// # Arguments
    ///
    /// * `path` - A string representing the path to the node (e.g., "root.key1.key2")
    /// * `position` - The marker position of the node
    ///
    /// # Returns
    ///
    /// A unique ID for the tracked node
    pub fn track_node_with_path(&mut self, path: &str, position: Marker) -> NodeId {
        let node_id = match self.node_path_map.get(path) {
            Some(&id) => id,
            None => {
                let id = self.next_node_id();
                self.node_path_map.insert(path.to_owned(), id);
                id
            }
        };

        // Create a PositionSpan with just the start position
        self.node_positions
            .insert(node_id, PositionSpan::new(position));
        node_id
    }

    /// Get a node position by its path
    ///
    /// # Arguments
    ///
    /// * `path` - A string representing the path to the node (e.g., "root.key1.key2")
    ///
    /// # Returns
    ///
    /// The position span of the node if found
    #[must_use]
    pub fn get_position_by_path(&self, path: &str) -> Option<PositionSpan> {
        self.node_path_map
            .get(path)
            .and_then(|id| self.node_positions.get(id))
            .copied()
    }

    /// Get all node paths that have been tracked
    ///
    /// # Returns
    ///
    /// An iterator over all node paths and their IDs
    #[must_use]
    pub fn get_all_node_paths(&self) -> impl Iterator<Item = (&String, &NodeId)> {
        self.node_path_map.iter()
    }

    /// Get all node positions by path
    ///
    /// # Returns
    ///
    /// An iterator over all node paths and their positions
    #[must_use]
    pub fn get_all_node_positions_by_path(
        &self,
    ) -> impl Iterator<Item = (&String, PositionSpan)> + '_ {
        self.node_path_map.iter().filter_map(|(path, id)| {
            self.node_positions
                .get(id)
                .map(|&position| (path, position))
        })
    }

    /// Register a complete span with both start and end positions
    ///
    /// This is a convenience method to set both the start and end position in one call
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to update
    /// * `start` - The start position marker
    /// * `end` - The end position marker
    pub fn track_span_with_end(&mut self, node_id: NodeId, start: Marker, end: Marker) {
        // Create a complete span with start and end positions
        let complete_span = PositionSpan::with_end(start, end);

        // Update the node's position in the node_positions map
        self.node_positions.insert(node_id, complete_span);

        // Collect paths that need updating to avoid borrowing issues
        let paths_to_update: Vec<String> = self
            .node_path_map
            .iter()
            .filter_map(|(path, id)| {
                if id == &node_id {
                    Some(path.clone())
                } else {
                    None
                }
            })
            .collect();

        // Now update each path separately
        for path in paths_to_update {
            // Update positions for each path that maps to this node
            if let Some(id) = self.node_path_map.get(&path).copied() {
                if id == node_id && self.node_positions.contains_key(&id) {
                    // Use track_node_with_path first to ensure the node is properly registered
                    self.track_node_with_path(&path, start);

                    // Now directly update the end position
                    if let Some(pos) = self.node_positions.get_mut(&id) {
                        pos.end = Some(end);
                    }
                }
            }
        }
    }

    /// Store the style of a node
    ///
    /// This method stores the style information for a node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node
    /// * `style` - The style of the node
    pub fn store_node_style(&mut self, node_id: NodeId, style: NodeStyle) {
        self.node_styles.insert(node_id, style);
    }

    /// Get the style of a node
    ///
    /// This method retrieves the style information for a node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node
    ///
    /// # Returns
    ///
    /// The style of the node, if available
    #[must_use]
    pub fn get_node_style(&self, node_id: NodeId) -> Option<NodeStyle> {
        self.node_styles.get(&node_id).copied()
    }

    /// Store the style of a sequence node
    ///
    /// This method stores the style information for a sequence node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node
    /// * `style` - The style of the sequence
    pub fn store_sequence_style(&mut self, node_id: NodeId, style: TSequenceStyle) {
        self.node_styles.insert(node_id, NodeStyle::Sequence(style));
    }

    /// Store the style of a mapping node
    ///
    /// This method stores the style information for a mapping node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node
    /// * `style` - The style of the mapping
    pub fn store_mapping_style(&mut self, node_id: NodeId, style: TMappingStyle) {
        self.node_styles.insert(node_id, NodeStyle::Mapping(style));
    }

    /// Get the style of a sequence node
    ///
    /// This method retrieves the style information for a sequence node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node
    ///
    /// # Returns
    ///
    /// The style of the sequence, if available
    #[must_use]
    pub fn get_sequence_style(&self, node_id: NodeId) -> Option<TSequenceStyle> {
        match self.node_styles.get(&node_id) {
            Some(NodeStyle::Sequence(style)) => Some(*style),
            _ => None,
        }
    }

    /// Get the style of a mapping node
    ///
    /// This method retrieves the style information for a mapping node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node
    ///
    /// # Returns
    ///
    /// The style of the mapping, if available
    #[must_use]
    pub fn get_mapping_style(&self, node_id: NodeId) -> Option<TMappingStyle> {
        match self.node_styles.get(&node_id) {
            Some(NodeStyle::Mapping(style)) => Some(*style),
            _ => None,
        }
    }

    fn calculate_end_position(&self, node_id: NodeId, mark: Marker) -> Marker {
        // Use style information to determine end position
        if let Some(style) = self.node_styles.get(&node_id) {
            match style {
                NodeStyle::Sequence(TSequenceStyle::Flow)
                | NodeStyle::Mapping(TMappingStyle::Flow) => {
                    // For flow style, use the closing bracket/brace position
                    mark
                }
                _ => {
                    // For block style, use the end of the last entry
                    if let Some(last_pos) = self.position_stack.last().map(|entry| entry.1) {
                        // Default to the last position on the stack if available
                        last_pos
                    } else {
                        // Fallback to the provided mark
                        mark
                    }
                }
            }
        } else {
            // Default behavior if style is not available
            mark
        }
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}

// Additional helper methods for debugging and testing
impl PositionTracker {
    /// Get all path to node ID mappings
    ///
    /// This method returns all path to node ID mappings in the position tracker.
    /// It's useful for debugging and testing.
    ///
    /// # Returns
    ///
    /// A vector of tuples containing the path and node ID
    #[must_use]
    pub fn get_path_mappings(&self) -> Vec<(String, NodeId)> {
        self.node_path_map
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Find a node ID by path
    ///
    /// This method finds a node ID by its path in the YAML document.
    /// It's a convenience wrapper around the find_node_id_by_path method.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the node in the YAML document
    ///
    /// # Returns
    ///
    /// * `Some(usize)` - The node ID if found
    /// * `None` - If no node ID is associated with this path
    #[must_use]
    pub fn find_node_id_by_path_str(&self, path: &str) -> Option<NodeId> {
        self.node_path_map.get(path).copied()
    }

    /// Set a position for a node by its ID
    ///
    /// This method sets a position for a node by its ID.
    /// It's useful for ensuring a node has a position in the source map.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node
    /// * `position` - The position to set
    pub fn set_node_position(&mut self, node_id: NodeId, position: Marker) {
        let span = PositionSpan {
            start: position,
            end: Some(position),
        };
        self.node_positions.insert(node_id, span);
    }
}
