# YAML Documents in YAML 1.2

## Definition

According to the YAML 1.2.2 specification, a YAML character stream may contain several documents, with each document being completely independent from the rest. A document is the primary unit of data in YAML, representing a complete and independent piece of data.

YAML documents have specific structure rules, markers, and components that define how they are processed by parsers and how position tracking is implemented.

## Document Structure

A YAML document consists of the following parts:

1. **Document Prefix** (Optional)
   - Byte Order Mark (BOM) to specify character encoding
   - Comment lines (starting with #)
   
2. **Directives** (Optional)
   - Lines beginning with `%` character (like `%YAML 1.2`)
   - Terminated by a directives end marker
   
3. **Document Content**
   - The actual YAML content (mappings, sequences, scalars)
   - Properly indented and structured according to YAML syntax rules
   
4. **Document Suffix** (Optional)
   - Document end marker to signal the end of the document

## Document Markers

YAML uses special marker lines to control document processing:

### Directives End Marker

The directives end marker separates directives from content:

```yaml
%YAML 1.2
---
# Content starts here
key: value
```

- Consists of three hyphens (`---`)
- Signals the end of directives and the start of content
- Required if the document contains directives

### Document End Marker

The document end marker signals the end of a document:

```yaml
key: value
...
# Next document can start here
```

- Consists of three dots (`...`)
- Signals the end of a document in a multi-document stream
- Optional, but helpful for explicitly terminating documents

## Types of Documents

The YAML specification defines three types of documents:

### 1. Directive Documents

Documents that contain one or more directives, followed by a directives end marker:

```yaml
%YAML 1.2
%TAG !prefix! tag:example.com,2000:app/
---
!prefix!type: value
```

### 2. Explicit Documents

Documents that begin with a directives end marker but don't have any directives:

```yaml
---
key: value
```

### 3. Bare Documents

Documents that don't begin with any directives or marker lines:

```yaml
key: value
```

Bare documents are very "clean" as they contain nothing other than content. In a bare document, the first non-comment line may not start with a `%` character to avoid ambiguity with directives.

## Document Content Indentation

Document nodes are indented as if they have a parent indented at -1 spaces. Since a node must be more indented than its parent node, this allows the document's node to be indented at zero or more spaces.

For example:

```yaml
# Valid document - content starts at column 0
key: value

# Also valid - content is indented
  key: value
```

## Multiple Documents in a Stream

YAML allows multiple documents in a single stream, separated by document markers:

```yaml
---
# Document 1
document: 1
...
---
# Document 2
document: 2
```

When a document is not terminated by a document end marker, the following document must begin with a directives end marker.

## Position Tracking in yaml-rust2

In the yaml-rust2 implementation, position tracking for documents is implemented as follows:

### Start Position

The document's start position is recorded as:
- For directive documents: The first character of the first directive or directive end marker
- For explicit documents: The first character of the directive end marker
- For bare documents: The first character of the first content node

### End Position

The document's end position is recorded as:
- If a document end marker is present: The last character of the end marker
- Otherwise: The last character of the last content node in the document

### Document Content Boundaries

Position tracking ensures that each document's content boundaries are properly tracked, including:
- Directive positions
- Marker positions
- Content node positions

This complete position tracking allows applications to:
- Map between YAML nodes and their exact source positions
- Provide accurate error messages with document context
- Implement document-aware editing features

## Common Document Processing Challenges

### 1. Distinguishing Document Boundaries

When processing multiple documents in a stream, it's essential to correctly identify document boundaries, especially for bare documents without explicit markers.

### 2. Handling Directives

Directives must be properly processed as they can affect how the document content is interpreted (e.g., YAML version directive).

### 3. Position Tracking Across Documents

Position tracking must accurately maintain line and column numbers across document boundaries, especially when document markers are present.

### 4. Empty Documents

Empty documents (with no content but with markers) must be correctly identified and handled:

```yaml
---
...
---
key: value
```

## References

This documentation is based on the YAML 1.2.2 specification, with particular focus on the requirements for document handling and position tracking in the yaml-rust2 implementation.