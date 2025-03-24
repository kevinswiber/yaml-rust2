# YAML 1.2.2 Position Tracking Requirements

This document specifies position tracking requirements for properly handling YAML 1.2.2 documents, including special attention to various flow and block collections.

## Position Tracking Fundamentals

Position tracking in YAML documents needs to record:

1. **Start Position**: The exact line and column where a YAML construct begins
2. **End Position**: The exact line and column where a YAML construct ends
3. **Content Type**: Whether the construct is a scalar, sequence, mapping, alias, etc.
4. **Style Information**: Whether collections use flow or block style

## Position Requirements by YAML Construct

### 1. Scalars

#### Plain Scalars
```yaml
key: value
```
- **Start**: First character of the value (`v`)
- **End**: Last character of the value (`e`)

#### Quoted Scalars (Single and Double)
```yaml
single: 'quoted value'
double: "quoted value"
```
- **Start**: First quote character
- **End**: Last quote character

#### Block Scalars (Literal and Folded)
```yaml
literal: |
  This is a multi-line
  literal value.

folded: >
  This is a multi-line
  folded value.
```
- **Start**: The indicator character (`|` or `>`)
- **End**: Last character of the last line of content

### 2. Flow Collections

#### Flow Sequences
```yaml
flow_sequence: [item1, item2, item3]
```
- **Start**: Opening bracket (`[`)
- **End**: Closing bracket (`]`)

#### Flow Mappings
```yaml
flow_mapping: {key1: value1, key2: value2}
```
- **Start**: Opening brace (`{`)
- **End**: Closing brace (`}`)

#### Nested Flow Collections
```yaml
nested: [item1, {key1: value1, key2: [a, b, c]}]
```
- Each collection must have properly tracked positions for both start and end

### 3. Block Collections

#### Block Sequences
```yaml
block_sequence:
  - item1
  - item2
  - item3
```
- **Start**: First dash (`-`)
- **End**: Last character of the last item

#### Block Mappings
```yaml
block_mapping:
  key1: value1
  key2: value2
```
- **Start**: First character of the first key
- **End**: Last character of the last value

### 4. Anchors and Aliases

#### Anchors
```yaml
anchor: &ref_value referenced content
```
- **Start**: Start of the anchor identifier (`&`)
- **End**: End of the anchor name (before the content)

#### Aliases
```yaml
alias: *ref_value
```
- **Start**: Start of the alias identifier (`*`)
- **End**: End of the alias name

## Technical Implementation Requirements

1. **Consistent Node Identity**:
   - Nodes with the same content should maintain the same identity throughout the parsing process
   - Pointer equality checks should work correctly in the source map

2. **Memory Safety**:
   - No usage of raw pointers (`*const Yaml`) as HashMap keys
   - No unsafe code blocks for normal operation
   - Stable node identifiers that don't rely on memory addresses

3. **Complete Coverage**:
   - All YAML nodes must have position information, not just anchored nodes
   - Both start and end positions must be tracked for all nodes

4. **Accurate Flow Collection Tracking**:
   - Flow collections must have properly tracked start and end positions
   - Delimiters must be included in the position spans

5. **Nested Structure Support**:
   - Position tracking must work correctly for deeply nested structures
   - Combined types (like flow sequences in block mappings) must be handled correctly

## API Requirements

1. **Source Map API**:
   - Find nodes by position (line, column)
   - Get position for any node in the document
   - Support for node range lookups
   - Error reporting with precise position information

2. **Heuristic Support**:
   - Detection of flow and block styles based on position characteristics
   - Support for malformed documents with missing end markers

## Testing Requirements

1. **Test Coverage**:
   - Tests for all YAML construct types
   - Tests for edge cases (empty collections, multi-line content, etc.)
   - Tests for nested structures with mixed styles

2. **Validation**:
   - Verify start and end positions for all nodes
   - Validate flow collection position tracking
   - Ensure position tracking works with all parser modes