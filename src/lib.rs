// Copyright 2015, Yuheng Chen.
// Copyright 2023, Ethiraric.
// See the LICENSE file at the top-level directory of this distribution.

//! YAML 1.2 implementation in pure Rust.
//!
//! # Usage
//!
//! This crate is [on github](https://github.com/Ethiraric/yaml-rust2) and can be used by adding
//! `yaml-rust2` to the dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! yaml-rust2 = "0.10.0"
//! ```
//!
//! # Examples
//! Parse a string into `Vec<Yaml>` and then serialize it as a YAML string.
//!
//! ```
//! use yaml_rust2::{YamlLoader, YamlEmitter};
//!
//! let docs = YamlLoader::load_from_str("[1, 2, 3]").unwrap();
//! let doc = &docs[0]; // select the first YAML document
//! assert_eq!(doc[0].as_i64().unwrap(), 1); // access elements by index
//!
//! let mut out_str = String::new();
//! let mut emitter = YamlEmitter::new(&mut out_str);
//! emitter.dump(doc).unwrap(); // dump the YAML object to a String
//!
//! ```
//!
//! You can also use the feature-gated loader API:
//!
//! ```
//! use yaml_rust2::yaml::loader;
//!
//! // This uses the default loader implementation which depends on feature flags
//! let docs = loader::load_from_str("[1, 2, 3]").unwrap();
//! let doc = &docs[0];
//! assert_eq!(doc[0].as_i64().unwrap(), 1);
//! ```
//!
//! # Features
//! **Note:** With all features disabled, this crate's MSRV is `1.65.0`.
//!
//! #### `encoding` (_enabled by default_)
//! Enables encoding-aware decoding of Yaml documents.
//!
//! The MSRV for this feature is `1.70.0`.
//!
//! #### `debug_prints`
//! Enables the `debug` module and usage of debug prints in the scanner and the parser. Do not
//! enable if you are consuming the crate rather than working on it as this can significantly
//! decrease performance.
//!
//! The MSRV for this feature is `1.70.0`.
//!
//! #### `position_tracked_loader`
//! Enables the position-tracked loader implementation that uses the `PositionTracker` for
//! managing anchors and references instead of using the internal anchor map. This provides
//! better position tracking for complex YAML documents with anchors and references.
//!
//! When this feature is enabled, the default loader type (`yaml::loader::DefaultLoader`) will
//! be the `PositionTrackedLoader` instead of the original `YamlLoader`.
//!
//! #### `source_mapping`
//! Enables source mapping capabilities that provide bidirectional mapping between YAML nodes
//! and their source positions in the original document. This is particularly useful for error
//! reporting, debugging, and tools that need to highlight specific sections of YAML documents.
//!
//! This feature requires the `position_tracked_loader` feature to be enabled, as it builds on
//! the position tracking functionality to provide source mapping capabilities.
//!
//! ```
//! // Example of using source mapping (requires both position_tracked_loader and source_mapping features)
//! use yaml_rust2::{PositionTrackedLoader, source_map::SourceMapSupport};
//!
//! let source = "key: value\nlist:\n  - item1\n  - item2";
//! let loader = PositionTrackedLoader::load_from_str(source).unwrap();
//! let source_maps = loader.build_source_maps();
//!
//! // Now you can find nodes by position
//! let node_id = source_maps[0].find_node_at_position(1, 1);
//! ```

#![warn(missing_docs, clippy::pedantic)]

extern crate hashlink;

pub(crate) mod char_traits;
#[macro_use]
pub(crate) mod debug;
pub mod emitter;
pub mod parser;
pub mod position;
pub mod scanner;
#[cfg(feature = "source_mapping")]
pub mod source_map;
pub mod yaml;

// reexport key APIs
pub use crate::emitter::{EmitError, YamlEmitter};
pub use crate::parser::Event;
pub use crate::position::PositionSpan;
pub use crate::scanner::ScanError;
#[cfg(feature = "source_mapping")]
pub use crate::source_map::{
    NodeId, SourceLocation, SourceMap, SourceMapBuilder, SourceMapSupport,
};
pub use crate::yaml::loader::{load_from_iter, load_from_str};
pub use crate::yaml::{PositionTrackedLoader, Yaml, YamlLoader};
