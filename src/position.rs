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
}

impl PositionTracker {
    /// Create a new position tracker
    #[must_use]
    pub fn new() -> Self {
        PositionTracker {
            position_stack: Vec::new(),
            anchor_positions: std::collections::HashMap::new(),
            anchor_nodes: std::collections::HashMap::new(),
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
    pub fn store_anchor_node(&mut self, anchor_id: usize, node: Yaml) {
        self.anchor_nodes.insert(anchor_id, node);
    }

    /// Get the node associated with an anchor
    ///
    /// Returns the node that was stored for the given anchor ID,
    /// or None if no node was stored.
    #[must_use]
    pub fn get_anchor_node(&self, anchor_id: usize) -> Option<&Yaml> {
        self.anchor_nodes.get(&anchor_id)
    }

    /// Process an event and update position tracking
    ///
    /// This method takes an event and its position, updates the position tracker's
    /// internal state, and returns a PositionSpan for the event.
    ///
    /// For flow collections, it builds a complete span from the opening to the closing
    /// delimiter. For other events, it just returns a span with the current position.
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
                    self.store_anchor_node(*anchor_id, Yaml::Hash(crate::yaml::Hash::new()));
                }

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
                    self.store_anchor_node(*anchor_id, Yaml::Array(Vec::new()));
                }

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
            Event::Alias(_anchor_id) => {
                // For aliases, we'll create a span with just the current position
                // We could also look up the anchor position if needed
                let span = PositionSpan::new(mark);

                // We don't need to handle the node resolution here, as that's done
                // in the loader. We just need to track the position.

                span
            }
            Event::Scalar(value, style, anchor_id, tag) if *anchor_id > 0 => {
                // For scalars with anchors, track the anchor position
                self.track_anchor(*anchor_id, mark);

                // Store the node content based on the style and value
                let node = if *style != TScalarStyle::Plain {
                    // Non-plain scalars are always stored as strings
                    Yaml::String(value.clone())
                } else if tag.is_some() {
                    // If there's a tag, we need to parse it according to the tag
                    // This is a simplified version - in practice you'd need more logic
                    if let Some(Tag {
                        ref handle,
                        ref suffix,
                    }) = tag
                    {
                        if handle == "tag:yaml.org,2002:" {
                            match suffix.as_ref() {
                                "bool" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        Yaml::Boolean(v)
                                    } else {
                                        Yaml::BadValue
                                    }
                                }
                                "int" => {
                                    if let Ok(v) = value.parse::<i64>() {
                                        Yaml::Integer(v)
                                    } else {
                                        Yaml::BadValue
                                    }
                                }
                                "float" => Yaml::Real(value.clone()),
                                "null" => {
                                    if value == "~" || value == "null" {
                                        Yaml::Null
                                    } else {
                                        Yaml::BadValue
                                    }
                                }
                                _ => Yaml::String(value.clone()),
                            }
                        } else {
                            Yaml::String(value.clone())
                        }
                    } else {
                        // Fallback to string if tag parsing fails
                        Yaml::String(value.clone())
                    }
                } else {
                    // Try to convert the value based on its content (plain scalar)
                    // This is a simplified version of Yaml::from_str
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
                        Yaml::Real(value.clone())
                    } else {
                        Yaml::String(value.clone())
                    }
                };

                // Store the node for this anchor
                self.store_anchor_node(*anchor_id, node);

                PositionSpan::new(mark)
            }
            _ => {
                // For all other events, just return a span with the current position
                PositionSpan::new(mark)
            }
        }
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}
