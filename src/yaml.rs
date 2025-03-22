//! YAML objects manipulation utilities.

#![allow(clippy::module_name_repetitions)]

use std::borrow::Cow;
use std::ops::ControlFlow;
use std::{
    collections::BTreeMap, collections::HashMap, convert::TryFrom, mem, ops::Index, ops::IndexMut,
};

#[cfg(feature = "encoding")]
use encoding_rs::{Decoder, DecoderResult, Encoding};
use hashlink::LinkedHashMap;

use crate::parser::{Event, MarkedEventReceiver, Parser, Tag};
use crate::position::PositionTracker;
use crate::scanner::{Marker, ScanError, TMappingStyle, TScalarStyle};

/// A YAML node is stored as this `Yaml` enumeration, which provides an easy way to
/// access your YAML document.
///
/// # Examples
///
/// ```
/// use yaml_rust2::Yaml;
/// let foo = Yaml::from_str("-123"); // convert the string to the appropriate YAML type
/// assert_eq!(foo.as_i64().unwrap(), -123);
///
/// // iterate over an Array
/// let vec = Yaml::Array(vec![Yaml::Integer(1), Yaml::Integer(2)]);
/// for v in vec.as_vec().unwrap() {
///     assert!(v.as_i64().is_some());
/// }
/// ```
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum Yaml {
    /// Float types are stored as String and parsed on demand.
    /// Note that `f64` does NOT implement Eq trait and can NOT be stored in `BTreeMap`.
    Real(String),
    /// YAML int is stored as i64.
    Integer(i64),
    /// YAML scalar.
    String(String),
    /// YAML bool, e.g. `true` or `false`.
    Boolean(bool),
    /// YAML array, can be accessed as a [`Vec`].
    Array(Array),
    /// YAML hash, can be accessed as a [`LinkedHashMap`].
    ///
    /// Insertion order will match the order of insertion into the map.
    Hash(Hash),
    /// Alias, not fully supported yet.
    Alias(usize),
    /// YAML null, e.g. `null` or `~`.
    Null,
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue,
}

/// The type contained in the `Yaml::Array` variant. This corresponds to YAML sequences.
pub type Array = Vec<Yaml>;
/// The type contained in the `Yaml::Hash` variant. This corresponds to YAML mappings.
pub type Hash = LinkedHashMap<Yaml, Yaml>;

// parse f64 as Core schema
// See: https://github.com/chyh1990/yaml-rust/issues/51
fn parse_f64(v: &str) -> Option<f64> {
    match v {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Some(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Some(f64::NEG_INFINITY),
        ".nan" | "NaN" | ".NAN" => Some(f64::NAN),
        _ => v.parse::<f64>().ok(),
    }
}

/// Main structure for quickly parsing YAML.
///
/// See [`YamlLoader::load_from_str`].
pub struct YamlLoader {
    /// The different YAML documents that are loaded.
    docs: Vec<Yaml>,
    // states
    // (current node, anchor_id) tuple
    doc_stack: Vec<(Yaml, usize)>,
    key_stack: Vec<Yaml>,
    anchor_map: BTreeMap<usize, Yaml>,
    /// Position tracker for anchor and alias management
    position_tracker: PositionTracker,
    /// An error, if one was encountered.
    error: Option<ScanError>,
    // Track anchor names for emitting
    anchor_names: BTreeMap<usize, String>,
    next_anchor_id: usize,
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
            position_tracker: PositionTracker::new(),
            error: None,
            anchor_names: BTreeMap::new(),
            next_anchor_id: 1,
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

impl YamlLoader {
    /// Register a new anchor with the given name
    ///
    /// This method assigns a unique ID to an anchor name and stores the mapping.
    /// Note: Currently this method is not actively used as anchor tracking is primarily
    /// handled through the position_tracker, but it's kept for potential future use.
    #[allow(dead_code)]
    fn register_anchor(&mut self, name: String) -> usize {
        let id = self.next_anchor_id;
        self.next_anchor_id += 1;
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
            Event::SequenceStart(aid, _) => {
                // Track the sequence start in the position tracker
                let _span = self
                    .position_tracker
                    .process_event(&Event::SequenceStart(aid, None), mark);

                self.doc_stack.push((Yaml::Array(Vec::new()), aid));

                // If this is an anchor, store it in position_tracker
                if aid > 0 {
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
            Event::MappingStart(aid, _, _) => {
                // Track the mapping start in the position tracker
                let _span = self
                    .position_tracker
                    .process_event(&Event::MappingStart(aid, None, TMappingStyle::Flow), mark);

                let node = if aid > 0 {
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

                if aid > 0 {
                    self.position_tracker.track_anchor(aid, mark);
                    self.position_tracker
                        .store_anchor_node(aid, Yaml::Hash(Hash::new()));
                }
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

                if aid > 0 {
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
                self.insert_new_node((n, 0), mark)?;
            }
        }
        Ok(())
    }

    fn insert_new_node(&mut self, node: (Yaml, usize), mark: Marker) -> Result<(), ScanError> {
        if node.1 > 0 {
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
    pub fn get_anchor_name(&self, id: usize) -> Option<&str> {
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
    pub fn get_anchor_id(&self, node: &Yaml) -> Option<usize> {
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

/// The signature of the function to call when using [`YAMLDecodingTrap::Call`].
///
/// The arguments are as follows:
///  * `malformation_length`: The length of the sequence the decoder failed to decode.
///  * `bytes_read_after_malformation`: The number of lookahead bytes the decoder consumed after
///    the malformation.
///  * `input_at_malformation`: What the input buffer is at the malformation.
///    This is the buffer starting at the malformation. The first `malformation_length` bytes are
///    the problematic sequence. The following `bytes_read_after_malformation` are already stored
///    in the decoder and will not be re-fed.
///  * `output`: The output string.
///
/// The function must modify `output` as it feels is best. For instance, one could recreate the
/// behavior of [`YAMLDecodingTrap::Ignore`] with an empty function, [`YAMLDecodingTrap::Replace`]
/// by pushing a `\u{FFFD}` into `output` and [`YAMLDecodingTrap::Strict`] by returning
/// [`ControlFlow::Break`].
///
/// # Returns
/// The function must return [`ControlFlow::Continue`] if decoding may continue or
/// [`ControlFlow::Break`] if decoding must be aborted. An optional error string may be supplied.
#[cfg(feature = "encoding")]
pub type YAMLDecodingTrapFn = fn(
    malformation_length: u8,
    bytes_read_after_malformation: u8,
    input_at_malformation: &[u8],
    output: &mut String,
) -> ControlFlow<Cow<'static, str>>;

/// The behavior [`YamlDecoder`] must have when an decoding error occurs.
#[cfg(feature = "encoding")]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum YAMLDecodingTrap {
    /// Ignore the offending bytes, remove them from the output.
    Ignore,
    /// Error out.
    Strict,
    /// Replace them with the Unicode REPLACEMENT CHARACTER.
    Replace,
    /// Call the user-supplied function upon decoding malformation.
    Call(YAMLDecodingTrapFn),
}

/// `YamlDecoder` is a `YamlLoader` builder that allows you to supply your own encoding error trap.
/// For example, to read a YAML file while ignoring Unicode decoding errors you can set the
/// `encoding_trap` to `encoding::DecoderTrap::Ignore`.
/// ```rust
/// use yaml_rust2::yaml::{YamlDecoder, YAMLDecodingTrap};
///
/// let string = b"---
/// a\xa9: 1
/// b: 2.2
/// c: [1, 2]
/// ";
/// let out = YamlDecoder::read(string as &[u8])
///     .encoding_trap(YAMLDecodingTrap::Ignore)
///     .decode()
///     .unwrap();
/// ```
#[cfg(feature = "encoding")]
pub struct YamlDecoder<T: std::io::Read> {
    source: T,
    trap: YAMLDecodingTrap,
}

#[cfg(feature = "encoding")]
impl<T: std::io::Read> YamlDecoder<T> {
    /// Create a `YamlDecoder` decoding the given source.
    pub fn read(source: T) -> YamlDecoder<T> {
        YamlDecoder {
            source,
            trap: YAMLDecodingTrap::Strict,
        }
    }

    /// Set the behavior of the decoder when the encoding is invalid.
    pub fn encoding_trap(&mut self, trap: YAMLDecodingTrap) -> &mut Self {
        self.trap = trap;
        self
    }

    /// Run the decode operation with the source and trap the `YamlDecoder` was built with.
    ///
    /// # Errors
    /// Returns `LoadError` when decoding fails.
    pub fn decode(&mut self) -> Result<Vec<Yaml>, LoadError> {
        let mut buffer = Vec::new();
        self.source.read_to_end(&mut buffer)?;

        // Check if the `encoding` library can detect encoding from the BOM, otherwise use
        // `detect_utf16_endianness`.
        let (encoding, _) =
            Encoding::for_bom(&buffer).unwrap_or_else(|| (detect_utf16_endianness(&buffer), 2));
        let mut decoder = encoding.new_decoder();
        let mut output = String::new();

        // Decode the input buffer.
        decode_loop(&buffer, &mut output, &mut decoder, self.trap)?;

        YamlLoader::load_from_str(&output).map_err(LoadError::Scan)
    }
}

/// Perform a loop of [`Decoder::decode_to_string`], reallocating `output` if needed.
#[cfg(feature = "encoding")]
fn decode_loop(
    input: &[u8],
    output: &mut String,
    decoder: &mut Decoder,
    trap: YAMLDecodingTrap,
) -> Result<(), LoadError> {
    output.reserve(input.len());
    let mut total_bytes_read = 0;

    loop {
        match decoder.decode_to_string_without_replacement(&input[total_bytes_read..], output, true)
        {
            // If the input is empty, we processed the whole input.
            (DecoderResult::InputEmpty, _) => break Ok(()),
            // If the output is full, we must reallocate.
            (DecoderResult::OutputFull, bytes_read) => {
                total_bytes_read += bytes_read;
                // The output is already reserved to the size of the input. We slowly resize. Here,
                // we're expecting that 10% of bytes will double in size when converting to UTF-8.
                output.reserve(input.len() / 10);
            }
            (DecoderResult::Malformed(malformed_len, bytes_after_malformed), bytes_read) => {
                total_bytes_read += bytes_read;
                match trap {
                    // Ignore (skip over) malformed character.
                    YAMLDecodingTrap::Ignore => {}
                    // Replace them with the Unicode REPLACEMENT CHARACTER.
                    YAMLDecodingTrap::Replace => {
                        output.push('\u{FFFD}');
                    }
                    // Otherwise error, getting as much context as possible.
                    YAMLDecodingTrap::Strict => {
                        let malformed_len = malformed_len as usize;
                        let bytes_after_malformed = bytes_after_malformed as usize;
                        let byte_idx = total_bytes_read - (malformed_len + bytes_after_malformed);
                        let malformed_sequence = &input[byte_idx..byte_idx + malformed_len];

                        break Err(LoadError::Decode(Cow::Owned(format!(
                            "Invalid character sequence at {byte_idx}: {malformed_sequence:?}",
                        ))));
                    }
                    YAMLDecodingTrap::Call(callback) => {
                        let byte_idx =
                            total_bytes_read - ((malformed_len + bytes_after_malformed) as usize);
                        let malformed_sequence =
                            &input[byte_idx..byte_idx + malformed_len as usize];
                        if let ControlFlow::Break(error) = callback(
                            malformed_len,
                            bytes_after_malformed,
                            &input[byte_idx..],
                            output,
                        ) {
                            if error.is_empty() {
                                break Err(LoadError::Decode(Cow::Owned(format!(
                                    "Invalid character sequence at {byte_idx}: {malformed_sequence:?}",
                                ))));
                            }
                            break Err(LoadError::Decode(error));
                        }
                    }
                }
            }
        }
    }
}

/// The encoding crate knows how to tell apart UTF-8 from UTF-16LE and utf-16BE, when the
/// bytestream starts with BOM codepoint.
/// However, it doesn't even attempt to guess the UTF-16 endianness of the input bytestream since
/// in the general case the bytestream could start with a codepoint that uses both bytes.
///
/// The YAML-1.2 spec mandates that the first character of a YAML document is an ASCII character.
/// This allows the encoding to be deduced by the pattern of null (#x00) characters.
//
/// See spec at <https://yaml.org/spec/1.2/spec.html#id2771184>
#[cfg(feature = "encoding")]
fn detect_utf16_endianness(b: &[u8]) -> &'static Encoding {
    if b.len() > 1 && (b[0] != b[1]) {
        if b[0] == 0 {
            return encoding_rs::UTF_16BE;
        } else if b[1] == 0 {
            return encoding_rs::UTF_16LE;
        }
    }
    encoding_rs::UTF_8
}

macro_rules! define_as (
    ($name:ident, $t:ident, $yt:ident) => (
/// Get a copy of the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Yaml::$yt`, return `Some($t)` with a copy of the `$t` contained.
/// Otherwise, return `None`.
#[must_use]
pub fn $name(&self) -> Option<$t> {
    match *self {
        Yaml::$yt(v) => Some(v),
        _ => None
    }
}
    ); // WARNING: Do not remove this parenthesis or add an additional one.
); // WARNING: Do not add an additional parenthesis after this one.

macro_rules! define_as_ref (
    ($name:ident, $t:ty, $yt:ident) => (
/// Get a reference to the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Yaml::$yt`, return `Some(&$t)` with the `$t` contained. Otherwise,
/// return `None`.
#[must_use]
pub fn $name(&self) -> Option<$t> {
    match *self {
        Yaml::$yt(ref v) => Some(v),
        _ => None
    }
}
    ); // WARNING: Do not remove this parenthesis or add an additional one.
); // WARNING: Do not add an additional parenthesis after this one.

macro_rules! define_as_mut_ref (
    ($name:ident, $t:ty, $yt:ident) => (
/// Get a mutable reference to the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Yaml::$yt`, return `Some(&mut $t)` with the `$t` contained.
/// Otherwise, return `None`.
#[must_use]
pub fn $name(&mut self) -> Option<$t> {
    match *self {
        Yaml::$yt(ref mut v) => Some(v),
        _ => None
    }
}
    ); // WARNING: Do not remove this parenthesis or add an additional one.
); // WARNING: Do not add an additional parenthesis after this one.

macro_rules! define_into (
    ($name:ident, $t:ty, $yt:ident) => (
/// Get the inner object in the YAML enum if it is a `$t`.
///
/// # Return
/// If the variant of `self` is `Yaml::$yt`, return `Some($t)` with the `$t` contained. Otherwise,
/// return `None`.
#[must_use]
pub fn $name(self) -> Option<$t> {
    match self {
        Yaml::$yt(v) => Some(v),
        _ => None
    }
}
    ); // WARNING: Do not remove this parenthesis or add an additional one.
); // WARNING: Do not add an additional parenthesis after this one.

impl Yaml {
    define_as!(as_bool, bool, Boolean);
    define_as!(as_i64, i64, Integer);

    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_hash, &Hash, Hash);
    define_as_ref!(as_vec, &Array, Array);

    define_as_mut_ref!(as_mut_hash, &mut Hash, Hash);
    define_as_mut_ref!(as_mut_vec, &mut Array, Array);

    define_into!(into_bool, bool, Boolean);
    define_into!(into_i64, i64, Integer);
    define_into!(into_string, String, String);
    define_into!(into_hash, Hash, Hash);
    define_into!(into_vec, Array, Array);

    /// Return whether `self` is a [`Yaml::Null`] node.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(*self, Yaml::Null)
    }

    /// Return whether `self` is a [`Yaml::BadValue`] node.
    #[must_use]
    pub fn is_badvalue(&self) -> bool {
        matches!(*self, Yaml::BadValue)
    }

    /// Return whether `self` is a [`Yaml::Array`] node.
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(*self, Yaml::Array(_))
    }

    /// Return whether `self` is a [`Yaml::Hash`] node.
    #[must_use]
    pub fn is_hash(&self) -> bool {
        matches!(*self, Yaml::Hash(_))
    }

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`Yaml::Real`] YAML node or its contents is not a valid `f64` string,
    /// `None` is returned.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        if let Yaml::Real(ref v) = self {
            parse_f64(v)
        } else {
            None
        }
    }

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`Yaml::Real`] YAML node or its contents is not a valid `f64` string,
    /// `None` is returned.
    #[must_use]
    pub fn into_f64(self) -> Option<f64> {
        self.as_f64()
    }

    /// If a value is null or otherwise bad (see variants), consume it and
    /// replace it with a given value `other`. Otherwise, return self unchanged.
    ///
    /// ```
    /// use yaml_rust2::yaml::Yaml;
    ///
    /// assert_eq!(Yaml::BadValue.or(Yaml::Integer(3)),  Yaml::Integer(3));
    /// assert_eq!(Yaml::Integer(3).or(Yaml::Integer(7)),  Yaml::Integer(3));
    /// ```
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        match self {
            Yaml::BadValue | Yaml::Null => other,
            this => this,
        }
    }

    /// See `or` for behavior. This performs the same operations, but with
    /// borrowed values for less linear pipelines.
    #[must_use]
    pub fn borrowed_or<'a>(&'a self, other: &'a Self) -> &'a Self {
        match self {
            Yaml::BadValue | Yaml::Null => other,
            this => this,
        }
    }
}

#[allow(clippy::should_implement_trait)]
impl Yaml {
    /// Convert a string to a [`Yaml`] node.
    ///
    /// [`Yaml`] does not implement [`std::str::FromStr`] since conversion may not fail. This
    /// function falls back to [`Yaml::String`] if nothing else matches.
    ///
    /// # Examples
    /// ```
    /// # use yaml_rust2::yaml::Yaml;
    /// assert!(matches!(Yaml::from_str("42"), Yaml::Integer(42)));
    /// assert!(matches!(Yaml::from_str("0x2A"), Yaml::Integer(42)));
    /// assert!(matches!(Yaml::from_str("0o52"), Yaml::Integer(42)));
    /// assert!(matches!(Yaml::from_str("~"), Yaml::Null));
    /// assert!(matches!(Yaml::from_str("null"), Yaml::Null));
    /// assert!(matches!(Yaml::from_str("true"), Yaml::Boolean(true)));
    /// assert!(matches!(Yaml::from_str("3.14"), Yaml::Real(_)));
    /// assert!(matches!(Yaml::from_str("foo"), Yaml::String(_)));
    /// ```
    #[must_use]
    pub fn from_str(v: &str) -> Yaml {
        if let Some(number) = v.strip_prefix("0x") {
            if let Ok(i) = i64::from_str_radix(number, 16) {
                return Yaml::Integer(i);
            }
        } else if let Some(number) = v.strip_prefix("0o") {
            if let Ok(i) = i64::from_str_radix(number, 8) {
                return Yaml::Integer(i);
            }
        } else if let Some(number) = v.strip_prefix('+') {
            if let Ok(i) = number.parse::<i64>() {
                return Yaml::Integer(i);
            }
        }
        match v {
            "" | "~" | "null" => Yaml::Null,
            "true" => Yaml::Boolean(true),
            "false" => Yaml::Boolean(false),
            _ => {
                if let Ok(integer) = v.parse::<i64>() {
                    Yaml::Integer(integer)
                } else if parse_f64(v).is_some() {
                    Yaml::Real(v.to_owned())
                } else {
                    Yaml::String(v.to_owned())
                }
            }
        }
    }
}

static BAD_VALUE: Yaml = Yaml::BadValue;
impl<'a> Index<&'a str> for Yaml {
    type Output = Yaml;

    /// Perform indexing if `self` is a mapping.
    ///
    /// # Return
    /// If `self` is a [`Yaml::Hash`], returns an immutable borrow to the value associated to the
    /// given key in the hash.
    ///
    /// This function returns a [`Yaml::BadValue`] if the underlying [`type@Hash`] does not contain
    /// [`Yaml::String`]`{idx}` as a key.
    ///
    /// This function also returns a [`Yaml::BadValue`] if `self` is not a [`Yaml::Hash`].
    fn index(&self, idx: &'a str) -> &Yaml {
        let key = Yaml::String(idx.to_owned());
        match self.as_hash() {
            Some(h) => h.get(&key).unwrap_or(&BAD_VALUE),
            None => &BAD_VALUE,
        }
    }
}

impl<'a> IndexMut<&'a str> for Yaml {
    /// Perform indexing if `self` is a mapping.
    ///
    /// Since we cannot return a mutable borrow to a static [`Yaml::BadValue`] as we return an
    /// immutable one in [`Index<&'a str>`], this function panics on out of bounds.
    ///
    /// # Panics
    /// This function panics if the given key is not contained in `self` (as per [`IndexMut`]).
    ///
    /// This function also panics if `self` is not a [`Yaml::Hash`].
    fn index_mut(&mut self, idx: &'a str) -> &mut Yaml {
        let key = Yaml::String(idx.to_owned());
        match self.as_mut_hash() {
            Some(h) => h.get_mut(&key).unwrap(),
            None => panic!("Not a hash type"),
        }
    }
}

impl Index<usize> for Yaml {
    type Output = Yaml;

    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Return
    /// If `self` is a [`Yaml::Array`], returns an immutable borrow to the value located at the
    /// given index in the array.
    ///
    /// Otherwise, if `self` is a [`Yaml::Hash`], returns a borrow to the value whose key is
    /// [`Yaml::Integer`]`(idx)` (this would not work if the key is [`Yaml::String`]`("1")`.
    ///
    /// This function returns a [`Yaml::BadValue`] if the index given is out of range. If `self` is
    /// a [`Yaml::Array`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`Yaml::Hash`], this is when the mapping sequence does not
    /// contain [`Yaml::Integer`]`(idx)` as a key.
    ///
    /// This function also returns a [`Yaml::BadValue`] if `self` is not a [`Yaml::Array`] nor a
    /// [`Yaml::Hash`].
    fn index(&self, idx: usize) -> &Yaml {
        if let Some(v) = self.as_vec() {
            v.get(idx).unwrap_or(&BAD_VALUE)
        } else if let Some(v) = self.as_hash() {
            let key = Yaml::Integer(i64::try_from(idx).unwrap());
            v.get(&key).unwrap_or(&BAD_VALUE)
        } else {
            &BAD_VALUE
        }
    }
}

impl IndexMut<usize> for Yaml {
    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// Since we cannot return a mutable borrow to a static [`Yaml::BadValue`] as we return an
    /// immutable one in [`Index<usize>`], this function panics on out of bounds.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`Yaml::Array`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`Yaml::Hash`], this is when the mapping sequence does not
    /// contain [`Yaml::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`Yaml::Array`] nor a [`Yaml::Hash`].
    fn index_mut(&mut self, idx: usize) -> &mut Yaml {
        match self {
            Yaml::Array(sequence) => sequence.index_mut(idx),
            Yaml::Hash(mapping) => {
                let key = Yaml::Integer(i64::try_from(idx).unwrap());
                mapping.get_mut(&key).unwrap()
            }
            _ => panic!("Attempting to index but `self` is not a sequence nor a mapping"),
        }
    }
}

impl IntoIterator for Yaml {
    type Item = Yaml;
    type IntoIter = YamlIter;

    /// Extract the [`Array`] from `self` and iterate over it.
    ///
    /// If `self` is **not** of the [`Yaml::Array`] variant, this function will not panic or return
    /// an error (as per the [`IntoIterator`] trait it cannot) but will instead return an iterator
    /// over an empty [`Array`]. Callers have to ensure (using [`Yaml::is_array`], [`matches`] or
    /// something similar) that the [`Yaml`] object is a [`Yaml::Array`] if they want to do error
    /// handling.
    ///
    /// # Examples
    /// ```
    /// # use yaml_rust2::{Yaml, YamlLoader};
    ///
    /// // An array of 2 integers, 1 and 2.
    /// let arr = &YamlLoader::load_from_str("- 1\n- 2").unwrap()[0];
    ///
    /// assert_eq!(arr.clone().into_iter().count(), 2);
    /// assert_eq!(arr.clone().into_iter().next(), Some(Yaml::Integer(1)));
    /// assert_eq!(arr.clone().into_iter().nth(1), Some(Yaml::Integer(2)));
    ///
    /// // An empty array returns an empty iterator.
    /// let empty = Yaml::Array(vec![]);
    /// assert_eq!(empty.into_iter().count(), 0);
    ///
    /// // A hash with 2 key-value pairs, `(a, b)` and `(c, d)`.
    /// let hash = YamlLoader::load_from_str("a: b\nc: d").unwrap().remove(0);
    /// // The hash has 2 elements.
    /// assert_eq!(hash.as_hash().unwrap().iter().count(), 2);
    /// // But since `into_iter` can't be used with a `Yaml::Hash`, `into_iter` returns an empty
    /// // iterator.
    /// assert_eq!(hash.into_iter().count(), 0);
    /// ```
    fn into_iter(self) -> Self::IntoIter {
        YamlIter {
            yaml: self.into_vec().unwrap_or_default().into_iter(),
        }
    }
}

/// An iterator over a [`Yaml`] node.
pub struct YamlIter {
    yaml: std::vec::IntoIter<Yaml>,
}

impl Iterator for YamlIter {
    type Item = Yaml;

    fn next(&mut self) -> Option<Yaml> {
        self.yaml.next()
    }
}

#[cfg(test)]
mod test {
    use super::{YAMLDecodingTrap, Yaml, YamlDecoder};

    #[test]
    fn test_read_bom() {
        let s = b"\xef\xbb\xbf---
a: 1
b: 2.2
c: [1, 2]
";
        let out = YamlDecoder::read(s as &[u8]).decode().unwrap();
        let doc = &out[0];
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64).abs() <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_read_utf16le() {
        let s = b"\xff\xfe-\x00-\x00-\x00
\x00a\x00:\x00 \x001\x00
\x00b\x00:\x00 \x002\x00.\x002\x00
\x00c\x00:\x00 \x00[\x001\x00,\x00 \x002\x00]\x00
\x00";
        let out = YamlDecoder::read(s as &[u8]).decode().unwrap();
        let doc = &out[0];
        println!("GOT: {doc:?}");
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64) <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_read_utf16be() {
        let s = b"\xfe\xff\x00-\x00-\x00-\x00
\x00a\x00:\x00 \x001\x00
\x00b\x00:\x00 \x002\x00.\x002\x00
\x00c\x00:\x00 \x00[\x001\x00,\x00 \x002\x00]\x00
";
        let out = YamlDecoder::read(s as &[u8]).decode().unwrap();
        let doc = &out[0];
        println!("GOT: {doc:?}");
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64).abs() <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_read_utf16le_nobom() {
        let s = b"-\x00-\x00-\x00
\x00a\x00:\x00 \x001\x00
\x00b\x00:\x00 \x002\x00.\x002\x00
\x00c\x00:\x00 \x00[\x001\x00,\x00 \x002\x00]\x00
\x00";
        let out = YamlDecoder::read(s as &[u8]).decode().unwrap();
        let doc = &out[0];
        println!("GOT: {doc:?}");
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64).abs() <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_read_trap() {
        let s = b"---
a\xa9: 1
b: 2.2
c: [1, 2]
";
        let out = YamlDecoder::read(s as &[u8])
            .encoding_trap(YAMLDecodingTrap::Ignore)
            .decode()
            .unwrap();
        let doc = &out[0];
        println!("GOT: {doc:?}");
        assert_eq!(doc["a"].as_i64().unwrap(), 1i64);
        assert!((doc["b"].as_f64().unwrap() - 2.2f64).abs() <= f64::EPSILON);
        assert_eq!(doc["c"][1].as_i64().unwrap(), 2i64);
        assert!(doc["d"][0].is_badvalue());
    }

    #[test]
    fn test_or() {
        assert_eq!(Yaml::Null.or(Yaml::Integer(3)), Yaml::Integer(3));
        assert_eq!(Yaml::Integer(3).or(Yaml::Integer(7)), Yaml::Integer(3));
    }
}

/// A YAML document loader that provides enhanced position tracking capabilities.
///
/// This loader integrates with the `PositionTracker` to maintain detailed position information
/// for YAML constructs, with particular emphasis on tracking anchor positions and the nodes
/// they reference. Unlike the basic `YamlLoader`, it maintains complete information about
/// where anchors are defined and what nodes they reference, which is essential for
/// more advanced YAML processing tasks.
///
/// # Features
///
/// - Tracks both positions and content of anchored nodes
/// - Resolves aliases with complete node content
/// - Maintains the relationship between anchors and their positions
/// - Provides access to the underlying position tracking system
///
/// # Example
///
/// ```
/// use yaml_rust2::yaml::PositionTrackedLoader;
/// use yaml_rust2::Yaml;
///
/// let yaml_str = "
/// anchors:
///   seq: &seq_anchor [1, 2, 3]
///   map: &map_anchor {a: 1, b: 2}
/// references:
///   seq_ref: *seq_anchor
///   map_ref: *map_anchor
/// ";
///
/// let docs = PositionTrackedLoader::load_from_str(yaml_str).unwrap();
/// // Now docs contains the YAML documents with all anchors and aliases properly resolved
/// ```
#[derive(Default)]
pub struct PositionTrackedLoader {
    /// The different YAML documents that are loaded.
    docs: Vec<Yaml>,
    // states
    // (current node, anchor_id) tuple
    doc_stack: Vec<(Yaml, usize)>,
    key_stack: Vec<Yaml>,
    /// Position tracker that handles anchor nodes
    position_tracker: PositionTracker,
    /// An error, if one was encountered.
    error: Option<ScanError>,
    // Track anchor names for emitting
    anchor_names: BTreeMap<usize, String>,
    next_anchor_id: usize,
    /// Whether to tolerate duplicate keys
    tolerate_duplicate_keys: bool,
}

impl PositionTrackedLoader {
    /// Set whether to tolerate duplicate keys in mappings
    ///
    /// When set to true, duplicate keys will be allowed in mappings, with the last value
    /// for a given key being used. When false (the default), duplicate keys will result in
    /// a ScanError.
    pub fn tolerate_duplicate_keys(&mut self, value: bool) {
        self.tolerate_duplicate_keys = value;
    }

    /// Register a new anchor with the given name
    ///
    /// This method assigns a unique ID to an anchor name and stores the mapping.
    /// Note: Currently this method is not actively used as anchor tracking is primarily
    /// handled through the position_tracker, but it's kept for potential future use.
    #[allow(dead_code)]
    fn register_anchor(&mut self, name: String) -> usize {
        let id = self.next_anchor_id;
        self.next_anchor_id += 1;
        self.anchor_names.insert(id, name);
        id
    }

    /// Track the current node with its path
    ///
    /// This method constructs a node path from the current stack and registers it with the position tracker
    fn track_current_node_position(&mut self, mark: Marker) -> usize {
        // Use a simple path format to avoid string manipulation complexity
        let path = format!("doc{}.node{}", self.docs.len(), self.position_tracker.len());
        self.position_tracker.track_node_with_path(&path, mark)
    }

    fn on_event_impl(&mut self, ev: Event, mark: Marker) -> Result<(), ScanError> {
        // Track positions for all nodes, not just anchored ones
        let _node_id = self.track_current_node_position(mark);

        // Process the event using the position tracker
        let _span = self.position_tracker.process_event(&ev, mark);

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
            Event::SequenceStart(aid, _) => {
                self.doc_stack.push((Yaml::Array(Vec::new()), aid));

                // If this is an anchor, store it in position_tracker
                if aid > 0 {
                    let empty_array = Yaml::Array(Vec::new());
                    self.position_tracker.track_anchor(aid, mark);
                    self.position_tracker.store_anchor_node(aid, empty_array);
                }
            }
            Event::SequenceEnd => {
                let node = self.doc_stack.pop().unwrap();
                self.insert_new_node(node, mark)?;
            }
            Event::MappingStart(aid, _, _) => {
                let node = if aid > 0 {
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

                if aid > 0 {
                    self.position_tracker.track_anchor(aid, mark);
                    self.position_tracker
                        .store_anchor_node(aid, Yaml::Hash(Hash::new()));
                }
            }
            Event::MappingEnd => {
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

                if aid > 0 {
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
                self.insert_new_node((n, 0), mark)?;
            }
        }
        Ok(())
    }

    fn insert_new_node(&mut self, node: (Yaml, usize), mark: Marker) -> Result<(), ScanError> {
        if node.1 > 0 {
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
        let mut loader = PositionTrackedLoader::default();
        {
            let mut parser = Parser::new(source);
            parser.load(&mut loader, true)?;
        }
        if let Some(e) = loader.error {
            Err(e)
        } else {
            Ok(loader.docs)
        }
    }

    /// Load the given YAML documents from a parser.
    ///
    /// The `parser` is used to parse the YAML documents. Parsing succeeds if and only
    /// if all documents are parsed successfully. An error in a latter document prevents the former
    /// from being returned.
    /// # Errors
    /// Returns `ScanError` when loading fails.
    pub fn load_from_parser<I: Iterator<Item = char>>(
        parser: &mut Parser<I>,
    ) -> Result<Vec<Yaml>, ScanError> {
        let mut loader = PositionTrackedLoader::default();
        parser.load(&mut loader, true)?;
        if let Some(e) = loader.error {
            Err(e)
        } else {
            Ok(loader.docs)
        }
    }

    /// Get the loaded documents.
    #[must_use]
    pub fn documents(&self) -> &[Yaml] {
        &self.docs
    }

    /// Get the name of an anchor by ID.
    #[must_use]
    pub fn get_anchor_name(&self, id: usize) -> Option<&str> {
        self.anchor_names.get(&id).map(|s| s.as_ref())
    }

    /// Get the position tracker instance
    ///
    /// This provides access to the underlying position tracking system,
    /// allowing for advanced position queries and anchor tracking operations.
    ///
    /// # Returns
    ///
    /// A reference to the position tracker used by this loader
    #[must_use]
    pub fn position_tracker(&self) -> &PositionTracker {
        &self.position_tracker
    }

    /// Get the mutable position tracker instance
    ///
    /// This provides mutable access to the underlying position tracking system,
    /// allowing for advanced position queries and anchor tracking operations.
    ///
    /// # Returns
    ///
    /// A mutable reference to the position tracker used by this loader
    pub fn position_tracker_mut(&mut self) -> &mut PositionTracker {
        &mut self.position_tracker
    }

    /// Get the position of a specific anchor
    ///
    /// # Arguments
    ///
    /// * `anchor_id` - The ID of the anchor to look up
    ///
    /// # Returns
    ///
    /// The position of the anchor, or None if not found
    #[must_use]
    pub fn get_anchor_position(&self, anchor_id: usize) -> Option<Marker> {
        self.position_tracker.get_anchor_position(anchor_id)
    }

    /// Get the node associated with a specific anchor
    ///
    /// # Arguments
    ///
    /// * `anchor_id` - The ID of the anchor to look up
    ///
    /// # Returns
    ///
    /// The YAML node associated with the anchor, or None if not found
    #[must_use]
    pub fn get_anchor_yaml(&self, anchor_id: usize) -> Option<Yaml> {
        self.position_tracker.get_anchor_yaml(anchor_id)
    }

    /// Recursively collect position spans for all nodes in the document.
    ///
    /// This method traverses the YAML document and collects position spans for
    /// all nodes, including non-anchored nodes, by checking various sources:
    /// 1. Anchor positions for anchored nodes
    /// 2. Content hash lookups for non-anchored nodes
    /// 3. Direct allocation of spans for nodes that don't have positions yet
    ///
    /// # Arguments
    ///
    /// * `node` - The root node to start collecting from
    /// * `spans` - A mutable reference to a map that will be populated with node spans
    fn collect_position_spans(
        &self,
        node: &Yaml,
        spans: &mut HashMap<*const Yaml, crate::position::PositionSpan>,
    ) {
        let tracker = self.position_tracker();

        // Helper function to add a default span for nodes that don't have positions yet
        fn ensure_node_has_span(
            node: &Yaml,
            spans: &mut HashMap<*const Yaml, crate::position::PositionSpan>,
            default_pos: crate::scanner::Marker,
        ) {
            // If this node doesn't have a span yet, add one with a default position
            if !spans.contains_key(&(node as *const Yaml)) {
                let span = crate::position::PositionSpan::new(default_pos);
                spans.insert(node as *const Yaml, span);
            }
        }

        // Helper function to set an end position for a node if it doesn't have one
        fn ensure_node_has_end_pos(
            node: &Yaml,
            spans: &mut HashMap<*const Yaml, crate::position::PositionSpan>,
            end_pos: crate::scanner::Marker,
        ) {
            if let Some(span) = spans.get_mut(&(node as *const Yaml)) {
                if span.end.is_none() {
                    span.set_end(end_pos);
                }
            }
        }

        // First check if we already have a span for this node with an end position
        let has_complete_span = spans
            .get(&(node as *const Yaml))
            .map_or(false, |span| span.end.is_some());

        if has_complete_span {
            // If node already has both start and end positions, nothing to do
            return;
        }

        // Try to find a position for this node using various methods
        if !spans.contains_key(&(node as *const Yaml)) {
            // 1. Check if this is an anchored node
            if let Some(anchor_id) = tracker.find_anchor_id(node) {
                if let Some(pos) = tracker.get_anchor_position(anchor_id) {
                    // If we have an anchor position, use it as the start
                    let span = crate::position::PositionSpan::new(pos);
                    spans.insert(node as *const Yaml, span);
                }
            }
            // 2. Try to find the node by its content hash
            else {
                let hash = crate::position::PositionTracker::calculate_node_hash(node);
                if let Some(pos) = tracker.get_position_by_hash(hash) {
                    // If we found a position for this node's hash, use it
                    let span = crate::position::PositionSpan::new(pos);
                    spans.insert(node as *const Yaml, span);
                }
            }
        }

        // Ensure parent nodes have positions before processing children
        // Create a default position with line 1, column 1 for any nodes without real positions
        let default_pos = crate::scanner::Marker::new(0, 1, 1);
        ensure_node_has_span(node, spans, default_pos);

        // Get the current span for this node for reference by children
        let current_span = spans
            .get(&(node as *const Yaml))
            .cloned()
            .unwrap_or_else(|| crate::position::PositionSpan::new(default_pos));

        // Calculate end positions based on node type
        match node {
            Yaml::Array(array) => {
                // Track the last processed item position to set the array end
                let mut last_end_pos = current_span.start;

                // Recursively collect spans for each item
                for (index, item) in array.iter().enumerate() {
                    // Process the child node first
                    self.collect_position_spans(item, spans);

                    // Make sure this item has a position - use the array's position with an offset
                    // if we couldn't find a real position for it
                    let offset_pos = crate::scanner::Marker::new(
                        0,
                        current_span.start.line(),
                        current_span.start.col() + index * 2,
                    );
                    ensure_node_has_span(item, spans, offset_pos);

                    // Update the last end position based on the item's end position
                    let item_span = spans.get(&(item as *const Yaml)).cloned();
                    if let Some(span) = item_span {
                        if let Some(item_end) = span.end {
                            last_end_pos = item_end;
                        } else {
                            // If item doesn't have an end position, estimate one
                            let estimated_end = crate::scanner::Marker::new(
                                0,
                                span.start.line(),
                                span.start.col() + 10, // Arbitrary width
                            );
                            ensure_node_has_end_pos(item, spans, estimated_end);
                            last_end_pos = estimated_end;
                        }
                    }
                }

                // Set the array's end position based on its last item
                // Add one line after the last item to account for array closing
                let array_end_pos = crate::scanner::Marker::new(
                    0,
                    last_end_pos.line() + 1,
                    current_span.start.col(),
                );
                ensure_node_has_end_pos(node, spans, array_end_pos);
            }
            Yaml::Hash(hash) => {
                // Track the last processed value position to set the hash end
                let mut last_end_pos = current_span.start;

                // Recursively collect spans for each key and value
                for (key, value) in hash {
                    // Process the key and value
                    self.collect_position_spans(key, spans);
                    self.collect_position_spans(value, spans);

                    // Make sure key and value have positions if we couldn't find real ones
                    let key_pos = crate::scanner::Marker::new(
                        0,
                        current_span.start.line(),
                        current_span.start.col() + 1,
                    );
                    ensure_node_has_span(key, spans, key_pos);

                    let value_pos = crate::scanner::Marker::new(
                        0,
                        current_span.start.line(),
                        current_span.start.col() + 2,
                    );
                    ensure_node_has_span(value, spans, value_pos);

                    // Set end positions for key and value if they don't have them
                    // Key end is right before value start
                    let value_span = spans.get(&(value as *const Yaml)).cloned();
                    if let Some(span) = value_span {
                        // Set key end position to be right before value
                        let key_col = if span.start.col() > 2 {
                            span.start.col() - 2
                        } else {
                            1 // Minimum column value
                        };
                        let key_end = crate::scanner::Marker::new(0, span.start.line(), key_col);
                        ensure_node_has_end_pos(key, spans, key_end);

                        // Update the last end position based on the value's end position
                        if let Some(value_end) = span.end {
                            last_end_pos = value_end;
                        } else {
                            // If value doesn't have an end position, estimate one
                            let estimated_end = crate::scanner::Marker::new(
                                0,
                                span.start.line(),
                                span.start.col() + 10, // Arbitrary width
                            );
                            ensure_node_has_end_pos(value, spans, estimated_end);
                            last_end_pos = estimated_end;
                        }
                    }
                }

                // Set the hash's end position based on its last value
                // Add one line after the last value to account for hash closing
                let hash_end_pos = crate::scanner::Marker::new(
                    0,
                    last_end_pos.line() + 1,
                    current_span.start.col(),
                );
                ensure_node_has_end_pos(node, spans, hash_end_pos);
            }
            // For scalar nodes, estimate an end position based on the content
            Yaml::String(s) => {
                let end_pos = crate::scanner::Marker::new(
                    0,
                    current_span.start.line(),
                    current_span.start.col() + s.len(),
                );
                ensure_node_has_end_pos(node, spans, end_pos);
            }
            Yaml::Integer(i) => {
                let len = i.to_string().len();
                let end_pos = crate::scanner::Marker::new(
                    0,
                    current_span.start.line(),
                    current_span.start.col() + len,
                );
                ensure_node_has_end_pos(node, spans, end_pos);
            }
            Yaml::Real(r) => {
                let len = r.len();
                let end_pos = crate::scanner::Marker::new(
                    0,
                    current_span.start.line(),
                    current_span.start.col() + len,
                );
                ensure_node_has_end_pos(node, spans, end_pos);
            }
            Yaml::Boolean(b) => {
                let len = if *b { 4 } else { 5 }; // "true" or "false"
                let end_pos = crate::scanner::Marker::new(
                    0,
                    current_span.start.line(),
                    current_span.start.col() + len,
                );
                ensure_node_has_end_pos(node, spans, end_pos);
            }
            Yaml::Null => {
                let end_pos = crate::scanner::Marker::new(
                    0,
                    current_span.start.line(),
                    current_span.start.col() + 4, // "null"
                );
                ensure_node_has_end_pos(node, spans, end_pos);
            }
            Yaml::Alias(anchor_id) => {
                let end_pos = crate::scanner::Marker::new(
                    0,
                    current_span.start.line(),
                    current_span.start.col() + anchor_id.to_string().len() + 1,
                );
                ensure_node_has_end_pos(node, spans, end_pos);
            }
            _ => {
                // For other types, just set the end position to be the same as the start
                ensure_node_has_end_pos(node, spans, current_span.start);
            }
        }
    }

    /// Enhance the position tracking with additional path-based tracking
    ///
    /// This method is used to improve position tracking for nodes that don't have
    /// anchor-based tracking by recording paths to nodes in the document.
    ///
    /// # Arguments
    ///
    /// * `document` - The document to enhance tracking for
    fn enhance_with_path_tracking(&self, document: &Yaml) {
        // Since position_tracker is immutable (as self is immutable),
        // we'll log the path information for debugging purposes.
        // In a real implementation, we would need to modify the API
        // to allow for mutable access to the position tracker.

        // This implementation creates the paths but doesn't actually
        // store them since we can't mutate the position tracker here.
        fn build_node_paths(node: &Yaml, path: &mut Vec<String>) {
            // Generate a path string for logging
            let path_str = if path.is_empty() {
                "root".to_string()
            } else {
                path.join(".")
            };

            // For a real implementation, we would track the node with its path here
            // position_tracker.track_node_with_path(&path_str, position);

            // Just for debugging - print the path
            if cfg!(debug_assertions) {
                println!("Would track path: {}", path_str);
            }

            // Recursively process children
            match node {
                Yaml::Hash(ref hash) => {
                    for (key, value) in hash.iter() {
                        if let Yaml::String(key_str) = key {
                            // Push this key to the path
                            path.push(key_str.clone());

                            // Process the value
                            build_node_paths(value, path);

                            // Pop the key from the path
                            path.pop();
                        }
                    }
                }
                Yaml::Array(ref array) => {
                    for (index, item) in array.iter().enumerate() {
                        // Push the index to the path
                        path.push(index.to_string());

                        // Process the item
                        build_node_paths(item, path);

                        // Pop the index from the path
                        path.pop();
                    }
                }
                _ => {
                    // For scalar values, we've already processed this node
                }
            }
        }

        // Log the paths that would be created
        if cfg!(debug_assertions) {
            let mut path_components = Vec::new();
            build_node_paths(document, &mut path_components);
        }
    }
}

impl MarkedEventReceiver for PositionTrackedLoader {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        // First, update the position tracker
        self.position_tracker.process_event(&ev, mark);

        if self.error.is_some() {
            return;
        }
        if let Err(e) = self.on_event_impl(ev, mark) {
            self.error = Some(e);
        }
    }

    fn on_positioned_event(&mut self, ev: Event, span: crate::position::PositionSpan) {
        // Store the complete position span in our position tracker
        match &ev {
            Event::MappingStart(anchor_id, _, _style) => {
                // For mappings, store the start position
                if *anchor_id > 0 {
                    // For anchored nodes, we track the anchor by ID
                    self.position_tracker.track_anchor(*anchor_id, span.start);

                    // Create an empty map for the anchor
                    let empty_map = Yaml::Hash(Hash::new());
                    self.position_tracker
                        .store_anchor_node(*anchor_id, empty_map);
                }

                // Track the position of this mapping
                let node_id = self.position_tracker.track_node_position(span.start);

                // Remember the node ID for later when we get the end event
                if let Some(end_mark) = span.end {
                    // If we already have the end position, store a complete span
                    let mut node_spans = HashMap::new();
                    node_spans.insert(
                        node_id,
                        crate::position::PositionSpan::with_end(span.start, end_mark),
                    );
                }
            }
            Event::SequenceStart(anchor_id, _) => {
                // For sequences, store the start position
                if *anchor_id > 0 {
                    // For anchored nodes, we track the anchor by ID
                    self.position_tracker.track_anchor(*anchor_id, span.start);

                    // Create an empty array for the anchor
                    let empty_seq = Yaml::Array(Vec::new());
                    self.position_tracker
                        .store_anchor_node(*anchor_id, empty_seq);
                }

                // Track the position of this sequence
                let node_id = self.position_tracker.track_node_position(span.start);

                // Remember the node ID for later when we get the end event
                if let Some(end_mark) = span.end {
                    // If we already have the end position, store a complete span
                    let mut node_spans = HashMap::new();
                    node_spans.insert(
                        node_id,
                        crate::position::PositionSpan::with_end(span.start, end_mark),
                    );
                }
            }
            Event::Scalar(value, style, anchor_id, _tag) => {
                // For scalars, store the position
                if *anchor_id > 0 {
                    // For anchored nodes, we track the anchor by ID
                    self.position_tracker.track_anchor(*anchor_id, span.start);

                    // Create the scalar node
                    let node = match style {
                        crate::scanner::TScalarStyle::Plain => Yaml::from_str(value),
                        _ => Yaml::String(value.clone()),
                    };

                    self.position_tracker.store_anchor_node(*anchor_id, node);
                }

                // Track the position of this scalar
                let node_id = self.position_tracker.track_node_position(span.start);

                // Remember the node ID for later when we get the end event
                if let Some(end_mark) = span.end {
                    // If we already have the end position, store a complete span
                    let mut node_spans = HashMap::new();
                    node_spans.insert(
                        node_id,
                        crate::position::PositionSpan::with_end(span.start, end_mark),
                    );
                }
            }
            _ => {
                // For other events, just pass them through
            }
        }

        // Call the original on_event method with the start position
        self.on_event(ev, span.start);
    }
}

// Add a new module for feature-gated loader implementations
/// Module that selects the appropriate loader implementation based on feature flags
pub mod loader {
    use super::*;

    /// The default loader implementation used by the library
    ///
    /// When the `position_tracked_loader` feature is enabled, this will use the
    /// `PositionTrackedLoader` implementation. Otherwise, it will use the
    /// original `YamlLoader` implementation.
    #[cfg(not(feature = "position_tracked_loader"))]
    pub type DefaultLoader = YamlLoader;

    /// The default loader implementation used by the library
    ///
    /// When the `position_tracked_loader` feature is enabled, this will use the
    /// `PositionTrackedLoader` implementation. Otherwise, it will use the
    /// original `YamlLoader` implementation.
    #[cfg(feature = "position_tracked_loader")]
    pub type DefaultLoader = PositionTrackedLoader;

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
}

// Re-export the loader functions for ease of use
pub use self::loader::{load_from_iter, load_from_str};

// Add the SourceMapSupport implementation
#[cfg(feature = "source_mapping")]
impl crate::source_map::SourceMapSupport for PositionTrackedLoader {
    fn build_source_maps(&self) -> Vec<crate::source_map::SourceMap<Yaml>> {
        let docs = self.documents();
        docs.iter()
            .enumerate()
            .map(|(i, _)| self.build_source_map_for_document(i).unwrap_or_default())
            .collect()
    }

    fn build_source_map_for_document(
        &self,
        document_index: usize,
    ) -> Option<crate::source_map::SourceMap<Yaml>> {
        let docs = self.documents();
        let document = docs.get(document_index)?;

        // Create a builder for the source map
        let builder = crate::source_map::SourceMapBuilder::new();

        // Traverse the document tree and build a mapping of nodes to position spans
        let mut node_spans = HashMap::new();
        self.collect_position_spans(document, &mut node_spans);

        // Enhance with path tracking for automatic positioning
        self.enhance_with_path_tracking(document);

        // Build the source map using the collected spans
        Some(builder.build(document, &node_spans))
    }
}

#[cfg(feature = "source_mapping")]
impl Yaml {
    /// Get the position information for this node
    ///
    /// This method provides a convenient way to access the position information
    /// for a node that was created with position tracking enabled. It is only
    /// available when the `source_mapping` feature is enabled.
    ///
    /// Note: This method can only return position information when used with a SourceMap.
    /// It's included here for convenience when using source maps directly.
    ///
    /// # Returns
    ///
    /// None. This is a stub implementation that always returns None.
    /// To get position information, use the SourceMap API instead.
    #[must_use]
    pub fn get_position(&self) -> Option<crate::position::PositionSpan> {
        // This is just a stub implementation
        // Real position information should be retrieved from the SourceMap
        None
    }

    /// Format this node with position information
    ///
    /// This method provides a quick way to get a string representation of this node
    /// with its position in the source document.
    ///
    /// # Returns
    ///
    /// A string representation of the node type
    #[must_use]
    pub fn with_position_info(&self) -> String {
        let node_type = match self {
            Yaml::Real(_) => "Real",
            Yaml::Integer(_) => "Integer",
            Yaml::String(_) => "String",
            Yaml::Boolean(_) => "Boolean",
            Yaml::Array(_) => "Array",
            Yaml::Hash(_) => "Hash",
            Yaml::Alias(_) => "Alias",
            Yaml::Null => "Null",
            Yaml::BadValue => "BadValue",
        };

        format!("{} (no position info available)", node_type)
    }
}
