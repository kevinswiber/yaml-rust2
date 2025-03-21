//! Position tracking for YAML constructs.
//!
//! This module provides structures for tracking the positions of YAML constructs
//! in the source document, including both start and end positions.

use crate::parser::Event;
use crate::scanner::{Marker, ScanError, TMappingStyle};

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
    /// ```
    /// # use yaml_rust2::scanner::Marker;
    /// # use yaml_rust2::position::PositionSpan;
    /// # let start = Marker::new(0, 1, 1);
    /// let span = PositionSpan::new(start);
    /// assert_eq!(span.start, start);
    /// assert_eq!(span.end, None);
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
    /// ```
    /// # use yaml_rust2::scanner::Marker;
    /// # use yaml_rust2::position::PositionSpan;
    /// # let start = Marker::new(0, 1, 1);
    /// # let end = Marker::new(0, 1, 10);
    /// let span = PositionSpan::with_end(start, end);
    /// assert_eq!(span.start, start);
    /// assert_eq!(span.end, Some(end));
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
    /// ```
    /// # use yaml_rust2::scanner::Marker;
    /// # use yaml_rust2::position::PositionSpan;
    /// # let start = Marker::new(0, 1, 1);
    /// # let end = Marker::new(0, 1, 10);
    /// let mut span = PositionSpan::new(start);
    /// assert_eq!(span.end, None);
    /// span.set_end(end);
    /// assert_eq!(span.end, Some(end));
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
}

impl PositionTracker {
    /// Create a new position tracker
    #[must_use]
    pub fn new() -> Self {
        PositionTracker {
            position_stack: Vec::new(),
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

    /// Process an event and update position tracking
    ///
    /// This method takes an event and its position, updates the position tracker's
    /// internal state, and returns a PositionSpan for the event.
    ///
    /// For flow collections, it builds a complete span from the opening to the closing
    /// delimiter. For other events, it just returns a span with the current position.
    pub fn process_event(&mut self, event: &Event, mark: Marker) -> PositionSpan {
        match event {
            Event::MappingStart(_, _, style, _) if *style == TMappingStyle::Flow => {
                // For flow mappings, push the start position to the stack
                self.push(0, mark);
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
            Event::SequenceStart(_, _) => {
                // For sequences, push the start position to the stack
                self.push(1, mark);
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
