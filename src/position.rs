//! Position tracking for YAML constructs.
//!
//! This module provides structures for tracking the positions of YAML constructs
//! in the source document, including both start and end positions.

use crate::parser::Event;
use crate::parser::Tag;
use crate::scanner::{Marker, ScanError, TMappingStyle, TScalarStyle};
use crate::yaml::Yaml;

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
    position_stack: Vec<(usize, Marker)>,
    /// Map of anchor ID to position
    anchor_positions: std::collections::HashMap<usize, Marker>,
    /// Map of anchor ID to node content
    anchor_nodes: std::collections::HashMap<usize, Yaml>,
    /// Map of node pointers to positions (for all nodes, not just anchors)
    node_positions: std::collections::HashMap<usize, Marker>,
    /// Map of node content hash to node ID
    node_content_hash_map: std::collections::HashMap<u64, usize>,
    /// Map of node path to node ID for tracking non-anchored nodes
    node_path_map: std::collections::HashMap<String, usize>,
    /// Counter for generating unique node IDs
    next_node_id: usize,
}

impl PositionTracker {
    /// Create a new position tracker
    #[must_use]
    pub fn new() -> Self {
        PositionTracker {
            position_stack: Vec::new(),
            anchor_positions: std::collections::HashMap::new(),
            anchor_nodes: std::collections::HashMap::new(),
            node_positions: std::collections::HashMap::new(),
            node_content_hash_map: std::collections::HashMap::new(),
            node_path_map: std::collections::HashMap::new(),
            next_node_id: 1, // Start from 1
        }
    }

    /// Push a new position to the stack
    ///
    /// This is used when starting a new construct (such as a flow mapping or sequence)
    /// where we want to track the position of the opening delimiter.
    ///
    /// The `id` parameter is used to identify the construct type (e.g., 0 for flow mappings,
    /// 1 for flow sequences).
    pub fn push(&mut self, id: usize, position: Marker) {
        self.position_stack.push((id, position));
    }

    /// Pop a position from the stack
    ///
    /// This is used when ending a construct (such as a flow mapping or sequence)
    /// where we want to retrieve the position of the opening delimiter.
    ///
    /// Returns `None` if the stack is empty.
    #[must_use]
    pub fn pop(&mut self) -> Option<(usize, Marker)> {
        self.position_stack.pop()
    }

    /// Peek at the top position on the stack
    ///
    /// This is used to check the position of the most recently opened construct
    /// without removing it from the stack.
    ///
    /// Returns `None` if the stack is empty.
    #[must_use]
    pub fn peek(&self) -> Option<&(usize, Marker)> {
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
    pub fn track_anchor(&mut self, anchor_id: usize, position: Marker) {
        self.anchor_positions.insert(anchor_id, position);
    }

    /// Get the position of an anchor
    ///
    /// Returns the position where the anchor was declared
    #[must_use]
    pub fn get_anchor_position(&self, anchor_id: usize) -> Option<Marker> {
        self.anchor_positions.get(&anchor_id).copied()
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
    /// as it allows for complete node resolution when encountering aliases.
    pub fn store_anchor_node(&mut self, anchor_id: usize, node: Yaml) {
        self.anchor_nodes.insert(anchor_id, node);
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
    pub fn get_anchor_node(&self, anchor_id: usize) -> Option<&Yaml> {
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
    pub fn get_anchor_yaml(&self, anchor_id: usize) -> Option<Yaml> {
        self.get_anchor_node(anchor_id).cloned()
    }

    /// Track a node's position regardless of whether it has an anchor
    ///
    /// This method stores the position of any node, providing more comprehensive
    /// position tracking beyond just anchors.
    ///
    /// # Returns
    ///
    /// A unique ID for the tracked node
    pub fn track_node_position(&mut self, position: Marker) -> usize {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        self.node_positions.insert(node_id, position);
        node_id
    }

    /// Get the position of a node by its ID
    ///
    /// # Returns
    ///
    /// The position of the node, or None if not found
    #[must_use]
    pub fn get_node_position(&self, node_id: usize) -> Option<Marker> {
        self.node_positions.get(&node_id).copied()
    }

    /// Get all non-anchor node positions
    ///
    /// # Returns
    ///
    /// An iterator over (node_id, position) pairs
    #[must_use]
    pub fn get_all_node_positions(&self) -> impl Iterator<Item = (usize, Marker)> + '_ {
        self.node_positions.iter().map(|(&id, &pos)| (id, pos))
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
    ///
    /// # Returns
    ///
    /// A unique ID for the tracked node
    pub fn track_node_with_hash(&mut self, content_hash: u64, position: Marker) -> usize {
        let node_id = self.track_node_position(position);
        self.node_content_hash_map.insert(content_hash, node_id);
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
    /// The position of the node, or None if not found
    #[must_use]
    pub fn get_position_by_hash(&self, content_hash: u64) -> Option<Marker> {
        self.node_content_hash_map
            .get(&content_hash)
            .and_then(|node_id| self.get_node_position(*node_id))
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
    /// not just those with anchors.
    ///
    /// # Arguments
    ///
    /// * `event` - The YAML event to process
    /// * `mark` - The position marker for the event
    ///
    /// # Returns
    ///
    /// A position span for the event
    pub fn process_event(&mut self, event: &Event, mark: Marker) -> PositionSpan {
        match event {
            Event::MappingStart(anchor_id, _, style) if *style == TMappingStyle::Flow => {
                // For flow mappings, push the start position to the stack
                self.push(0, mark);

                // If this is also an anchor, track it
                if *anchor_id > 0 {
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
                self.track_node_with_hash(hash, mark);

                PositionSpan::new(mark)
            }
            Event::MappingEnd => {
                // For mappings, if we have a start position on the stack,
                // create a position span with both start and end positions
                if let Some((0, start_mark)) = self.pop() {
                    // If the item on top of the stack is a flow mapping (id 0),
                    // return a span from its start position to the current position
                    PositionSpan::with_end(start_mark, mark)
                } else {
                    // Otherwise, just return the current position
                    PositionSpan::new(mark)
                }
            }
            Event::SequenceStart(anchor_id, _) => {
                // For sequences, push the start position to the stack
                self.push(1, mark);

                // If this is also an anchor, track it
                if *anchor_id > 0 {
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
                self.track_node_with_hash(hash, mark);

                PositionSpan::new(mark)
            }
            Event::SequenceEnd => {
                // If the item on top of the stack is a sequence (id 1),
                // return a span from its start position to the current position
                if let Some((1, start_mark)) = self.pop() {
                    PositionSpan::with_end(start_mark, mark)
                } else {
                    // Otherwise, just return the current position
                    PositionSpan::new(mark)
                }
            }
            Event::Scalar(value, style, anchor_id, tag) => {
                // For scalars, create a span with just the current position
                let span = PositionSpan::new(mark);

                // Convert the scalar value to the appropriate Yaml type
                let node = Self::convert_scalar_value(value, style, tag);

                // Generate a hash for this scalar node
                let hash = Self::calculate_node_hash(&node);

                // Track this node with its content hash
                self.track_node_with_hash(hash, mark);

                // If this is an anchor, track it
                if *anchor_id > 0 {
                    // For scalars with anchors, track the anchor position
                    self.track_anchor(*anchor_id, mark);
                    // Store the converted node
                    self.store_anchor_node(*anchor_id, node);
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
                PositionSpan::new(mark)
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
    pub fn find_anchor_id(&self, node: &Yaml) -> Option<usize> {
        for (id, anchor_node) in &self.anchor_nodes {
            if anchor_node == node {
                return Some(*id);
            }
        }
        None
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
    pub fn track_node_with_path(&mut self, path: &str, position: Marker) -> usize {
        let node_id = match self.node_path_map.get(path) {
            Some(&id) => id,
            None => {
                let id = self.next_node_id;
                self.next_node_id += 1;
                self.node_path_map.insert(path.to_owned(), id);
                id
            }
        };

        self.node_positions.insert(node_id, position);
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
    /// The marker position of the node if found
    #[must_use]
    pub fn get_position_by_path(&self, path: &str) -> Option<Marker> {
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
    pub fn get_all_node_paths(&self) -> impl Iterator<Item = (&String, &usize)> {
        self.node_path_map.iter()
    }

    /// Get all node positions by path
    ///
    /// # Returns
    ///
    /// An iterator over all node paths and their positions
    #[must_use]
    pub fn get_all_node_positions_by_path(&self) -> impl Iterator<Item = (&String, Marker)> + '_ {
        self.node_path_map.iter().filter_map(|(path, id)| {
            self.node_positions
                .get(id)
                .map(|&position| (path, position))
        })
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}
