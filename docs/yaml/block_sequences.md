# Block Sequences in YAML

## Definition

According to the YAML 1.2.2 specification, a block sequence is "simply a series of nodes, each denoted by a leading `-` indicator." Block sequences represent an ordered list of values (similar to arrays or lists in programming languages).

The specification notes that:
- The `-` indicator must be separated from the node by white space
- This allows `-` to be used as the first character in a plain scalar if followed by a non-space character (e.g., `-42`)
- Block sequences use indentation to determine their structure (rather than explicit delimiters)

Block sequences are one of the fundamental collection types in YAML 1.2, providing an easy-to-read format for representing ordered data. Unlike flow sequences (which use square brackets and commas), block sequences emphasize readability through consistent indentation and clear delineation of entries.

## Syntax Rules

### Basic Syntax

Block sequences follow these core syntax rules:

1. Each sequence entry starts with a hyphen (`-`) character followed by a space
2. All entries in the sequence must be aligned at the same indentation level
3. The content of each entry begins after the hyphen+space and can be any valid YAML node (scalar, sequence, or mapping)
4. Entries are separated vertically, with each entry on its own line

```yaml
# A simple block sequence with scalar entries
- item1 
- item2
- item3
```

### Indentation Requirements

Indentation is crucial for block sequences:

- The hyphen character must be indented consistently for all entries in the sequence
- The sequence entries must be indented more than their parent node
- In nested structures, each level typically adds 2 spaces of indentation

```yaml
# Block sequence as a value in a mapping
key: 
  - first item     # Note the indentation
  - second item    # All entries align at the same level
  - third item
```

### Nested Sequences

Block sequences can contain other block sequences:

```yaml
- 
  - nested item 1
  - nested item 2
- 
  - another nested item 1
  - another nested item 2
```

### Empty Entries

A block sequence can have empty entries, represented by a hyphen with no value:

```yaml
- item1
-            # This is an empty entry (null value)
- item3
```

## Complex Values in Block Sequences

### Multi-line Scalar Values

For multi-line scalar values, you can use block scalar styles (literal `|` or folded `>`):

```yaml
- |
  This is a literal block scalar
  that preserves newlines
  exactly as written.
- >
  This is a folded block scalar
  where newlines are converted
  to spaces.
```

### Mappings in Block Sequences

Block sequences can contain mappings (key-value pairs):

```yaml
- name: John Doe
  age: 30
  role: Developer
- name: Jane Smith
  age: 28
  role: Designer
```

### Compact Notation for Complex Entries

According to the YAML 1.2.2 specification, the entry node may be either completely empty, be a nested block node, or use a "compact in-line notation."

For complex values, there are two syntax options:

1. **Block notation** - The value is placed on a new line with deeper indentation:

```yaml
- 
  name: John Doe
  age: 30
```

2. **Compact notation** - The value begins on the same line as the hyphen:

```yaml
- name: John Doe
  age: 30
```

The specification notes that the compact notation may be used when the entry is itself a nested block collection. In this case, both the `-` indicator and the following spaces are considered to be part of the indentation of the nested collection.

When using compact notation, it's not possible to specify node properties (like anchors or tags) for the collection itself.

```yaml
# Compact sequence inside a sequence:
- - nested item 1
  - nested item 2
```

Both formats produce identical YAML data structures, but the choice affects readability.

## Position Tracking in Block Sequences

In the yaml-rust2 implementation, position tracking for block sequences follows these rules:

### Start Position

The starting position of a block sequence is at the first hyphen (`-`) character of the first entry. The position includes:

- Line number of the first entry
- Column number where the first hyphen appears
- Character index in the source document

### End Position

The end position of a block sequence is at the last character of the last entry's value. This includes:

- Line number of the last entry's final character
- Column number of the last character
- Character index of the last character

For example:

```yaml
block_sequence:
  - item1
  - item2:
      nested_key: nested_value
  - item3
```

In this example, the block sequence starts at the hyphen before "item1" and ends after "item3".

### Position Tracking Implementation Details

As shown in the `test_automatic_position_tracking_for_block_sequences` test:

1. The block sequence's start position is recorded at line 3, at or before column 2 (where the first hyphen appears)
2. The end position is after line 7 (where the last entry "item3" ends)
3. Each individual item within the sequence also has its own position information:
   - "item1" starts at line 4, after column 2
   - The mapping in "item2" starts around line 5-6
   - "item3" starts at line 7, after column 2

Position tracking is critical for YAML parsers to provide accurate error messages and enable applications to link YAML content to source locations, particularly important for configuration validation and IDE integrations.

## Edge Cases and Restrictions

### Empty Sequences

An empty block sequence can be represented in two ways:

```yaml
empty_sequence: []     # Flow style (preferred for empty sequences)
another_empty: [  ]    # Spaces are allowed inside the brackets
```

Empty block-style sequences would require a hyphen with no following content, which is rarely used:

```yaml
empty_block_sequence:
  -
```

### Trailing Comments

Comments can appear after entries in a block sequence:

```yaml
- item1  # This is a comment about item1
- item2  # This is a comment about item2
```

### Line Folding and Chomping Controls

Block sequences support special line folding and chomping control indicators when using block scalars:

```yaml
- |+  # Keep all trailing newlines
  This text has
  multiple lines
  with preserved newlines.

- |-  # Strip all trailing newlines
  This text will have
  the final newline removed.
```

### Indentation Errors

Common indentation errors in block sequences include:

1. **Inconsistent indentation** - Entries not aligned at the same level
2. **Insufficient indentation** - Entries not indented more than their parent
3. **Misaligned content** - Content after the hyphen not properly indented

These indentation issues can lead to parsing errors or unexpected document structures.

## Interaction with Other YAML Features

### Block Sequences with Anchors and Aliases

Block sequences can be anchored and referenced elsewhere in the document:

```yaml
- &sequence_anchor
  - item1
  - item2
- *sequence_anchor  # This references the entire sequence defined above
```

### Tags with Block Sequences

Tags can be applied to block sequences to indicate their type:

```yaml
tasks: !!seq
  - task: Clean room
    priority: high
  - task: Buy groceries
    priority: medium
```

### Merge Key with Block Sequences

Block sequences can be used with the merge key to combine mappings:

```yaml
- &defaults
  timeout: 30
  retry: true

- <<: *defaults
  timeout: 10  # Override specific values
```

### Block Sequences in YAML Document Streams

A block sequence can start a YAML document without needing an explicit document start marker:

```yaml
- document1
- document2
---
- document3  # Start of another document
```

## Common Parsing Challenges

### Whitespace Sensitivity

YAML's whitespace sensitivity presents challenges when parsing block sequences:

1. Inconsistent indentation may lead to structural misinterpretation
2. Editors that mix tabs and spaces can cause parsing issues
3. Invisible Unicode whitespace characters may cause unexpected behavior

### Handling Complex Nested Structures

Deeply nested block sequences can be challenging to parse correctly, particularly when:

1. Multiple sequence and mapping levels are interleaved
2. Content spans many lines with varying indentation
3. Complex scalar types are embedded within sequences

### Position Tracking in Mixed Structures

Accurately tracking positions in documents with mixed block and flow styles can be complex:

```yaml
mixed_sequence:
  - block_item1
  - [flow, sequence, items]
  - block_item2: 
      nested: value
```

The parser must track position transitions between different style contexts while maintaining hierarchy information.

### Performance Considerations

For large YAML documents with extensive block sequences, performance considerations include:

1. Memory usage when building the document object model
2. Handling line-by-line parsing efficiently
3. Optimizing position tracking for large collections

## Conclusion

Block sequences are a fundamental component of YAML 1.2, providing a human-readable format for ordered collections. The yaml-rust2 implementation focuses on accurate parsing and position tracking, enabling robust error reporting and source mapping capabilities.

For developers working with the yaml-rust2 crate, understanding how block sequences are represented in both the document model and position tracking system is essential for building robust YAML processing applications.