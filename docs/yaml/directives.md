# YAML Directives

YAML directives are instructions that provide information to the YAML processor about the document's content. This document outlines the specification, syntax, and implementation details of YAML directives in yaml-rust2, including position tracking aspects.

## YAML 1.2.2 Specification

According to the YAML 1.2.2 specification:

- Directives are document-level instructions beginning with the `%` character
- Directives provide global metadata for the YAML processor
- Directives must appear before the document content
- YAML 1.2 defines two directives: YAML and TAG

### Directive Syntax

Directives follow this general syntax:

```
%NAME parameter1 parameter2 ...
```

The YAML 1.2.2 specification defines two standard directives:

1. **YAML Directive**: Specifies the YAML version
   ```
   %YAML 1.2
   ```

2. **TAG Directive**: Defines a tag handle and its prefix
   ```
   %TAG !handle! prefix
   ```

### Directive Rules

- Directives can only appear at the beginning of a document, before any content
- All directives end at the document marker (`---`) or the first content node
- A document may have multiple directives
- Directives only apply to the document they appear in
- Undefined directives are an error

## Implementation in yaml-rust2

### Directive Representation

In yaml-rust2, directives are processed during parsing but are not retained in the final YAML data model. However, they influence the parser's behavior.

### Scanner Level

The scanner identifies directive tokens:

- `TokenType::VersionDirective(u32, u32)` - For YAML version directives
- `TokenType::TagDirective(String, String)` - For tag directives

### Parser Level

The parser processes directives and updates its internal state:

- For version directives, it validates the YAML version
- For tag directives, it updates the `tags` HashMap that maps tag handles to prefixes
- A special `keep_tags` option controls whether tag directives are preserved across documents

### Directive Validation

The parser enforces several validation rules:

- YAML version must be supported (1.1 or 1.2)
- TAG directives must have valid tag handles and prefixes
- Duplicate tag handles overwrite previous definitions
- Reserved handles (`!` and `!!`) can be overridden but must use compatible prefixes

## Position Tracking

Position tracking for directives in yaml-rust2 is implemented as follows:

### Directive Position Information

- **Start Position**: At the `%` character of the directive
- **End Position**: After the last parameter of the directive (at the line break)

### Position Tracking Implementation

In the yaml-rust2 implementation:

- Directives are tokenized with position information
- The positions are available during parsing
- Since directives don't appear in the final YAML model, their positions aren't typically retained in the source map
- Position information may be used for error reporting during parsing

## API Usage

### Handling Version Directives

```rust
let yaml_str = "%YAML 1.2\n---\nkey: value";
let docs = YamlLoader::load_from_str(yaml_str).unwrap();
// The version directive is processed but not explicitly accessible
```

### Working with Tag Directives

```rust
let yaml_str = "%TAG !prefix! tag:example.com,2011:\n---\n!prefix!type: value";
let docs = YamlLoader::load_from_str(yaml_str).unwrap();
// The tag directive is processed, affecting tag resolution
```

### Controlling Tag Directive Behavior

```rust
let mut parser = Parser::new(scanner);
// Optional: preserve tag directives across documents
parser.keep_tags(true);
```

## Common Challenges

### Directive Scope Management

- Ensuring directives only affect the current document
- Optional preservation of tag directives across documents (non-standard)
- Handling unknown directives

### Version Compatibility

- Supporting multiple YAML versions (1.1 and 1.2)
- Different tag resolution rules between versions
- Compatibility with various YAML processors

### Position Tracking Limitations

- Directives don't appear in the final YAML model
- Position information may be limited to error reporting
- Complexities with multi-line directives (though rare)

## Implementation Notes

### Tag Directive Behavior

The parser maintains a `tags` HashMap that maps tag handles to prefixes:

```rust
// In the Parser struct
tags: HashMap<String, String>,
```

When a TAG directive is encountered, the parser:
1. Validates the handle and prefix
2. Updates the `tags` HashMap
3. Uses this mapping for resolving tags in the document

### Version Directive Handling

When a YAML directive is encountered, the parser:
1. Validates the version (must be 1.1 or 1.2)
2. May adjust parsing behavior based on the version
3. Reports an error for unsupported versions

### The `keep_tags` Option

yaml-rust2 provides a `keep_tags` option which:
- Controls whether tag directives are preserved across documents
- Is non-standard behavior (the spec says directives are document-specific)
- Can be useful for multi-document streams where tags are consistent

```rust
// Enable non-standard tag preservation
parser.keep_tags(true);
```

## Examples

### Example 1: YAML Version Directive

```yaml
%YAML 1.2
---
key: value
```

The position tracking would include:
- Directive start position at the `%` character (column 0)
- Directive end position after the last character of `1.2` (column 10)

### Example 2: Multiple Directives

```yaml
%YAML 1.2
%TAG !prefix! tag:example.com,2011:
---
!prefix!type: value
```

Each directive would have its own position information, influencing the parser's behavior.

### Example 3: Multiple Documents with Tag Directives

```yaml
%TAG !prefix! tag:example.com,2011:
---
!prefix!type: document1
...
%TAG !prefix! tag:example.org,2012:
---
!prefix!type: document2
```

By default, the second TAG directive replaces the first one for the second document. With `keep_tags(true)`, the first directive would persist unless explicitly overridden.

## Related Documentation

- [YAML Tags](tags.md) - Information about YAML tags, which are affected by TAG directives
- [YAML Documents](document.md) - Structure of YAML documents, including directive sections
- [YAML Streams](stream.md) - Multiple documents that may have different directives
- [Position Tracked Loader](/docs/position_tracked_loader.md) - Details on position tracking implementation