# Block Scalar Styles in YAML 1.2

Block scalars are a key feature of YAML that allow for multi-line string representation with precise control over whitespace and line breaks. Unlike flow scalars, they use indentation rather than quotes to define their boundaries. The YAML 1.2 specification defines two block scalar styles: literal and folded.

## Block Scalar Header

All block scalars begin with a **header** that consists of:

1. An indicator character (| or >)
2. Optional chomping indicator (- or +)
3. Optional indentation indicator (a number)
4. Optional comment

```yaml
# Format: indicator[chomping][indentation] [comment]
scalar: |  # Simple literal, no explicit chomping or indentation
value: >-2 # Folded with strip chomping and explicit indentation level
```

## 1. Literal Style (`|`)

The literal style preserves all line breaks and is ideal for content where newlines are significant, such as code snippets or pre-formatted text.

### Definition

The literal style has the following characteristics:
- Denoted by the `|` indicator
- Preserves all line breaks within the content
- Respects indentation based on the header's indentation indicator
- Controls trailing newlines through chomping
- Only leading indentation (up to the content indentation level) is excluded from the content

### Syntax and Examples

```yaml
# Basic literal block scalar
literal: |
  Line one
  Line two
  Line three
```

The value is: `"Line one\nLine two\nLine three\n"` (note the trailing newline)

```yaml
# Literal with indentation
indented: |
    First line (indented 4 spaces)
    Second line (indentation preserved)
      Third line (additional indentation preserved)
```

```yaml
# Literal with explicit indentation indicator
explicit: |2
  First line
  Second line
```

The indentation indicator specifies that the content starts at the indentation level of the scalar plus the specified number.

## 2. Folded Style (`>`)

The folded style converts line breaks to spaces, but preserves empty lines and more-indented lines. It's useful for text that should be word-wrapped.

### Definition

The folded style has the following characteristics:
- Denoted by the `>` indicator
- Line breaks within content are folded to spaces
- Empty lines are preserved as single newlines
- More indented lines preserve their internal line breaks
- Controls trailing newlines through chomping
- Only leading indentation (up to the content indentation level) is excluded from the content

### Syntax and Examples

```yaml
# Basic folded block scalar
folded: >
  This is a paragraph
  that spans multiple lines.
  Line breaks are folded to spaces.

  Empty lines create real line breaks.
```

The value is: `"This is a paragraph that spans multiple lines. Line breaks are folded to spaces.\n\nEmpty lines create real line breaks.\n"`

```yaml
# Folded with preserved newlines for more-indented lines
folded_indented: >
  Top level paragraph
  
    More indented lines
    preserve their newlines
  Back to the top level
```

## 3. Chomping Control

Chomping controls how final line breaks and trailing empty lines are handled in block scalars. YAML provides three chomping methods:

### Strip Chomping (`-`)

Stripping is indicated by a `-` after the style indicator. It removes the final line break and any trailing empty lines.

```yaml
strip: |-
  This is a test
  with a stripped end
```

The value is: `"This is a test\nwith a stripped end"` (no trailing newline)

### Clip Chomping (Default)

Clipping is the default behavior when no chomping indicator is specified. It keeps the final line break but removes any trailing empty lines.

```yaml
clip: |
  This is a test
  with a clipped end

```

The value is: `"This is a test\nwith a clipped end\n"` (one trailing newline)

### Keep Chomping (`+`)

Keeping is indicated by a `+` after the style indicator. It preserves the final line break and all trailing empty lines.

```yaml
keep: |+
  This is a test
  with kept empty lines


```

The value is: `"This is a test\nwith kept empty lines\n\n\n"` (all trailing newlines kept)

## 4. Indentation Indicator

The indentation indicator is a decimal digit that specifies the content indentation level relative to the level of the block scalar itself.

### How It's Determined

1. **Explicit Indicator**: If a decimal digit is provided, the content indentation level is the block scalar indentation level plus the integer value.

```yaml
explicit: |2
  Content starts at indentation 2 + base level
```

2. **Auto-detection**: If no indicator is given, the content indentation level is determined by:
   - The number of leading spaces on the first non-empty line of content
   - If all lines are empty, the number of spaces on the longest line

```yaml
auto-detect: |
  # Indentation level detected from first content line
  Content here determines the level
```

## Position Tracking for Block Scalars

The yaml-rust2 implementation needs to track precise position information for block scalars to support source mapping.

### Key Position Points

1. **Start Position**:
   - Position of the style indicator character (`|` or `>`)
   - The header (chomping and indentation indicators) is part of the scalar definition but not its content

2. **Content Start Position**:
   - First character after the first line break following the header
   - Adjusted for the content indentation level

3. **End Position**:
   - Last character of the last non-excluded line
   - Influenced by the chomping behavior

### Position Tracking Implementation

For block scalars, yaml-rust2 tracks:

1. **Header Information**:
   - Style (literal or folded)
   - Chomping indicator
   - Indentation indicator

2. **Content Boundaries**:
   - Start of the first content line
   - End of the last content line
   - Indentation level to determine actual content

3. **Logical vs. Physical Position**:
   - For folded style, the position tracker needs to handle the difference between physical line breaks in the source and logical spaces in the parsed content

```rust
// Example position tracking for a block scalar
let position_span = PositionSpan {
    start: Marker { line: 1, col: 10, index: 10 }, // Position of '|' or '>' indicator
    end: Some(Marker { line: 5, col: 20, index: 120 }) // Position of the last character of content
};
```

## Common Challenges

### 1. Empty Lines and Chomping

The interaction between empty lines and chomping controls can be complex:

```yaml
# Different representations of the same logical value
value1: >
  text

value2: >-
  text

value3: >+
  text
```

These may have different physical newlines but could map to the same logical value depending on chomping.

### 2. Indentation Edge Cases

Block scalars with complex indentation require careful tracking:

```yaml
complex: |
  First line
    More indented
  Back to first indent
      Deep indent
   Mixed indentation
```

The position tracker must account for preserved indentation beyond the content indentation level.

### 3. Interactions with Comments

Comments after the header or within the content can affect position tracking:

```yaml
with_comment: | # This is a comment
  The content starts
  on the next line
```

The position tracker must distinguish between the comment (not part of content) and the actual scalar content.

## Implementation Details

In the yaml-rust2 implementation:

1. **Scanner** identifies the block scalar indicator and header
2. **Parser** processes indentation and chomping logic
3. **Position tracker** maintains full position information:
   - Creates a span from the indicator to the end of content
   - Tracks indentation level for content boundaries
   - Applies chomping rules for end boundary determination

For source mapping purposes, it's important to maintain both:
- Physical positions in the source document
- Logical positions after processing indentation and chomping

## References

This documentation is based on the YAML 1.2.2 specification, with particular focus on the requirements for position tracking in the yaml-rust2 implementation.