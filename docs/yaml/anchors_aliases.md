# Anchors and Aliases in YAML

## Introduction

Anchors and aliases are powerful features in YAML that allow for reusing content within a document. They help reduce repetition and maintain consistency across complex data structures.

According to the YAML 1.2.2 specification:

> In the representation graph, a node may appear in more than one collection. When serializing such data, the first occurrence of the node is identified by an anchor. Each subsequent occurrence is serialized as an alias node which refers back to this anchor. Otherwise, anchor names are a serialization detail and are discarded once composing is completed.

This document explains how anchors and aliases work in YAML 1.2, their syntax, and how they are handled in the yaml-rust2 library, with particular focus on position tracking.

## What Are Anchors and Aliases?

### Anchors

An anchor in YAML is a marker that "labels" a node in the document so it can be referred to later. Anchors are defined using the `&` character followed by an anchor name:

```yaml
key: &anchor_name value
```

According to the YAML 1.2.2 specification, anchors:
- Must not contain the `[`, `]`, `{`, `}`, and `,` characters, as these would cause ambiguity with flow collection structures
- Are preserved in the serialization tree but not reflected in the representation graph
- Must not be used to convey content information (they are a serialization detail)
- Need not be preserved once the representation is composed

Anchors can be applied to any YAML node type:
- Scalars (strings, numbers, booleans, null)
- Sequences (arrays)
- Mappings (objects/dictionaries)
- Empty nodes

### Aliases

An alias is a reference to a previously defined anchor. It's defined using the `*` character followed by the anchor name:

```yaml
reference: *anchor_name
```

According to the YAML 1.2.2 specification:
- When composing a representation graph from serialized events, an alias event refers to the most recent event in the serialization having the specified anchor
- This means anchors need not be unique within a serialization (though it's good practice to make them unique)
- An anchor need not have an alias node referring to it

When a YAML processor encounters an alias, it replaces the alias with a copy of the content marked by the corresponding anchor.

## Syntax Rules and Examples

### Basic Anchor and Alias Syntax

1. **Anchors** are defined with `&` followed by the anchor name
2. **Aliases** are defined with `*` followed by the anchor name
3. Anchor names can contain alphanumeric characters, hyphens, and underscores
4. Anchor names must be unique within a document
5. Aliases must refer to previously defined anchors

### Examples of Valid Usage

**Simple Scalar Anchor:**
```yaml
name: &person_name John Smith
greeting: Hello, *person_name!
```

**Sequence (Array) Anchor:**
```yaml
&fruits
- apple
- banana
- orange

favorite_fruits: *fruits
```

**Mapping (Dictionary) Anchor:**
```yaml
&address
  street: 123 Main St
  city: Anytown
  postal_code: 12345

shipping_address: *address
billing_address: *address
```

**Nested Anchors:**
```yaml
person:
  &person_data
  name: John Smith
  address:
    &home_address
    street: 123 Main St
    city: Anytown

employee:
  <<: *person_data  # Merge the person_data into this mapping
  work_address:
    street: 456 Business Ave
    city: Corporate City
  
emergency_contact:
  name: Jane Smith
  address: *home_address  # Just reference the home address
```

## Edge Cases and Restrictions

### 1. Anchor Redefinition

In YAML 1.2, if the same anchor name is defined multiple times, the later definition overrides the earlier one:

```yaml
value1: &anchor value1
value2: &anchor value2
reference: *anchor  # This will resolve to "value2"
```

According to the YAML 1.2.2 specification:
- Anchors need not be unique within a serialization
- An alias event refers to the most recent event in the serialization having the specified anchor
- The first occurrence of a node is identified by an anchor, and subsequent occurrences are serialized as alias nodes
- Anchor names are a serialization detail and are discarded once composing is completed

In yaml-rust2, each anchor definition gets a unique ID, even when redefining an existing anchor name. This allows for more accurate tracking of anchor positions and follows the specification's requirement that "an alias node refers to the most recent preceding node having the same anchor."

### 2. Self-Referential and Circular References

YAML allows self-referential structures using anchors and aliases:

```yaml
&self_ref
  name: Self-referential example
  reference: *self_ref  # Points to itself
```

This creates a circular reference, which YAML processors must handle carefully to avoid infinite recursion. The yaml-rust2 library properly detects and handles these circular references.

### 3. Empty Scalar Anchors

YAML 1.2 allows empty scalar anchors, but they can be tricky to handle correctly:

```yaml
&empty_anchor
reference: *empty_anchor  # References an empty scalar
```

yaml-rust2's improved anchor handling includes proper support for empty scalar anchors in various positions.

### 4. Merging with Anchors

YAML has a special key `<<:` that can be used with anchors for merging mappings:

```yaml
defaults: &defaults
  adapter: postgres
  host: localhost

development:
  <<: *defaults
  database: myapp_development

production:
  <<: *defaults
  database: myapp_production
  host: db.example.com  # Overrides the host from defaults
```

Note that the merge key `<<:` is a YAML tag feature (technically the `tag:yaml.org,2002:merge` tag) and may not be supported in all YAML implementations.

## Position Tracking for Anchors and Aliases

In yaml-rust2, position tracking for anchors and aliases is crucial for source mapping and error reporting. The implementation, particularly in the bangarang fork, includes enhanced anchor handling with unique IDs and precise position tracking.

### Anchor Positions

For each anchor:
- Start position (line, column, index) of the `&` symbol
- End position after the anchor name
- The anchor ID (a unique numeric identifier)
- The node content associated with the anchor
- A mapping from anchor ID to node ID for resolution

The anchor position is tracked separately from the node position, allowing precise source mappings for both the anchor marker and the actual content.

### Alias Positions

For each alias:
- Start position (line, column, index) of the `*` symbol
- End position after the alias name
- The referenced anchor ID
- The position of the original anchor's node

Since aliases refer to the content of an anchored node, position tracking maintains both the location of the alias reference itself and the link to the original node.

### Implementation Details in yaml-rust2

According to the position_tracked_loader.md and other project documentation:

1. **Anchor Registration**: When an anchor is encountered:
   ```rust
   // In PositionTracker
   fn track_anchor(&mut self, anchor_id: usize, position: Marker) {
       self.anchor_positions.insert(anchor_id, position);
   }
   ```

2. **Node Association**: The actual node associated with an anchor is stored:
   ```rust
   // In PositionTracker
   fn store_anchor_node(&mut self, anchor_id: usize, node: Yaml) {
       self.anchor_nodes.insert(anchor_id, node);
   }
   ```

3. **Position Tracking**: Both the anchor marker and associated node's positions are tracked:
   ```rust
   // Tracking both anchor marker and node positions
   self.position_tracker.track_anchor(anchor_id, anchor_position);
   self.position_tracker.store_node_position(&node, node_position_span);
   ```

4. **Alias Resolution**: Aliases are resolved by retrieving the node associated with their referenced anchor ID:
   ```rust
   // In PositionTracker
   fn get_anchor_yaml(&self, anchor_id: usize) -> Option<Yaml> {
       self.anchor_nodes.get(&anchor_id).cloned()
   }
   ```

5. **Unique IDs**: Each anchor gets a unique ID, even when redefining an existing anchor name:
   ```rust
   // When parsing an anchor
   let anchor_id = self.next_anchor_id();
   self.anchor_names.insert(anchor_id, anchor_name.to_string());
   ```

These position tracking mechanisms ensure that even complex anchoring scenarios can be accurately mapped back to their source positions.

## Interaction with Other YAML Features

### 1. Tags and Anchors

Anchors can be used with explicit YAML type tags:

```yaml
!!str &string_anchor Hello
!!seq &sequence_anchor [1, 2, 3]
!!map &mapping_anchor {key: value}
```

When an alias references an anchor with a tag, the alias resolves to the tagged value, maintaining the tag information.

### 2. Anchors in Flow Collections

Anchors can be used in flow-style collections (the compact JSON-like syntax):

```yaml
flow_sequence: &seq_anchor [1, 2, 3]
flow_mapping: &map_anchor {key1: value1, key2: value2}
```

yaml-rust2 provides enhanced position tracking for flow collections with anchors.

### 3. Anchors and Document Structure

Anchors are document-specific and don't cross document boundaries in a multi-document YAML file:

```yaml
# Document 1
&anchor_in_doc1 value
---
# Document 2
*anchor_in_doc1  # This would be invalid because anchors don't cross document boundaries
```

## Common Parsing Challenges and Gotchas

### 1. Node Identity and Position Tracking

In yaml-rust2, a key challenge is maintaining consistent node identity when using anchors. The library uses a combination of strategies:

- Pointer equality for direct references
- Content-based hashing for comparing node content
- A centralized node registry (`all_nodes`) to ensure consistent node identity

### 2. Self-Referential Structures

When a node refers to itself through an anchor, special care is needed to prevent infinite recursion. yaml-rust2 detects self-references and handles them appropriately during alias resolution.

### 3. Anchor Resolution Order

During parsing, aliases might appear before their corresponding anchors have been fully constructed (especially for complex nested structures).

According to the YAML 1.2.2 specification:
- "It is an error for an alias node to use an anchor that does not previously occur in the document"
- "An alias node must not specify any properties or content, as these were already specified at the first occurrence of the node"
- "The alias refers to the most recent preceding node having the same anchor"

The yaml-rust2 implementation ensures proper handling by:

- Creating placeholder nodes for anchors
- Updating references when the full anchor structure is known
- Properly resolving aliases to their corresponding anchors
- Validating that aliases refer to previously defined anchors

### 4. Position Tracking Accuracy

Accurately tracking positions for anchors and aliases requires careful coordination between the scanner, parser, and loader. Challenges include:

- Maintaining correct line and column information
- Tracking both start and end positions for complex structures
- Ensuring anchored nodes have consistent position information

## Implementation in yaml-rust2

The yaml-rust2 library implements anchor handling with these key components:

### 1. Anchor Tracking

- Each anchor gets a unique ID
- Anchor positions are tracked in the `anchor_positions` map
- Anchor names are stored in the `anchor_names` map
- Actual node content is stored in the `anchor_nodes` map

### 2. Alias Resolution

- Aliases are resolved by looking up their referenced anchor ID
- The `get_anchor_yaml(id)` method retrieves the node associated with an anchor
- For circular references, special handling prevents infinite recursion

### 3. Position Tracking

- The `PositionTracker` handles both start and end positions
- Flow collections (mappings and sequences) get special position tracking
- Both anchors and regular nodes have their positions tracked for complete source mapping

### 4. API Access

- `get_anchor_names()` provides access to all anchor names
- `find_anchor_id(node)` finds the anchor ID associated with a node
- `get_anchor_position(id)` retrieves the position of an anchor

## Conclusion

Anchors and aliases are powerful YAML features that allow for content reuse and more compact documents. The yaml-rust2 library provides robust handling of these features, including accurate position tracking, proper anchor resolution, and support for complex use cases like self-references and empty scalar anchors.

Understanding how anchors and aliases work is essential for creating maintainable YAML documents and correctly processing them in applications that use the yaml-rust2 library.