# Flow Sequences in YAML 1.2

Flow sequences are a concise representation of lists in YAML that use square brackets (`[` and `]`) to delimit their contents. This document provides a comprehensive overview of flow sequences, their syntax, usage patterns, and implementation details specific to the yaml-rust2 library.

## Definition

A flow sequence in YAML 1.2 is a collection that:
- Is enclosed in square brackets (`[` and `]`)
- Contains zero or more elements separated by commas (`,`)
- Follows JSON array-like syntax
- Allows in-line representation of sequence data

Flow sequences are part of YAML's "flow style" collections (along with flow mappings), which enable more compact representations than the indentation-based "block style" collections.

## Basic Syntax

According to the YAML 1.2.2 specification, flow sequence content is denoted by surrounding `[` and `]` characters. The basic syntax is:

```yaml
[element1, element2, element3, ...]
```

Where each element can be any valid YAML value, including:
- Scalars (strings, numbers, booleans, null)
- Nested flow sequences
- Flow mappings
- References to anchors

The specification defines:
- Sequence entries are separated by a `,` character
- A trailing comma is allowed after the last entry
- Empty entries (consecutive commas) are not allowed in YAML 1.2
- Whitespace around separators is optional
- A flow sequence can be completely empty (e.g., `[]`)

## Examples of Valid Flow Sequences

### Empty Sequence

```yaml
empty_sequence: []
```

### Simple Values

```yaml
numbers: [1, 2, 3, 4, 5]
strings: [red, green, blue]
mixed: [42, "string", true, null]
```

### Multi-line Flow Sequences

```yaml
multi_line: [
  item1,
  item2,
  item3
]
```

### Nested Sequences

```yaml
nested: [
  [1, 2, 3],
  [a, b, c],
  [true, false]
]
```

### Mixed with Flow Mappings

```yaml
complex: [
  {name: John, age: 30},
  {name: Jane, age: 28},
  {name: Bob, age: 35}
]
```

## Whitespace Handling

Flow sequences allow flexible whitespace:

```yaml
# All these are equivalent
compact: [1,2,3]
spaced: [ 1, 2, 3 ]
padded: [  1  ,  2  ,  3  ]
```

## Edge Cases and Restrictions

### Trailing Commas

YAML 1.2 allows a single trailing comma in flow sequences:

```yaml
valid: [1, 2, 3,]  # Valid in YAML 1.2
```

### Empty Elements

Empty elements (consecutive commas) are not allowed:

```yaml
invalid: [1,,2]  # Invalid in YAML 1.2
```

### Implicit Types vs. Explicit Tags

Flow sequence elements receive implicit typing by default, but can have explicit tags:

```yaml
implicit: [1, 2, 3]  # Implicitly typed as integers
explicit: [!str 1, !str 2, !str 3]  # Explicitly tagged as strings
```

### Line Folding

Unlike block styles, flow sequences do not perform special line folding:

```yaml
# Line breaks in plain scalars in flow sequences are converted to spaces
sequence: [
  Line breaks 
  are converted
  to spaces
]
```

## Position Tracking in yaml-rust2

Position tracking for flow sequences in yaml-rust2 involves tracking both start and end positions in the document.

### Start Position

The start position of a flow sequence is the position of the opening `[` character:

```yaml
sequence: [1, 2, 3]
         ^ Start position is here
```

### End Position

The end position of a flow sequence is the position immediately after the closing `]` character:

```yaml
sequence: [1, 2, 3]
                  ^ End position is here
```

### Implementation Details

In the yaml-rust2 implementation:

1. The parser tracks the position of the opening `[` when it encounters a `FlowSequenceStart` token
2. When the parser encounters a `FlowSequenceEnd` token, it creates a span from the start to the end position
3. This span is then passed to the position-tracked loader to associate with the YAML node

The primary structures involved include:

- `PositionSpan`: Contains the start and end positions of a YAML element
- `PositionTracker`: Manages the mapping between YAML nodes and their positions

### Known Issues with Position Tracking

Based on the position_tracking_bugs.md file, there are several known issues related to position tracking for flow sequences in yaml-rust2:

1. **Missing End Positions**: Flow sequences like `flow_seq: [1, 2, 3]` may have missing end positions in the source map.
2. **Flow Collection Start and End Delimiter Tracking**: The position tracking for opening and closing delimiters (`[` and `]`) may be inconsistent.
3. **Pointer Equality Issues**: Flow sequence nodes may have inconsistent identity tracking, causing position information to be lost.

The root causes include:
- Inconsistent node creation during parsing
- Position propagation issues from scanner to loader
- Special handling for flow collections not properly tracking positions

Work is ongoing to improve these aspects of the implementation.

## Interacting with Other YAML Features

### Anchors and Aliases

Flow sequences can use anchors to allow references to the sequence or its elements:

```yaml
# Anchoring a flow sequence
anchored_sequence: &seq_anchor [1, 2, 3]
reference: *seq_anchor  # References the entire sequence

# Elements can also have anchors
elements: [&a 1, &b 2, &c 3]
references: [*c, *b, *a]  # References individual elements
```

Position tracking for anchors in flow sequences tracks both the anchor declaration and references.

### Tags

Flow sequences can have tags to specify the type of the sequence or its elements:

```yaml
# Tag for the entire sequence
tagged_sequence: !!seq [1, 2, 3]

# Tags for individual elements
mixed_tags: [!!int 1, !!str "2", !!bool true]
```

### Comments

Comments are allowed within flow sequences:

```yaml
with_comments: [
  1,  # First item
  2,  # Second item
  3   # Last item
]
```

## Flow Sequence Entries

The YAML 1.2.2 specification notes that any flow node may be used as a flow sequence entry. It also provides a compact notation for flow sequence entries that are mappings with a single key-value pair:

```yaml
# This compact notation:
[foo: bar]

# Is equivalent to:
[{foo: bar}]
```

This compact notation allows for more readable sequences of simple mappings. The yaml-rust2 parser must handle this special case correctly.

## Common Parsing Challenges

### 1. Nested Flow Collections

Deeply nested flow collections can be challenging to parse correctly, especially when position tracking is involved:

```yaml
deeply_nested: [
  [1, 2, [3, 4, [5, 6]]],
  {key: [a, b, {nested: [c, d]}]}
]
```

The parser must correctly track the hierarchy of nested structures and their positions. According to the YAML specification, each level of nesting creates a new context with its own rules.

### 2. Mixed Styles

Mixing flow and block styles can cause parsing difficulties:

```yaml
mixed_styles:
  - block_item1
  - block_item2
  - [flow_item1, flow_item2]
```

### 3. Position Tracking Challenges

Position tracking for flow sequences in yaml-rust2 faces several challenges:

1. **Maintaining Node Identity**: When the same sequence is referenced multiple times, node identity may not be preserved consistently.

2. **End Position Calculation**: Accurately determining the end position of a flow sequence, especially for multi-line sequences.

3. **Error Reporting**: When parsing fails within a flow sequence, providing accurate position information for error messages.

4. **Flow vs. Block Style Distinction**: The parser needs to handle different position tracking logic for flow vs. block sequences.

### 4. Self-Referential Structures

Sequences that refer to themselves through anchors and aliases create cycles:

```yaml
self_referential: &self [1, 2, *self]
```

## Implementation-Specific Details for yaml-rust2

The yaml-rust2 implementation handles flow sequences in several stages:

1. **Scanner**: Tokenizes the input, identifying flow sequence delimiters and contents
2. **Parser**: Converts tokens into events with position information
3. **PositionTrackedLoader**: Builds YAML nodes with position tracking
4. **SourceMap**: Provides a queryable interface to find nodes by position

### Position Tracking Flow

1. When the scanner encounters a `[` character, it generates a `FlowSequenceStart` token with the current position.
2. The parser creates a `SequenceStart` event and tracks the position of the opening delimiter.
3. The parser processes all elements until the closing `]` is found.
4. When the closing `]` is encountered, the parser creates a `SequenceEnd` event with a span from the start to the current position.
5. The loader builds a YAML node with the position information.
6. The source map associates the node with its position span for later lookup.

## Best Practices

1. **Use Block Sequences for Readability**: Flow sequences are compact but can be harder to read for complex data. Consider using block sequences for better readability:

   ```yaml
   # Flow style
   planets: [Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus, Neptune]
   
   # Block style (more readable for longer lists)
   planets:
     - Mercury
     - Venus
     - Earth
     - Mars
     - Jupiter
     - Saturn
     - Uranus
     - Neptune
   ```

2. **Consistent Style**: Choose either flow or block style consistently within a document.

3. **Beware of Implicit Type Conversion**: Elements in flow sequences undergo implicit type conversion. Use explicit tags when necessary.

4. **Trailing Commas**: Be careful with trailing commas, as they're allowed in YAML 1.2 but may not be in other formats like JSON.

## Conclusion

Flow sequences provide a compact way to represent lists in YAML, following a syntax similar to JSON arrays. While they are powerful and flexible, proper position tracking for flow sequences remains challenging in implementations like yaml-rust2, particularly for complex nested structures.

Understanding the syntax rules, restrictions, and implementation details of flow sequences is essential for working effectively with YAML in the yaml-rust2 library, especially when dealing with position tracking and source mapping.