# Block Mappings in YAML

Block mappings are a fundamental component of YAML, providing a structured way to represent key-value pairs in a human-readable format. This document describes how block mappings work in YAML 1.2, with special attention to how they are handled in the yaml-rust2 library, particularly for position tracking and duplicate key handling.

## Definition

According to the YAML 1.2.2 specification, a block mapping is "a series of entries, each presenting a key/value pair." Block mappings associate keys with values within a YAML document.

The specification notes that:
- Block mappings rely on indentation to determine their structure
- Keys are separated from values by a colon and space (`: `)
- Each entry shares the same indentation level
- Nested mappings use deeper indentation levels
- Block mappings can use both implicit and explicit entry styles

A key feature of block mappings is their readability, making them easy for humans to understand and modify.

In YAML 1.2, a block mapping is represented as a series of key-value pairs where:
- Each key-value pair begins on a new line
- Keys are separated from values by a colon and space (`: `)
- Each entry shares the same indentation level
- Nested mappings use deeper indentation levels

## Syntax Rules

### Basic Syntax

```yaml
key1: value1
key2: value2
key3: value3
```

### Nested Mappings

Block mappings can be nested by increasing the indentation level:

```yaml
top_level:
  nested_key1: nested_value1
  nested_key2: nested_value2
another_key: value
```

### Complex Values

Block mappings can have values that are other YAML constructs:

```yaml
key_with_sequence:
  - item1
  - item2
  - item3

key_with_mapping:
  nested_key1: value1
  nested_key2: value2

key_with_multiline_scalar: |
  This is a multiline string
  that preserves newlines
  exactly as written.
```

### Empty Values

Keys can have empty values:

```yaml
empty_key:
key_with_value: value
```

## Entry Types in Block Mappings

The YAML 1.2.2 specification defines two types of entries in block mappings:

### Implicit Block Mapping Entries

Implicit entries are the most common form. They don't use any special indicator before the key:

```yaml
plain key: in-line value
empty key: # Value is null
"quoted key": any value
```

In implicit entries, the key must be a single-line node.

### Explicit Block Mapping Entries

If the `?` indicator is specified, the mapping uses an explicit key:

```yaml
? explicit key # Empty value
? |
  block key
: - one       # Explicit compact
  - two       # block value
```

According to the specification, when the `?` indicator is explicitly specified, the optional value node must be specified on a separate line, denoted by the `:` indicator. This allows complex multi-line structures to be used as keys.

## Edge Cases and Restrictions

### Compact Notation for Nested Mappings

The YAML 1.2.2 specification notes that a compact in-line notation is available for block mappings within sequences:

```yaml
# Compact notation for mappings inside sequences
- sun: yellow
- ? earth: blue
  : moon: white
```

In this example, the compact notation is used to create:
1. A simple key-value pair (sun: yellow)
2. A complex mapping where both the key and value are themselves mappings

According to the specification, compact mappings can be used as entries in block sequences or as values in block mappings.

### Empty Mappings

An empty mapping can be represented by an empty node:

```yaml
empty_mapping: {}
```

The specification doesn't provide a block-style representation of an empty mapping, so flow style is typically used.

### Keys with Special Characters

If keys contain special characters or could be misinterpreted as other YAML types, they should be quoted:

```yaml
"key:with:colons": value
"1.2.3": version number as a key
"true": boolean-like string as key
```

### Duplicate Keys

According to the YAML 1.2 specification, mappings should have unique keys. However, in practice, many YAML parsers handle duplicate keys differently:

1. Some parsers (including older versions of yaml-rust) raise an error
2. Some use the first occurrence of a key
3. Some use the last occurrence (overwriting previous values)

The yaml-rust2 library now offers configurable behavior for duplicate keys via the `tolerate_duplicate_keys` option:

```rust
// Use last value for duplicate keys
let mut parser = Parser::new(yaml_str.chars());
parser.set_tolerate_duplicate_keys(true);
let result = YamlLoader::load_from_parser(&mut parser);
```

When `tolerate_duplicate_keys` is set to `true`, later occurrences of a key will overwrite earlier ones. When set to `false` (default), a duplicate key will result in a `ScanError`.

## Position Tracking

In yaml-rust2, position tracking for block mappings is a key feature that helps with error reporting and document analysis. For block mappings, positions are tracked as follows:

### Start Position

The start position of a block mapping is at the first line/character of the first key. For example, in:

```yaml
mapping:
  key1: value1
  key2: value2
```

The start position of the block mapping is at the `k` in `key1`.

### End Position

The end position of a block mapping is at the last line/character of the last value. In the example above, the end position would be at the end of `value2`.

### Implementation Details

Position tracking is implemented in the `PositionTracker` class, which:

1. Tracks the start of a block mapping when a `MappingStart` event is processed
2. Pushes the event onto a stack with a unique ID (2 for block mappings, 0 for flow mappings)
3. Tracks the end of a block mapping when a `MappingEnd` event is processed
4. Creates a `PositionSpan` with both start and end positions

In the yaml-rust2 implementation, a key focus has been on improving position tracking accuracy, especially for complex structures like nested mappings and flow collections.

## Interaction with Other YAML Features

Block mappings can interact with various other YAML features:

### Anchors and Aliases

Mappings can be anchored and referenced elsewhere in the document:

```yaml
mapping: &anchor
  key1: value1
  key2: value2

reference: *anchor  # This will be a copy of the 'mapping' node
```

yaml-rust2 tracks anchors through the `PositionTracker`, which maintains mappings between anchor IDs and node positions. This ensures that both the original mapping and its references can be properly located in the source document.

### Tags

Mappings can have explicit type tags:

```yaml
!!map
  key1: value1
  key2: value2
```

### Merge Key (`<<`)

YAML 1.1 and 1.2 support the merge key for combining mappings:

```yaml
defaults: &defaults
  adapter: postgres
  host: localhost

development:
  <<: *defaults
  port: 5432
```

## Common Parsing Challenges

### Indentation Sensitivity

Block mappings are highly sensitive to indentation. Inconsistent indentation can lead to parsing errors or unexpected structure:

```yaml
correct:
  key1: value1
  key2: value2

incorrect:
  key1: value1
 key2: value2  # Wrong indentation will cause parsing errors
```

### Empty or Null Values

Distinguishing between empty strings, null values, and missing values requires careful attention:

```yaml
empty_string: ""
null_value: null
missing_value:  # This is actually a null value in YAML
```

### Complex Nesting

Deeply nested structures can be confusing to parse and track positions for:

```yaml
level1:
  level2:
    level3:
      level4:
        key: value
```

The yaml-rust2 library uses a stack-based approach to maintain the hierarchy of mappings and their positions.

### Duplicate Keys Handling

As mentioned earlier, duplicate keys present a challenge for YAML parsers. The YAML 1.2 specification states that map keys should be unique, but implementations differ.

In yaml-rust2, the approach has evolved:

```rust
// In the insert_new_node method:
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
```

This allows for more flexible handling of duplicate keys, which is useful for applications that need to process malformed YAML documents.

## Best Practices

For working with block mappings in yaml-rust2:

1. **Use the Position Tracking API** when you need to locate nodes in the original source, especially for error reporting.

2. **Configure Duplicate Key Handling** based on your application's needs:
   ```rust
   parser.set_tolerate_duplicate_keys(true); // If you want to accept and use the last value
   ```

3. **Check for Start and End Positions** - Some complex structures may have incomplete position information:
   ```rust
   if let Some(end) = position_span.end {
       // Use end position
   } else {
       // Handle missing end position
   }
   ```

4. **Watch Indentation** - Especially when programmatically generating YAML, ensure consistent indentation for block mappings.

## Conclusion

Block mappings are a powerful and readable way to represent key-value data in YAML. The yaml-rust2 library provides robust support for parsing and position tracking of block mappings, with special attention to handling edge cases like duplicate keys. 

Understanding how block mappings are represented and tracked in the source document enables more effective error reporting and document processing in applications that use YAML for configuration, data exchange, or other purposes.