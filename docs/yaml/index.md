# YAML 1.2 Constructs Documentation

This directory contains comprehensive documentation on various YAML 1.2 constructs and how they are implemented in the yaml-rust2 library, with a particular focus on position tracking.

## Document Structure

- [YAML Streams](stream.md) - Multiple documents in a single input
- [YAML Documents](document.md) - Structure and semantics of individual documents
- [YAML Directives](directives.md) - Document-level instructions like %YAML and %TAG

## Scalar Types

- [Scalars Overview](scalars.md) - General information about all scalar types
- [Flow Scalars](flow_scalars.md) - Plain, single-quoted, and double-quoted styles
- [Block Scalars](block_scalars.md) - Literal and folded styles with chomping controls

## Collection Types

- [Block Sequences](block_sequences.md) - Lists using the dash notation with indentation
- [Flow Sequences](flow_sequences.md) - Lists using square brackets and commas
- [Block Mappings](block_mappings.md) - Key-value pairs using indentation
- [Flow Mappings](flow_mappings.md) - Key-value pairs using curly braces and commas

## Reuse Mechanisms

- [Anchors and Aliases](anchors_aliases.md) - Mechanisms for reusing content in YAML

## Type and Annotation Features

- [YAML Tags](tags.md) - Type annotations for nodes

## Additional Features

- [YAML Comments](comments.md) - Future support for preserving comments

## Position Tracking

Each document describes how position tracking is implemented for the specific YAML construct:

- Start positions (line, column, index)
- End positions (line, column, index)
- Special handling for different constructs
- Edge cases and challenges

## Known Issues

Some documents reference current implementation challenges in yaml-rust2, particularly:

- Flow collection position tracking issues
- Anchor resolution and pointer equality problems 
- Duplicate key handling
- Start and end position calculations

These issues are actively being worked on in the yaml-rust2 library.

## Further Reading

For more information on position tracking in yaml-rust2, see:

- [Position Tracked Loader](../position_tracked_loader.md)
- [Source Mapping](../source_mapping.md)