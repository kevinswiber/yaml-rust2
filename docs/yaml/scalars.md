# Scalars in YAML 1.2

## Overview

Scalars are the most basic values in YAML, representing strings, numbers, dates, booleans, and other primitive data types. Unlike collections (sequences and mappings), scalars contain a single value. YAML provides rich support for various scalar styles, each with different characteristics regarding readability, expressiveness, and special character handling.

## Scalar Styles

YAML 1.2 supports two main categories of scalar styles:

1. **Flow Scalar Styles** - Compact, inline representation:
   - **Plain** (unquoted) - Most readable, most context-sensitive, no escaping
   - **Single-quoted** - Preserves literal spacing, minimal escaping
   - **Double-quoted** - Full character escaping, Unicode support

2. **Block Scalar Styles** - Multi-line representation using indentation:
   - **Literal** (using `|`) - Preserves line breaks exactly
   - **Folded** (using `>`) - Folds line breaks to spaces, preserves empty lines

Each style has specific rules for:
- Line break handling
- Whitespace preservation/folding
- Character escaping
- Position tracking

## General Scalar Properties

### Content Model

All scalar values in YAML consist of:
- Zero or more Unicode characters
- Optional type information (explicit or implicit)
- Boundaries determined by the chosen style

### Implicit Typing

YAML applies implicit type detection based on patterns:
- `123` → integer
- `3.14` → floating point
- `true`, `false`, `yes`, `no` → boolean
- `null`, `~` → null value
- ISO8601 dates → date/time values

### Position Tracking

For source mapping in yaml-rust2, scalar positions include:

1. **Start Position**:
   - Line number (1-based)
   - Column number (0-based)
   - Character index in source

2. **End Position**:
   - Last line of the scalar content
   - Column of the last character
   - Character index of the last character

Position tracking depends on scalar style, with each style having different rules for determining boundaries.

### Implementation in yaml-rust2

Position tracking for scalars in yaml-rust2 follows this process:

1. **Scanner** identifies scalar tokens and their starting positions
2. **Parser** converts scalar tokens to events with complete position spans
3. **PositionTrackedLoader** creates YAML nodes with position information
4. **SourceMap** provides an interface to locate nodes by position

```rust
// Position tracking in a scalar
let position_span = PositionSpan {
    start: Marker { line: 1, col: 5, index: 5 },
    end: Some(Marker { line: 1, col: 10, index: 10 })
};
```

## Interaction with Anchors and Aliases

Scalars can interact with anchors and aliases:

```yaml
# Scalar with anchor
scalar: &my_anchor value

# Alias reference
reference: *my_anchor
```

In yaml-rust2, position tracking for anchored scalars handles both:
- The anchor marker's position (`&my_anchor`)
- The scalar value's position (`value`)

This separation is important for source mapping and error reporting.

## Scalar-specific Context Sensitivity

The interpretation of scalar content is sometimes context-dependent:

```yaml
key1: 2001-01-01    # Interpreted as a date
key2: "2001-01-01"  # Interpreted as a string
key3: !!str 2001-01-01  # Explicitly typed as string
```

This context sensitivity affects both parsing and position tracking.

## Common Parsing Challenges

Parsing scalars in YAML presents several challenges:

1. **Boundary Detection** - Determining where scalars begin and end, especially for plain scalars
2. **Style-specific Rules** - Each style has unique parsing rules
3. **Empty Scalars** - Handling empty values according to style
4. **Whitespace Significance** - Some styles preserve whitespace while others don't
5. **Position Tracking** - Accurately tracking positions in multi-line scalars

## Detailed Documentation

For detailed information on specific scalar styles, refer to:

- [Flow Scalars](flow_scalars.md) - Plain, single-quoted, and double-quoted styles
- [Block Scalars](block_scalars.md) - Literal and folded styles

## References

This document is based on the YAML 1.2.2 specification, with specific attention to position tracking requirements for the yaml-rust2 project.