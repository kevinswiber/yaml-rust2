# Flow Mappings in YAML 1.2

Flow mappings are a compact, inline representation of key-value pairs in YAML 1.2. They provide a JSON-like syntax for defining collections of key-value pairs, making them familiar to developers who work with multiple data serialization formats.

## Definition and Basic Syntax

According to the YAML 1.2.2 specification, flow mappings are denoted by surrounding `{` and `}` characters. They are collections of key-value pairs that use a more compact, inline representation compared to block mappings.

The basic syntax is:

```yaml
# Basic flow mapping syntax
{key1: value1, key2: value2}
```

Flow mappings can be used anywhere a block mapping can be used, but they take up less vertical space, making them ideal for compact data representation.

The specification defines:
- Mapping entries are separated by a `,` character
- A trailing comma is allowed after the last entry
- Whitespace around separators is optional 
- Keys and values are separated by colons `:`
- A flow mapping can be completely empty (e.g., `{}`)

## Syntax Rules

### Basic Rules

1. Flow mappings are enclosed in curly braces `{` and `}`
2. Key-value pairs are separated by colons `:` 
3. Multiple entries are separated by commas `,`
4. A trailing comma after the last entry is allowed
5. Whitespace around separators is optional

```yaml
# All of these are valid flow mappings
{key1:value1,key2:value2}
{ key1: value1, key2: value2 }
{key1: value1, key2: value2,}  # Trailing comma is allowed
```

### Key Formatting

In flow mappings, keys can be:

1. Plain (unquoted) strings
2. Single-quoted strings
3. Double-quoted strings
4. Flow nodes (such as other flow mappings or sequences)

```yaml
# Different key formats
{
  plain_key: value1,          # Plain (unquoted) key
  'quoted key': value2,       # Single-quoted key (allows spaces)
  "quoted key": value3,       # Double-quoted key (allows spaces and escape sequences)
  ? {nested: key}: value4     # Flow mapping as a key (with explicit ? indicator)
}
```

### Value Formatting

Values in flow mappings can be any valid YAML node:

1. Scalar values (strings, numbers, booleans, null)
2. Flow sequences `[item1, item2]`
3. Nested flow mappings `{nested_key: nested_value}`
4. Anchors and aliases

```yaml
# Different value formats
{
  key1: plain string value,       # Plain scalar
  key2: 42,                       # Integer
  key3: true,                     # Boolean
  key4: [1, 2, 3],                # Flow sequence
  key5: {nested: value},          # Nested flow mapping
  key6: &anchor value,            # Anchored value
  key7: *anchor                   # Alias reference
}
```

## Edge Cases and Restrictions

### Empty Flow Mappings

An empty flow mapping is represented as an empty pair of curly braces:

```yaml
empty_mapping: {}
```

### Omitted Values

In flow mappings, values can be omitted, which will be interpreted as `null`:

```yaml
{
  key_with_value: value,
  key_without_value:,           # Value is null
  trailing_key_without_value:   # Value is null (trailing comma omitted)
}
```

### Complex Keys and Explicit Entries

For complex keys, the YAML 1.2.2 specification requires the explicit key indicator `?`:

```yaml
{
  ? [complex, key, as, sequence]: value,
  ? {nested: mapping}: another_value
}
```

The specification distinguishes between explicit and implicit entries in flow mappings:

1. **Explicit entries** - Use the `?` indicator for the key
2. **Implicit entries** - Direct key-value pairs without the `?` indicator

If the `?` indicator is explicitly specified, parsing is unambiguous. This allows for very complex structures to be used as keys.

### Nested Collections

Flow mappings can contain nested collections, including other flow mappings and flow sequences:

```yaml
complex_mapping: {
  sequence: [1, 2, {key: value}],
  mapping: {a: 1, b: {c: 3}}
}
```

### Line Breaks in Flow Mappings

Flow mappings can span multiple lines for better readability:

```yaml
multiline_mapping: {
  key1: value1,
  key2: value2,
  key3: [
    item1,
    item2
  ]
}
```

### Value Handling

The YAML 1.2.2 specification defines several ways values can be represented in flow mappings:

1. **Separate Values**: Using a `:` with a separate value node
   ```yaml
   {
     key: value,
     another: separate value
   }
   ```

2. **Adjacent Values**: For JSON compatibility, when keys are JSON-like (quoted strings, etc.), YAML allows the value to be specified immediately after the `:` 
   ```yaml
   {
     "adjacent":value,
     "key":immediate
   }
   ```
   However, for better readability, the specification recommends separating the value from the `:`, even in this case.

3. **Empty Values**: Empty or omitted values become `null`
   ```yaml
   {
     empty:,
     also_empty:
   }
   ```

4. **Omitted Keys**: Similarly, an omitted key with a value becomes a mapping with `null` as the key
   ```yaml
   {
     : value_with_null_key
   }
   ```

### Duplicate Keys

YAML 1.2 specification allows duplicate keys in mappings, but how they are handled depends on the implementation:

- The yaml-rust2 implementation currently has ongoing work to tolerate duplicate keys
- By default, the last key-value pair with a given key will override previous occurrences
- Position tracking will track both occurrences, allowing applications to detect and handle duplicates

The specific behavior with duplicate keys is considered an implementation detail in the YAML specification.

## Position Tracking in yaml-rust2

The yaml-rust2 implementation (particularly the bangarang fork) includes enhanced position tracking capabilities for flow mappings. This allows applications to precisely locate the position of flow mapping components in the source document.

### How Position Tracking Works for Flow Mappings

When parsing a flow mapping, yaml-rust2 tracks several key positions:

1. The opening brace `{` position (line and column)
2. The closing brace `}` position (line and column) 
3. The position of each key and value within the mapping
4. Start and end positions for the entire mapping node

### Position Data Structure

Position information is stored in several related structures:

- **Marker**: A single point in the source document with line, column, and byte index
- **PositionSpan**: A range with start and optional end positions
- **SourceLocation**: A range with additional byte offset information

### Flow Mapping Position Tracking Challenges

Currently, the yaml-rust2 implementation faces several challenges with flow mapping position tracking:

1. **Incorrect Start Positions**: Flow mappings are sometimes detected starting at column 0 instead of their actual position (where the `{` character is located)
   
2. **Missing End Positions**: Some flow mappings are missing end positions (the position of the closing `}` brace)
   
3. **Pointer Equality Issues**: Nodes in the source map are associated with positions using pointer equality (`*const Yaml`), but there might be inconsistencies in how nodes are created and referenced

4. **Flow Collection Delimiter Tracking**: The position tracking for opening and closing delimiters (`{`, `}`) is inconsistent

### Position Tracking API

The yaml-rust2 implementation provides several API methods for accessing flow mapping positions:

```rust
// Get position information for a specific flow mapping
let position = loader.position_tracker().get_flow_mapping_position(id);

// Access all tracked flow mapping positions
let all_positions = loader.position_tracker().flow_mapping_positions();

// Get the source location for a node from the source map
let location = source_map.get_location(node_id);
```

When using the `PositionTrackedLoader` with the `source_mapping` feature enabled, you get enhanced position tracking capabilities:

```rust
use yaml_rust2::yaml::PositionTrackedLoader;
use yaml_rust2::source_map::SourceMapSupport;

// Parse the YAML
let mut loader = PositionTrackedLoader::default();
let mut parser = Parser::new(yaml_str.chars());
parser.load(&mut loader, true).unwrap();

// Build source maps
let source_maps = loader.build_source_maps();
let source_map = &source_maps[0];

// Get position information for a flow mapping
if let Some(location) = source_map.get_location(flow_mapping_id) {
    // Access start position
    let start = location.span.start;
    println!("Flow mapping starts at line {}, column {}", start.line(), start.col());
    
    // Access end position (if available)
    if let Some(end) = location.span.end {
        println!("Flow mapping ends at line {}, column {}", end.line(), end.col());
    }
}
```

## Integration with Other YAML Features

### Anchors and Aliases in Flow Mappings

Flow mappings can contain anchors and alias references:

```yaml
# Anchors in flow mappings
base: &base {
  key1: value1,
  key2: value2
}

# Reference the entire mapping
extended: *base

# Anchors on individual values
values: {
  v1: &anchor_v1 100,
  v2: *anchor_v1
}
```

Position tracking records both the position of the anchor definition and the alias references.

### Tags in Flow Mappings

Flow mappings can include explicit type tags:

```yaml
# Tags on the entire mapping
!!map {key1: value1, key2: value2}

# Tags on individual values
{
  key1: !!str "42",  # Force string interpretation
  key2: !!int "42"   # Force integer interpretation
}
```

### Merging with `<<` Operator

The merge operator `<<` can be used with aliases to merge mappings:

```yaml
defaults: &defaults {
  timeout: 30,
  retries: 3
}

config: {
  <<: *defaults,    # Merge in the defaults
  timeout: 60       # Override specific value
}
```

## Common Parsing Challenges and Gotchas

### Position Tracking Issues

1. **Start Position Detection**: The source position of a flow mapping's opening brace can be incorrectly tracked, especially when the mapping is nested within another collection.

2. **End Position Missing**: Sometimes the end position of flow mappings is not properly tracked, making it difficult to determine the exact span of the mapping in the source.

3. **Node Identity Consistency**: During parsing, different instances of the same logical node may be created, leading to challenges in maintaining consistent position information.

### Known Implementation Issues in yaml-rust2

The yaml-rust2 implementation (bangarang fork) is actively working on addressing several flow mapping position tracking issues:

1. **Root Causes**:
   - Inconsistent node creation during parsing
   - Position propagation issues from the scanner through the parser to the loader
   - Special logic for flow collections not correctly tracking delimiter positions
   - Anchor resolution challenges affecting node identity preservation

2. **Ongoing Solutions**:
   - Enhanced flow mapping position tracking with unique IDs
   - Direct position propagation throughout the parsing pipeline
   - Improved handling of delimiter positions
   - Node registry for consistent identity tracking

### Best Practices for Position Tracking

1. **Use PositionTrackedLoader**: Enable the `position_tracked_loader` feature to access enhanced position tracking capabilities.

2. **Check for Missing End Positions**: Always check if `position_span.end` is `Some` before trying to use the end position.

3. **Use Content-Based Node Lookup**: When looking up nodes in the source map, prefer content-based comparison over pointer equality for more reliable results.

4. **Handle Implementation Limitations**: Be aware of the current limitations in position tracking and implement appropriate fallbacks.

```rust
// Example of safely handling end positions
if let Some(location) = source_map.get_location(flow_mapping_id) {
    // Always safe to use start position
    println!("Start: line {}, column {}", location.span.start.line(), location.span.start.col());
    
    // Check if end position exists before using it
    if let Some(end) = location.span.end {
        println!("End: line {}, column {}", end.line(), end.col());
    } else {
        println!("End position not available");
        // Implement fallback strategy if needed
    }
}
```

## Future Direction

The yaml-rust2 project (particularly the bangarang fork) continues to enhance flow mapping handling and position tracking. Future improvements include:

1. Fixing node identity consistency to ensure reliable position tracking
2. Enhancing flow collection position tracking, particularly for start and end positions
3. Implementing a more robust end position calculation for all node types
4. Adding better debugging and validation for positions

## Conclusion

Flow mappings provide a compact, JSON-like syntax for representing collections of key-value pairs in YAML 1.2. While they offer advantages in terms of readability and familiarity, they also present challenges for parsers, particularly in tracking accurate source positions.

The yaml-rust2 implementation continues to improve its handling of flow mappings, with enhanced position tracking capabilities that allow applications to precisely locate these structures in source documents. By understanding both the YAML 1.2 specification rules for flow mappings and the implementation details of yaml-rust2, developers can effectively work with these structures in their applications.