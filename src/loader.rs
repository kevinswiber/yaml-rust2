//! YAML loading and parsing functionality.
//!
//! This module provides the core YAML loading capabilities through the [`YamlLoader`] type.
//! It handles parsing YAML documents into Rust data structures, managing anchors and aliases,
//! and tracking document positions.
//!
//! The main entry point is [`YamlLoader::load_from_str`] which parses YAML text into
//! a vector of [`Yaml`] values representing the documents.
//!
//! # Example
//!
//! ```
//! use yaml_rust2::YamlLoader;
//!
//! let yaml = "
//! # A YAML document
//! key: value
//! sequence:
//!   - item1
//!   - item2
//! ";
//!
//! let docs = YamlLoader::load_from_str(yaml).unwrap();
//! ```
//!
//! The loader supports:
//! - Multiple documents in a single stream
//! - Anchors and aliases
//! - All standard YAML scalar types
//! - Block and flow collection styles
//! - Position tracking for error reporting
//!
//! When the `position_tracked_loader` feature is enabled, additional position tracking
//! capabilities become available through the [`PositionTracker`].

use std::collections::BTreeMap;
use std::mem;

use hashlink::LinkedHashMap;

use crate::error::ScanError;
use crate::parser::{Event, MarkedEventReceiver, Parser, Tag};
use crate::position::PositionTracker;
use crate::scanner::{Marker, TMappingStyle, TScalarStyle};
use crate::yaml::{parse_f64, Hash};
use crate::{AnchorId, Yaml};

/// Main structure for quickly parsing YAML.
///
/// See [`YamlLoader::load_from_str`].
pub struct YamlLoader {
    /// The different YAML documents that are loaded.
    docs: Vec<Yaml>,
    // states
    // (current node, anchor_id) tuple
    doc_stack: Vec<(Yaml, AnchorId)>,
    key_stack: Vec<Yaml>,
    anchor_map: BTreeMap<AnchorId, Yaml>,
    /// Position tracker for anchor and alias management
    position_tracker: PositionTracker,
    /// An error, if one was encountered.
    error: Option<ScanError>,
    // Track anchor names for emitting
    anchor_names: BTreeMap<AnchorId, String>,
    next_anchor_id: AnchorId,
    /// Whether to tolerate duplicate keys
    tolerate_duplicate_keys: bool,
}

impl Default for YamlLoader {
    fn default() -> Self {
        YamlLoader {
            docs: Vec::new(),
            doc_stack: Vec::new(),
            key_stack: Vec::new(),
            anchor_map: BTreeMap::new(),
            position_tracker: crate::position::PositionTracker::new(),
            error: None,
            anchor_names: BTreeMap::new(),
            next_anchor_id: AnchorId::new(1),
            tolerate_duplicate_keys: false,
        }
    }
}

impl MarkedEventReceiver for YamlLoader {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        if self.error.is_some() {
            return;
        }
        if let Err(e) = self.on_event_impl(ev, mark) {
            self.error = Some(e);
        }
    }
}

impl YamlLoader {
    /// Register a new anchor with the given name
    ///
    /// This method assigns a unique ID to an anchor name and stores the mapping.
    /// Note: Currently this method is not actively used as anchor tracking is primarily
    /// handled through the position_tracker, but it's kept for potential future use.
    #[allow(dead_code)]
    fn register_anchor(&mut self, name: String) -> AnchorId {
        let id = self.next_anchor_id;
        self.next_anchor_id = AnchorId::new(self.next_anchor_id.value() + 1);
        self.anchor_names.insert(id, name);
        id
    }

    fn on_event_impl(&mut self, ev: Event, mark: Marker) -> Result<(), ScanError> {
        match ev {
            Event::DocumentStart | Event::Nothing | Event::StreamStart | Event::StreamEnd => {
                // do nothing
            }
            Event::DocumentEnd => {
                match self.doc_stack.len() {
                    // empty document
                    0 => self.docs.push(Yaml::BadValue),
                    1 => self.docs.push(self.doc_stack.pop().unwrap().0),
                    _ => unreachable!(),
                }
            }
            Event::SequenceStart(aid, tag, style) => {
                // Track the sequence start in the position tracker
                let _span = self
                    .position_tracker
                    .process_event(&Event::SequenceStart(aid, tag, style), mark);

                self.doc_stack.push((Yaml::Array(Vec::new()), aid));

                // If this is an anchor, store it in position_tracker
                if aid > AnchorId::new(0) {
                    let empty_array = Yaml::Array(Vec::new());
                    self.position_tracker.track_anchor(aid, mark);
                    self.position_tracker.store_anchor_node(aid, empty_array);
                }
            }
            Event::SequenceEnd => {
                // Track the sequence end in the position tracker
                self.position_tracker
                    .process_event(&Event::SequenceEnd, mark);

                let node = self.doc_stack.pop().unwrap();
                self.insert_new_node(node, mark)?;
            }
            Event::MappingStart(aid, tag, style) => {
                // Track the mapping start in the position tracker
                let _span = self
                    .position_tracker
                    .process_event(&Event::MappingStart(aid, tag, style), mark);

                let node = if aid > AnchorId::new(0) {
                    if let Some(referenced_node) = self.position_tracker.get_anchor_yaml(aid) {
                        // If it's an alias reference, get the referenced node from position_tracker
                        referenced_node.clone()
                    } else {
                        // If it's a new anchor, create new hash
                        let hash = Yaml::Hash(Hash::new());
                        // Store the node in the position_tracker
                        self.position_tracker.store_anchor_node(aid, hash.clone());
                        hash
                    }
                } else {
                    // Regular mapping, create new hash
                    Yaml::Hash(Hash::new())
                };
                self.doc_stack.push((node, aid));
                self.key_stack.push(Yaml::BadValue);

                if aid > AnchorId::new(0) {
                    let empty_hash = Yaml::Hash(LinkedHashMap::new());
                    self.position_tracker.track_anchor(aid, mark);
                    self.position_tracker.store_anchor_node(aid, empty_hash);
                }

                // Store the style information
                // self.position_tracker.store_mapping_style(aid, style);
            }
            Event::MappingEnd => {
                // Track the mapping end in the position tracker
                self.position_tracker
                    .process_event(&Event::MappingEnd, mark);

                self.key_stack.pop().unwrap();
                let node = self.doc_stack.pop().unwrap();
                self.insert_new_node(node, mark)?;
            }
            Event::Scalar(v, style, aid, tag) => {
                let node = if style != TScalarStyle::Plain {
                    Yaml::String(v.clone())
                } else if let Some(Tag {
                    ref handle,
                    ref suffix,
                }) = tag
                {
                    if handle == "tag:yaml.org,2002:" {
                        match suffix.as_ref() {
                            "bool" => match v.parse::<bool>() {
                                Err(_) => Yaml::BadValue,
                                Ok(v) => Yaml::Boolean(v),
                            },
                            "int" => match v.parse::<i64>() {
                                Err(_) => Yaml::BadValue,
                                Ok(v) => Yaml::Integer(v),
                            },
                            "float" => match parse_f64(&v) {
                                Some(_) => Yaml::Real(v.clone()),
                                None => Yaml::BadValue,
                            },
                            "null" => match v.as_ref() {
                                "~" | "null" => Yaml::Null,
                                _ => Yaml::BadValue,
                            },
                            _ => Yaml::String(v.clone()),
                        }
                    } else {
                        Yaml::String(v.clone())
                    }
                } else {
                    Yaml::from_str(&v)
                };

                if aid > AnchorId::new(0) {
                    if !self.anchor_names.contains_key(&aid) {
                        self.anchor_names.insert(aid, v);
                    }
                    // Store the node in position_tracker
                    self.position_tracker.store_anchor_node(aid, node.clone());
                }
                self.insert_new_node((node, aid), mark)?;
            }
            Event::Alias(id) => {
                // Track the alias event in the position tracker
                self.position_tracker.process_event(&Event::Alias(id), mark);

                // Create the alias node as before
                let n = Yaml::Alias(id);
                self.insert_new_node((n, AnchorId::new(0)), mark)?;
            }
        }
        Ok(())
    }

    fn insert_new_node(&mut self, node: (Yaml, AnchorId), mark: Marker) -> Result<(), ScanError> {
        if node.1 > AnchorId::new(0) {
            // Store the node in position_tracker instead of anchor_map
            self.position_tracker
                .store_anchor_node(node.1, node.0.clone());
        }
        if self.doc_stack.is_empty() {
            self.doc_stack.push(node);
        } else {
            let parent = self.doc_stack.last_mut().unwrap();
            match *parent {
                (Yaml::Array(ref mut v), _) => {
                    let mut newval = node.0;
                    if let Yaml::Alias(id) = newval {
                        // Get from position_tracker
                        let actual_val = self.position_tracker.get_anchor_yaml(id);

                        // Check for self-referential alias
                        if let Some(actual_val) = actual_val {
                            if let Yaml::Hash(ref h) = actual_val {
                                if h.is_empty() {
                                    newval = Yaml::BadValue;
                                } else {
                                    newval = actual_val;
                                }
                            } else {
                                newval = actual_val;
                            }
                        } else {
                            newval = Yaml::BadValue;
                        }
                    }
                    v.push(newval);
                }
                (Yaml::Hash(ref mut h), _) => {
                    let cur_key = self.key_stack.last_mut().unwrap();
                    if cur_key.is_badvalue() {
                        *cur_key = node.0;
                    } else {
                        let mut newkey = Yaml::BadValue;
                        mem::swap(&mut newkey, cur_key);
                        // Check if the key is an alias
                        let mut actual_key = newkey;
                        if let Yaml::Alias(id) = actual_key {
                            // Get from position_tracker
                            if let Some(referenced_key) = self.position_tracker.get_anchor_yaml(id)
                            {
                                actual_key = referenced_key;
                            }
                        }
                        // Check if the value is an alias
                        let mut actual_val = node.0;
                        if let Yaml::Alias(id) = actual_val {
                            // Get from position_tracker
                            let referenced_val = self.position_tracker.get_anchor_yaml(id);

                            // Check for self-referential alias
                            if let Some(referenced_val) = referenced_val {
                                if let Yaml::Hash(ref h) = referenced_val {
                                    if h.is_empty() {
                                        actual_val = Yaml::BadValue;
                                    } else {
                                        actual_val = referenced_val;
                                    }
                                } else {
                                    actual_val = referenced_val;
                                }
                            } else {
                                actual_val = Yaml::BadValue;
                            }
                        }
                        // Check if the key already exists in the mapping
                        if h.insert(actual_key.clone(), actual_val).is_some() {
                            // Only raise an error if tolerate_duplicate_keys is false
                            if !self.tolerate_duplicate_keys {
                                return Err(ScanError::new_string(
                                    mark,
                                    format!("{actual_key:?}: duplicated key in mapping"),
                                ));
                            }
                            // If tolerate_duplicate_keys is true, we've already inserted the new value
                            // and overwritten the old one, so we just continue
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    /// Load the given string as a set of YAML documents.
    ///
    /// The `source` is interpreted as YAML documents and is parsed. Parsing succeeds if and only
    /// if all documents are parsed successfully. An error in a latter document prevents the former
    /// from being returned.
    /// # Errors
    /// Returns `ScanError` when loading fails.
    pub fn load_from_str(source: &str) -> Result<Vec<Yaml>, ScanError> {
        Self::load_from_iter(source.chars())
    }

    /// Load the contents of the given iterator as a set of YAML documents.
    ///
    /// The `source` is interpreted as YAML documents and is parsed. Parsing succeeds if and only
    /// if all documents are parsed successfully. An error in a latter document prevents the former
    /// from being returned.
    /// # Errors
    /// Returns `ScanError` when loading fails.
    pub fn load_from_iter<I: Iterator<Item = char>>(source: I) -> Result<Vec<Yaml>, ScanError> {
        let mut parser = Parser::new(source);
        Self::load_from_parser(&mut parser)
    }

    /// Load the contents from the specified Parser as a set of YAML documents.
    ///
    /// Parsing succeeds if and only if all documents are parsed successfully.
    /// An error in a latter document prevents the former from being returned.
    /// # Errors
    /// Returns `ScanError` when loading fails.
    pub fn load_from_parser<I: Iterator<Item = char>>(
        parser: &mut Parser<I>,
    ) -> Result<Vec<Yaml>, ScanError> {
        let mut loader = YamlLoader::default();
        // Get the tolerate_duplicate_keys option from the parser
        loader.tolerate_duplicate_keys = parser.get_tolerate_duplicate_keys();
        parser.load(&mut loader, true)?;
        // Copy anchor names from parser
        for (id, name) in parser.get_anchor_names() {
            loader.anchor_names.insert(*id, name.clone());
        }
        if let Some(e) = loader.error {
            Err(e)
        } else {
            Ok(loader.docs)
        }
    }

    /// Return a reference to the parsed Yaml documents.
    #[must_use]
    pub fn documents(&self) -> &[Yaml] {
        &self.docs
    }

    /// Get the anchor name for a given ID
    pub fn get_anchor_name(&self, id: AnchorId) -> Option<&str> {
        self.anchor_names.get(&id).map(|s| s.as_str())
    }

    /// Get the anchor ID associated with a node, if any.
    ///
    /// This method searches the anchor map for a node that matches the provided one,
    /// and returns its anchor ID if found.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to look up
    ///
    /// # Returns
    ///
    /// The anchor ID if the node is anchored, or None otherwise
    pub fn get_anchor_id(&self, node: &Yaml) -> Option<AnchorId> {
        // First try using position_tracker to find the anchor ID
        if let Some(id) = self.position_tracker.find_anchor_id(node) {
            return Some(id);
        }

        // Fall back to scanning the anchor_map (for backward compatibility)
        for (id, anchor_node) in &self.anchor_map {
            if anchor_node == node {
                return Some(*id);
            }
        }
        None
    }
}

/// The default loader implementation used by the library
///
/// When the `position_tracked_loader` feature is enabled, this will use the
/// `PositionTrackedLoader` implementation. Otherwise, it will use the
/// original `YamlLoader` implementation.
#[cfg(not(feature = "source_mapping"))]
pub type DefaultLoader = YamlLoader;

/// The default loader implementation used by the library
///
/// When the `position_tracked_loader` feature is enabled, this will use the
/// `PositionTrackedLoader` implementation. Otherwise, it will use the
/// original `YamlLoader` implementation.
#[cfg(feature = "source_mapping")]
pub type DefaultLoader = crate::PositionTrackedLoader;

/// Load YAML documents from a string
///
/// This function uses the default loader implementation, which depends
/// on the feature flags enabled.
///
/// # Errors
/// Returns a `ScanError` if the YAML document could not be parsed.
pub fn load_from_str(source: &str) -> Result<Vec<Yaml>, ScanError> {
    DefaultLoader::load_from_str(source)
}

/// Load YAML documents from an iterator of characters
///
/// This function uses the default loader implementation, which depends
/// on the feature flags enabled.
///
/// # Errors
/// Returns a `ScanError` if the YAML document could not be parsed.
pub fn load_from_iter<I: Iterator<Item = char>>(source: I) -> Result<Vec<Yaml>, ScanError> {
    DefaultLoader::load_from_iter(source)
}
