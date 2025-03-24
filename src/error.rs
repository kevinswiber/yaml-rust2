//! Error types for YAML parsing and loading operations.
//!
//! This module provides error types that can occur during YAML processing:
//!
//! - [`ScanError`] - Represents errors that occur during the lexical scanning phase,
//!   such as invalid characters or malformed tokens
//!
//! - [`LoadError`] - Represents semantic errors that occur during document loading,
//!   such as duplicate keys or invalid anchors
//!
//! The error types implement standard Rust error traits and provide detailed
//! information about error locations and causes.

use std::{error::Error, fmt};

use crate::scanner::Marker;

/// An error that occurred while scanning.
#[derive(Clone, PartialEq, Debug, Eq)]
pub struct ScanError {
    /// The position at which the error happened in the source.
    mark: Marker,
    /// Human-readable details about the error.
    info: String,
}

impl ScanError {
    /// Create a new error from a location and an error string.
    #[must_use]
    pub fn new(loc: Marker, info: &str) -> ScanError {
        ScanError {
            mark: loc,
            info: info.to_owned(),
        }
    }

    /// Create a new error from a location and an error string.
    #[must_use]
    pub fn new_string(loc: Marker, info: String) -> ScanError {
        ScanError { mark: loc, info }
    }

    /// Return the marker pointing to the error in the source.
    #[must_use]
    pub fn marker(&self) -> &Marker {
        &self.mark
    }

    /// Return the information string describing the error that happened.
    #[must_use]
    pub fn info(&self) -> &str {
        self.info.as_ref()
    }

    /// Format the error with source location information
    ///
    /// # Arguments
    ///
    /// * `source` - The source text of the YAML document
    ///
    /// # Returns
    ///
    /// A formatted error message with location information and a pointer to the error position
    #[must_use]
    pub fn format_with_source(&self, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();

        // Get the line containing the error (adjust for 0-indexing)
        let line_number = self.mark.line();
        let line_idx = line_number.saturating_sub(1);

        if line_idx >= lines.len() {
            return format!("Error at line {}: {}", line_number, self.info);
        }

        let line_content = lines[line_idx];
        let column = self.mark.col().saturating_sub(1); // Convert to 0-indexed

        let mut result = format!(
            "Error at {}:{}: {}\n",
            line_number,
            self.mark.col(),
            self.info
        );
        result.push_str(&format!("{}: {}\n", line_number, line_content));

        if column <= line_content.len() {
            let padding = " ".repeat(line_number.to_string().len() + 2 + column);
            result.push_str(&format!("{}^\n", padding));
        }

        result
    }

    /// Format the error with context lines
    ///
    /// # Arguments
    ///
    /// * `source` - The source text of the YAML document
    /// * `context_lines` - Number of lines to show before and after the error line
    ///
    /// # Returns
    ///
    /// A formatted error message with location information and surrounding context
    #[must_use]
    pub fn format_with_context(&self, source: &str, context_lines: usize) -> String {
        let lines: Vec<&str> = source.lines().collect();

        // Get the line containing the error (adjust for 0-indexing)
        let line_number = self.mark.line();
        let line_idx = line_number.saturating_sub(1);

        if line_idx >= lines.len() {
            return format!("Error at line {}: {}", line_number, self.info);
        }

        // Calculate the range of lines to show
        let start_line = line_number.saturating_sub(context_lines);
        let end_line = line_number.saturating_add(context_lines).min(lines.len());

        let mut result = format!(
            "Error at {}:{}: {}\n\n",
            line_number,
            self.mark.col(),
            self.info
        );

        // Add line numbers and content for context
        for i in start_line..=end_line {
            let idx = i.saturating_sub(1);
            if idx < lines.len() {
                let line_content = lines[idx];

                // Highlight the error line
                if i == line_number {
                    result.push_str(&format!("> {}: {}\n", i, line_content));

                    // Add caret pointer
                    let column = self.mark.col().saturating_sub(1);
                    if column <= line_content.len() {
                        let padding = "  ".to_string()
                            + &" ".repeat(i.to_string().len())
                            + "  "
                            + &" ".repeat(column);
                        result.push_str(&format!("{}^\n", padding));
                    }
                } else {
                    result.push_str(&format!("  {}: {}\n", i, line_content));
                }
            }
        }

        result
    }
}

impl Error for ScanError {
    fn description(&self) -> &str {
        self.info.as_ref()
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl fmt::Display for ScanError {
    // col starts from 0
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{} at byte {} line {} column {}",
            self.info,
            self.mark.index,
            self.mark.line,
            self.mark.col + 1,
        )
    }
}

/// An error that happened when loading a YAML document.
#[derive(Debug)]
pub enum LoadError {
    /// An I/O error.
    IO(std::io::Error),
    /// An error within the scanner. This indicates a malformed YAML input.
    Scan(ScanError),
    /// A decoding error (e.g.: Invalid UTF-8).
    Decode(std::borrow::Cow<'static, str>),
}

impl From<std::io::Error> for LoadError {
    fn from(error: std::io::Error) -> Self {
        LoadError::IO(error)
    }
}
