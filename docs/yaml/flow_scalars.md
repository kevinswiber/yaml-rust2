# Flow Scalar Styles in YAML 1.2

Flow scalars in YAML allow inline representation of string values. They come in three styles: plain (unquoted), single-quoted, and double-quoted, each with different capabilities and restrictions.

## 1. Plain (Unquoted) Style

Plain scalars are the most readable but most restricted style. They have no explicit indicators and do not support any form of escaping.

### Definition

The plain style has the following characteristics:
- No explicit indicators (quotes)
- No escaping mechanism
- Context-sensitive boundaries
- Limited character set
- Cannot be empty
- Cannot have leading or trailing whitespace
- Limited starting characters
- Line breaks produce spaces in the parsed value

### Syntax Rules

Plain scalars follow these rules:

1. **Character Set**: Plain scalars can contain most printable characters.

2. **Starting Characters**: Plain scalars must not begin with most indicators as this would cause ambiguity with other YAML constructs. However, the characters `?`, `:`, and `-` may be used as the first character if followed by a non-space character.

3. **Content Restrictions**:
   - In flow context (within `[]` or `{}`): Cannot contain `:`, `[`, `]`, `{`, `}`, `,`
   - Cannot contain `#` preceded by whitespace (would be interpreted as a comment)
   - Cannot contain control characters (e.g., `\0`, `\t`, `\n`)

4. **Line Breaks**: Per the YAML 1.2.2 specification, line breaks in plain scalars follow special rules:
   - If a line break is followed by an empty line, it is _trimmed_; the first line break is discarded and the rest are retained as content
   - If the following line is not empty, the line break is converted to a single space
   - All continuation lines must be indented more than the first line

5. **Line Restriction**: Plain scalars are restricted to a single line when contained inside an implicit key.

### Position Tracking

Position tracking for plain scalars follows these rules:
- **Start Position**: The position of the first character
- **End Position**: 
  - For single-line: End of the scalar on the same line
  - For multi-line: End of the last line of content

### Examples

```yaml
# Simple plain scalar
plain: value

# Multi-line plain scalar
description: This is a long description
  that spans multiple lines
  and maintains indentation
```

## 2. Single-Quoted Style

The single-quoted style is specified by surrounding `'` indicators. It provides a way to include colons, commas, and other special characters that would otherwise be interpreted as structural elements.

### Definition

The single-quoted style has the following characteristics:
- Enclosed in single quotes (`'`)
- Minimal escaping (only single quotes are escaped)
- Preserves literal spacing
- Allows most special characters without escaping
- Line breaks produce spaces in the parsed value

### Syntax Rules

1. **Escaping**: The only escaping is for single quotes, which are represented by two consecutive single quotes (`''`).

2. **Special Characters**: Characters like `\`, `:`, and `"` can be used directly without any escaping.

3. **Line Breaks**: Line breaks in single-quoted scalars are converted to spaces, similar to plain scalars. A line break followed by an empty line is preserved as a newline character.

4. **Line Breaking**: It is only possible to break a long single-quoted line where a space character is surrounded by non-spaces.

### Position Tracking

Position tracking for single-quoted scalars follows these rules:
- **Start Position**: The position of the opening quote
- **End Position**: The position immediately after the closing quote

### Examples

```yaml
# Basic single-quoted scalar
single-quoted: 'This is a single-quoted string'

# Escaped single-quote
with-quote: 'It''s a string with a quote'

# Special characters that would need escaping in double-quotes
special: 'Characters like \ and " need no escaping'

# Multi-line
multiline: 'This is a
  multi-line string with
  preserved indentation'
```

## 3. Double-Quoted Style

The double-quoted style is specified by surrounding `"` indicators. It is the most expressive style, supporting full character escaping through backslash sequences.

### Definition

The double-quoted style has the following characteristics:
- Enclosed in double quotes (`"`)
- Full support for escape sequences
- Can represent any character
- Most versatile but least readable
- Unicode escape sequences

### Syntax Rules

1. **Escape Sequences**: Double-quoted scalars support a wide range of escape sequences:
   - `\"` - Double quote
   - `\\` - Backslash
   - `\0` - Null character
   - `\a` - Bell/alert
   - `\b` - Backspace
   - `\t` - Horizontal tab
   - `\n` - Line feed
   - `\v` - Vertical tab
   - `\f` - Form feed
   - `\r` - Carriage return
   - `\e` - Escape
   - `\ ` - Space (useful at the start or end of a line)
   - `\N` - Next line character
   - `\L` - Line separator
   - `\P` - Paragraph separator
   - `\xNN` - 8-bit Unicode character (2 hex digits)
   - `\uNNNN` - 16-bit Unicode character (4 hex digits)
   - `\UNNNNNNNN` - 32-bit Unicode character (8 hex digits)

2. **Line Breaks**: Line breaks in double-quoted scalars are handled similarly to single-quoted scalars, with additional escape sequence support for control over line break behavior.

3. **Whitespace Handling**: Whitespace in double-quoted strings is preserved, except for line breaks that are folded by default.

### Position Tracking

Position tracking for double-quoted scalars follows these rules:
- **Start Position**: The position of the opening double quote
- **End Position**: The position immediately after the closing double quote

### Examples

```yaml
# Basic double-quoted scalar
double-quoted: "This is a double-quoted string"

# With escape sequences
escaped: "Line 1\nLine 2\tTabbed"

# Unicode characters
unicode: "Euro symbol: \u20AC"

# Multi-line
multiline: "This is a
  multi-line string with
  preserved indentation"
```

## Special Cases and Edge Behaviors

### Empty Scalars

Each style handles empty values differently:
- **Plain**: Cannot be empty
- **Single-quoted**: `''` represents an empty string
- **Double-quoted**: `""` represents an empty string

### Line Folding Comparison

```yaml
# How line folding works across different styles:
plain: text
  with
  newlines

single: 'text
  with
  newlines'

double: "text
  with
  newlines"
```

All three examples result in: `"text with newlines"` (newlines folded to spaces), but with subtle differences in how empty lines are handled.

### Line Folding with Empty Lines

```yaml
# Empty lines create actual newlines:
plain: text

  with newlines

single: 'text

  with newlines'

double: "text

  with newlines"
```

Result: `"text\n\nwith newlines"` (empty lines preserved as newlines)

## Position Tracking in yaml-rust2

The yaml-rust2 implementation tracks positions for flow scalars as follows:

1. **Scanner Phase**:
   - Records the starting position of the scalar
   - Identifies the style (plain, single-quoted, double-quoted)
   - For quoted scalars, marks the opening quote position

2. **Parser Phase**:
   - Creates a PositionSpan with both start and end positions
   - For multi-line scalars, tracks indentation levels
   - Handles line folding according to the rules for each style

3. **Source Mapping**:
   - Maintains the start and end positions of the scalar
   - For multi-line scalars, tracks both the logical content (with line folding) and physical source

### Implementation Details for Each Style

- **Plain Scalars**: Tracked from first character to last character
- **Single-Quoted**: Tracked from opening quote to character after closing quote
- **Double-Quoted**: Tracked from opening quote to character after closing quote

## Common Challenges

### 1. Context Sensitivity

The interpretation of certain characters depends on context, particularly for plain scalars:

```yaml
plain: key:value  # A single plain scalar
flow: {key:value}  # A key-value pair in a flow mapping
```

### 2. Escaping and Position Mapping

For double-quoted scalars with escape sequences, position mapping becomes complex:

```yaml
escaped: "line\nbreak"
```

The logical position of "break" in memory is different from its physical position in the source document.

### 3. Line Folding Differences

Different scalar styles have subtle differences in line folding behavior that affect position tracking.

## References

This documentation is based on the YAML 1.2.2 specification, with particular focus on the requirements for position tracking in the yaml-rust2 implementation.