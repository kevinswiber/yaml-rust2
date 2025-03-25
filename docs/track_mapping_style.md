# Tracking Node Styles Instead of NodeIds

## Current Issue

Currently, the codebase uses NodeId values to distinguish between different types of YAML nodes:
- NodeId(0) = flow mapping
- NodeId(1) = flow sequence
- NodeId(2) = block mapping
- NodeId(3) = block sequence

This is a fundamental design error:
1. Magic numbers are not type-safe
2. The meaning of each NodeId is not clear without comments
3. NodeIds should be used for unique identification, not type information
4. The approach is error-prone and hard to maintain
5. Mixing node identity with style information violates single responsibility principle

## Breaking Change

We will NOT maintain backward compatibility with the NodeId-based style tracking. This is an intentional breaking change because:
1. The current design is fundamentally flawed
2. Maintaining both systems would increase complexity
3. A clean break allows for better design decisions
4. The sooner we remove this, the fewer downstream dependencies will be affected

## Proposed Solution

Replace the NodeId-based style tracking entirely with proper style enums. NodeIds will be used solely for node identity/reference.

### 1. Create TSequenceStyle Enum

```rust
pub enum TSequenceStyle {
    Flow,  // For [1, 2, 3] style sequences
    Block, // For - item style sequences
}
```

### 2. Update Event Enum

```rust
pub enum Event {
    SequenceStart(AnchorId, Option<Tag>, TSequenceStyle),
    SequenceEnd,
    MappingStart(AnchorId, Option<Tag>, TMappingStyle),
    MappingEnd,
    // ... other variants stay the same
}
```

### 3. Update Position Stack

```rust
struct StackEntry {
    node_id: NodeId,      // Used only for node identity
    position: Marker,     // Start position
    style: NodeStyle,     // Style information for end position calculation
    parent_style: Option<NodeStyle>,  // Parent's style affects child end positions
}

enum NodeStyle {
    Sequence(TSequenceStyle),
    Mapping(TMappingStyle),
    // Could add other node types if needed
}

// Helper methods for style-aware position tracking
impl StackEntry {
    fn is_flow_style(&self) -> bool {
        matches!(
            self.style,
            NodeStyle::Sequence(TSequenceStyle::Flow) |
            NodeStyle::Mapping(TMappingStyle::Flow)
        )
    }

    fn needs_explicit_end_marker(&self) -> bool {
        // Flow style always needs explicit end marker
        // Block style ends after last entry
        self.is_flow_style()
    }

    fn calculate_end_position(&self, last_entry_end: Marker, closing_marker: Marker) -> Marker {
        if self.needs_explicit_end_marker() {
            closing_marker
        } else {
            last_entry_end
        }
    }
}
```

### 4. Update PositionTracker Methods

```rust
impl PositionTracker {
    pub fn push_sequence(&mut self, id: NodeId, style: TSequenceStyle, position: Marker) {
        self.position_stack.push(StackEntry {
            node_id: id,
            position,
            style: NodeStyle::Sequence(style),
            parent_style: None,
        });
    }

    pub fn push_mapping(&mut self, id: NodeId, style: TMappingStyle, position: Marker) {
        self.position_stack.push(StackEntry {
            node_id: id,
            position,
            style: NodeStyle::Mapping(style),
            parent_style: None,
        });
    }

    fn process_event(&mut self, ev: &Event, mark: Marker) -> PositionSpan {
        match ev {
            Event::SequenceStart(aid, _, style) => {
                // Get parent's style from stack top if any
                let parent_style = self.position_stack.last().map(|e| e.style.clone());
                
                let entry = StackEntry {
                    node_id: self.next_node_id(),
                    position: mark,
                    style: NodeStyle::Sequence(*style),
                    parent_style,
                };
                self.position_stack.push(entry);
                PositionSpan::new(mark)
            }
            Event::SequenceEnd => {
                if let Some(entry) = self.position_stack.pop() {
                    let end_pos = if entry.is_flow_style() {
                        // For flow style, use the closing bracket position
                        mark
                    } else {
                        // For block style, use the end of the last entry
                        // This might need adjustment based on indentation
                        self.last_entry_end_position()
                    };
                    
                    // Create span from start to calculated end
                    PositionSpan::with_end(entry.position, end_pos)
                } else {
                    // Error case - unmatched end
                    PositionSpan::new(mark)
                }
            }
            Event::MappingStart(aid, _, style) => {
                // Similar to SequenceStart but with mapping style
                let parent_style = self.position_stack.last().map(|e| e.style.clone());
                
                let entry = StackEntry {
                    node_id: self.next_node_id(),
                    position: mark,
                    style: NodeStyle::Mapping(*style),
                    parent_style,
                };
                self.position_stack.push(entry);
                PositionSpan::new(mark)
            }
            Event::MappingEnd => {
                if let Some(entry) = self.position_stack.pop() {
                    let end_pos = if entry.is_flow_style() {
                        // For flow style, use the closing brace position
                        mark
                    } else {
                        // For block style, use the end of the last value
                        self.last_entry_end_position()
                    };
                    
                    PositionSpan::with_end(entry.position, end_pos)
                } else {
                    // Error case - unmatched end
                    PositionSpan::new(mark)
                }
            }
            // ... handle other events ...
        }
    }

    fn last_entry_end_position(&self) -> Marker {
        // This would track the end position of the last entry we saw
        // Might need to be stored separately or calculated from other state
        self.last_entry_end.unwrap_or_else(|| self.current_position)
    }
}
```

## Migration Impact

### Breaking Changes
1. Remove all NodeId-based style checks
2. Remove style-related NodeId generation
3. Update all code that assumed NodeId values indicated style
4. Update all tests that relied on NodeId for style information

### Required Updates for Downstream Code
1. Replace `is_flow_sequence(node_id)` with `get_sequence_style(node_id)`
2. Replace `is_flow_mapping(node_id)` with `get_mapping_style(node_id)`
3. Update any custom position tracking logic
4. Update any style-dependent code

### New API (Clean Design)
```rust
impl PositionTracker {
    pub fn get_sequence_style(&self, node_id: NodeId) -> Option<TSequenceStyle> {
        // Implementation based purely on style tracking
    }

    pub fn get_mapping_style(&self, node_id: NodeId) -> Option<TMappingStyle> {
        // Implementation based purely on style tracking
    }
}
```

## Implementation Progress

### Phase 1: Initial Integration (In Progress)
- [x] Added `TSequenceStyle` enum to represent sequence styles
- [x] Updated `Event` enum to include style information in `SequenceStart`
- [x] Fixed pattern matching in event handlers to accommodate style parameter
- [ ] Update `MappingStart` event to include style information
- [ ] Create `TMappingStyle` enum for mapping styles

### Next Steps

1. **Complete Event Updates**
   - Update `EventReceiver` implementations to handle style information
   - Ensure `Parser` correctly sets style information when generating events
   - Update `EventReporter` in tests to display style information

2. **Implement Style Storage**
   - Add style tracking to `PositionTracker`
   - Create a mapping from `NodeId` to style information
   - Implement helper methods for style retrieval

3. **Update Position Calculation**
   - Modify position calculation logic to consider node styles
   - Update end position tracking based on style-specific rules
   - Handle nested style interactions correctly

4. **Test Coverage**
   - Add tests for style preservation
   - Verify correct position tracking with different styles
   - Test edge cases like empty collections and mixed styles

5. **Documentation**
   - Update API documentation to reflect style-aware methods
   - Provide migration examples for downstream users
   - Document breaking changes clearly

## Implementation Steps

### Phase 1: Remove Old System
1. Remove all NodeId-based style tracking
2. Remove style-related NodeId generation
3. Update all internal usage to handle the absence of style information

### Phase 2: Implement New System
1. Add style enums and related structures
2. Update parser to track styles explicitly
3. Update position tracking to include style information
4. Add new style-based API methods

### Phase 3: Update Tests
1. Remove tests that relied on NodeId for style
2. Add new style-aware tests
3. Update existing tests to use new style system
4. Add tests verifying NodeId is only used for identity

### Phase 4: Documentation
1. Document the breaking change
2. Provide migration guide for downstream users
3. Update API documentation
4. Add examples of correct style usage

## Migration Guide for Users

### Before (Wrong)
```rust
// Don't do this anymore
if position_tracker.is_flow_sequence(node_id) {
    // Handle flow sequence
}
```

### After (Correct)
```rust
// Do this instead
match position_tracker.get_sequence_style(node_id) {
    Some(TSequenceStyle::Flow) => {
        // Handle flow sequence
    }
    Some(TSequenceStyle::Block) => {
        // Handle block sequence
    }
    None => {
        // Handle non-sequence node
    }
}
```

## Benefits

1. **Type Safety**: The compiler will help catch mistakes in style handling
2. **Clarity**: The style of each node is explicitly communicated
3. **Maintainability**: No magic numbers or implicit meanings
4. **Extensibility**: Easy to add new styles or node types
5. **Documentation**: Types make the intent clear
6. **Spec Alignment**: Better matches how YAML spec describes these constructs

## Implementation Challenges

1. **Parser Updates**: Need to update parser to properly set styles
2. **Breaking Changes**: May need to handle transition period
3. **Test Updates**: Need to update tests to use new style enums
4. **Dependencies**: May need to update code that depends on NodeId type checks

## Migration Strategy

1. First introduce the new types and methods alongside existing ones
2. Update internal usage gradually to use new style-based methods
3. Mark NodeId-based style checks as deprecated
4. Remove old style checks in a future breaking release

## Future Considerations

1. Could extend to other YAML constructs that have style variations
2. Could add validation that style matches content
3. Could use style information for better error messages
4. Could expose style information in source maps for tooling

## Related Issues

- Current position tracking bugs may be related to style confusion
- Flow sequence end position tracking could be improved with explicit style handling
- Block sequence position tracking might need style-specific logic

## Testing Strategy

### 1. Unit Tests for Style Enums
```rust
#[test]
fn test_sequence_style_comparison() {
    assert_ne!(TSequenceStyle::Flow, TSequenceStyle::Block);
}

#[test]
fn test_style_preservation() {
    let yaml = "flow_seq: [1, 2, 3]\nblock_seq:\n  - 1\n  - 2";
    let docs = PositionTrackedLoader::load_from_str(yaml).unwrap();
    // Test that styles are correctly preserved in the AST
}
```

### 2. Position Tracking Tests
```rust
#[test]
fn test_flow_sequence_positions() {
    let yaml = "[1, 2, 3]";
    let docs = PositionTrackedLoader::load_from_str(yaml).unwrap();
    let source_map = docs.build_source_map();
    // Verify positions match flow sequence delimiters
}

#[test]
fn test_block_sequence_positions() {
    let yaml = "- 1\n- 2\n- 3";
    // Similar verification for block sequences
}
```

### 3. Edge Cases
```yaml
# Test cases to implement:
mixed_styles:
  flow_in_block:
    - [1, 2, 3]
    - {key: value}
  block_in_flow: [{
    key:
      - value1
      - value2
  }]
```

### 4. Migration Tests
```rust
// Tests that verify both old and new style tracking work during migration
#[test]
fn test_style_backward_compatibility() {
    // Verify NodeId-based checks still work
    // Verify new style-based checks work
    // Verify they produce the same results
}
```

## Detailed Migration Steps

### Phase 1: Introduce New Types
1. Add TSequenceStyle enum
2. Add style fields to relevant structs
3. Keep existing NodeId logic
4. Add `#[deprecated]` to NodeId-based style checks

### Phase 2: Update Parser
1. Modify scanner to track style information
2. Update parser to propagate styles
3. Add style information to Event enum
4. Keep NodeId generation for backward compatibility

### Phase 3: Update Position Tracker
1. Add style-aware position tracking
2. Update position stack to include style
3. Maintain both old and new tracking temporarily
4. Add new style-based helper methods

### Phase 4: Update Source Maps
1. Include style information in source maps
2. Add style-based queries
3. Update position span handling
4. Keep NodeId-based lookups working

### Phase 5: Clean Up
1. Remove NodeId-based style checks
2. Clean up deprecated methods
3. Update all tests to use new style system
4. Update documentation

## API Compatibility

### Current API (to maintain temporarily)
```rust
impl PositionTracker {
    pub fn is_flow_sequence(&self, node_id: NodeId) -> bool {
        // Existing implementation
    }

    pub fn is_flow_mapping(&self, node_id: NodeId) -> bool {
        // Existing implementation
    }
}
```

### New API
```rust
impl PositionTracker {
    pub fn get_sequence_style(&self, node_id: NodeId) -> Option<TSequenceStyle> {
        // New implementation
    }

    pub fn get_mapping_style(&self, node_id: NodeId) -> Option<TMappingStyle> {
        // New implementation
    }

    // Helper methods
    pub fn is_flow_style_sequence(&self, node_id: NodeId) -> bool {
        self.get_sequence_style(node_id)
            .map_or(false, |s| matches!(s, TSequenceStyle::Flow))
    }
}
```

## Error Handling

### Style Mismatch Detection
```rust
#[derive(Debug)]
pub enum StyleError {
    MismatchedStyle {
        expected: NodeStyle,
        found: NodeStyle,
        position: Marker,
    },
    InvalidStyle {
        node_type: &'static str,
        style: NodeStyle,
        position: Marker,
    },
}
```

### Validation
```rust
impl PositionTracker {
    pub fn validate_style(&self, node_id: NodeId) -> Result<(), StyleError> {
        // Validate style matches content
        // Check for inconsistencies
        // Return detailed errors
    }
}
```

## Performance Considerations

### Memory Impact
- Style enum vs NodeId integers
- Additional stack entry fields
- Temporary duplicate tracking during migration

### CPU Impact
- Style comparisons vs integer comparisons
- Additional validation checks
- Migration period overhead

### Optimization Opportunities
- Style enum size optimization
- Stack entry memory layout
- Validation caching

## Documentation Updates

### 1. Public API Changes
- Document new style enums
- Explain migration path
- Show conversion examples

### 2. Internal Documentation
- Detail style tracking mechanism
- Explain validation rules
- Document edge cases

### 3. Examples
```rust
// Example: Working with styles
let yaml = "flow: [1, 2, 3]\nblock:\n  - 1\n  - 2";
let docs = PositionTrackedLoader::load_from_str(yaml)?;
let source_map = docs.build_source_map();

// Get style information
let flow_style = source_map.get_sequence_style(flow_node_id);
assert_eq!(flow_style, Some(TSequenceStyle::Flow));

// Position-aware style checking
let block_style = source_map.get_sequence_style(block_node_id);
assert_eq!(block_style, Some(TSequenceStyle::Block));
```

## Future Enhancements

### 1. Style Preservation
- Track original style during round-trip parsing
- Preserve style during modifications
- Style-aware YAML emission

### 2. Style Analysis
- Detect inconsistent style usage
- Suggest style improvements
- Style statistics

### 3. IDE Integration
- Style-aware syntax highlighting
- Style-based code folding
- Style-based formatting

## Related Issues

- Flow sequence end position tracking
- Block sequence position tracking
- Style-dependent position calculation
- Comment preservation
- Round-trip parsing

## Stack-Based Style Tracking

The key to accurate end position tracking is maintaining style information on the stack. This allows us to correctly handle end positions based on the collection's style.

### Stack Entry with Style

```rust
struct StackEntry {
    node_id: NodeId,      // Used only for node identity
    position: Marker,     // Start position
    style: NodeStyle,     // Style information for end position calculation
    parent_style: Option<NodeStyle>,  // Parent's style affects child end positions
}

// Track both direct and parent styles
enum NodeStyle {
    Sequence(TSequenceStyle),
    Mapping(TMappingStyle),
}

// Helper methods for style-aware position tracking
impl StackEntry {
    fn is_flow_style(&self) -> bool {
        matches!(
            self.style,
            NodeStyle::Sequence(TSequenceStyle::Flow) |
            NodeStyle::Mapping(TMappingStyle::Flow)
        )
    }

    fn needs_explicit_end_marker(&self) -> bool {
        // Flow style always needs explicit end marker
        // Block style ends after last entry
        self.is_flow_style()
    }

    fn calculate_end_position(&self, last_entry_end: Marker, closing_marker: Marker) -> Marker {
        if self.needs_explicit_end_marker() {
            closing_marker
        } else {
            last_entry_end
        }
    }
}
```

### Style-Aware Event Processing

```rust
impl PositionTracker {
    fn process_event(&mut self, ev: &Event, mark: Marker) -> PositionSpan {
        match ev {
            Event::SequenceStart(aid, _, style) => {
                // Get parent's style from stack top if any
                let parent_style = self.position_stack.last().map(|e| e.style.clone());
                
                let entry = StackEntry {
                    node_id: self.next_node_id(),
                    position: mark,
                    style: NodeStyle::Sequence(*style),
                    parent_style,
                };
                self.position_stack.push(entry);
                PositionSpan::new(mark)
            }
            Event::SequenceEnd => {
                if let Some(entry) = self.position_stack.pop() {
                    let end_pos = if entry.is_flow_style() {
                        // For flow style, use the closing bracket position
                        mark
                    } else {
                        // For block style, use the end of the last entry
                        // This might need adjustment based on indentation
                        self.last_entry_end_position()
                    };
                    
                    // Create span from start to calculated end
                    PositionSpan::with_end(entry.position, end_pos)
                } else {
                    // Error case - unmatched end
                    PositionSpan::new(mark)
                }
            }
            Event::MappingStart(aid, _, style) => {
                // Similar to SequenceStart but with mapping style
                let parent_style = self.position_stack.last().map(|e| e.style.clone());
                
                let entry = StackEntry {
                    node_id: self.next_node_id(),
                    position: mark,
                    style: NodeStyle::Mapping(*style),
                    parent_style,
                };
                self.position_stack.push(entry);
                PositionSpan::new(mark)
            }
            Event::MappingEnd => {
                if let Some(entry) = self.position_stack.pop() {
                    let end_pos = if entry.is_flow_style() {
                        // For flow style, use the closing brace position
                        mark
                    } else {
                        // For block style, use the end of the last value
                        self.last_entry_end_position()
                    };
                    
                    PositionSpan::with_end(entry.position, end_pos)
                } else {
                    // Error case - unmatched end
                    PositionSpan::new(mark)
                }
            }
            // ... handle other events ...
        }
    }

    fn last_entry_end_position(&self) -> Marker {
        // This would track the end position of the last entry we saw
        // Might need to be stored separately or calculated from other state
        self.last_entry_end.unwrap_or_else(|| self.current_position)
    }
}
```

### End Position Calculation Rules

1. **Flow Sequence (`[1, 2, 3]`)**
   - End position is at the closing bracket `]`
   - Includes the bracket in the span
   - Example: `[1, 2, 3]`
                       ^ end here

2. **Block Sequence**
   ```yaml
   - item1
   - item2
   - item3  # End position is here
   ```
   - End position is at the end of the last item
   - Does not include any following empty lines
   - Considers indentation level

3. **Flow Mapping (`{key: value}`)**
   - End position is at the closing brace `}`
   - Includes the brace in the span
   - Example: `{key: value}`
                          ^ end here

4. **Block Mapping**
   ```yaml
   key1: value1
   key2: value2
   key3: value3  # End position is here
   ```
   - End position is at the end of the last value
   - Does not include any following empty lines
   - Considers indentation level

### Nested Style Considerations

The parent_style field helps handle cases like:

```yaml
flow_in_block:
  - [1, 2, 3]      # Flow sequence in block sequence
  - {key: value}   # Flow mapping in block sequence
block_in_flow: [{
  key:
    - value1       # Block sequence in flow mapping
    - value2
}]
```

Each node needs to know both its own style and its parent's style to correctly calculate positions, especially for end positions of nested structures. 

## Start Position Handling

Start positions require special handling, particularly for root-level constructs and their styles.

### Root-Level Position Rules

1. **Root Flow Sequence**
   ```yaml
   [1, 2, 3]  # Root flow sequence
   ^
   Start position here (column 0)
   ```
   - Start position is at the opening bracket
   - Always at document start (usually column 0)
   - No indentation considerations needed

2. **Root Flow Mapping**
   ```yaml
   {key: value}  # Root flow mapping
   ^
   Start position here (column 0)
   ```
   - Start position is at the opening brace
   - Always at document start (usually column 0)
   - No indentation considerations needed

3. **Root Block Sequence**
   ```yaml
   - first item   # Root block sequence
   ^
   Start position here (column 0)
   ```
   - Start position is at the first hyphen
   - Always at document start (usually column 0)
   - Subsequent items must maintain consistent indentation

4. **Root Block Mapping**
   ```yaml
   key1: value1   # Root block mapping
   ^
   Start position here (column 0)
   ```
   - Start position is at the first key
   - Always at document start (usually column 0)
   - Subsequent entries must maintain consistent indentation

### Nested Position Rules

1. **Flow in Block**
   ```yaml
   items:
     - [1, 2, 3]     # Flow sequence in block mapping
       ^
       Start at first non-whitespace after hyphen
     - {key: value}  # Flow mapping in block sequence
       ^
       Start at first non-whitespace after hyphen
   ```
   - Start position is after any leading indentation
   - Must account for parent's indentation level
   - Must account for sequence indicators if present

2. **Block in Flow**
   ```yaml
   [{
     key:
       - value1    # Block sequence in flow mapping
       ^
       Start at first hyphen, considering indentation
     nested:
       key: value  # Block mapping in flow sequence
       ^
       Start at first key, considering indentation
   }]
   ```
   - Start position must account for parent's flow context
   - Must maintain proper indentation relative to flow delimiters
   - Special care needed for indentation after flow opening delimiters

### Implementation Details

```rust
impl PositionTracker {
    fn calculate_start_position(&self, mark: Marker, style: &NodeStyle) -> Marker {
        match (self.is_root_node(), style) {
            (true, _) => {
                // Root nodes start at their exact position
                mark
            }
            (false, NodeStyle::Sequence(TSequenceStyle::Block)) => {
                // Block sequences in nested context need indentation adjustment
                self.adjust_for_block_sequence_indentation(mark)
            }
            (false, NodeStyle::Mapping(TMappingStyle::Block)) => {
                // Block mappings in nested context need indentation adjustment
                self.adjust_for_block_mapping_indentation(mark)
            }
            (false, _) => {
                // Flow styles in nested context need parent context
                self.adjust_for_nested_flow(mark)
            }
        }
    }

    fn adjust_for_block_sequence_indentation(&self, mark: Marker) -> Marker {
        // Handle indentation for block sequences
        // Must consider:
        // - Parent's indentation level
        // - Sequence indicator (-)
        // - Any additional indentation for content
        todo!()
    }

    fn adjust_for_block_mapping_indentation(&self, mark: Marker) -> Marker {
        // Handle indentation for block mappings
        // Must consider:
        // - Parent's indentation level
        // - Any additional indentation for nested content
        todo!()
    }

    fn adjust_for_nested_flow(&self, mark: Marker) -> Marker {
        // Handle flow collections in nested context
        // Must consider:
        // - Parent's style (block vs flow)
        // - Parent's indentation level
        // - Flow indicators ([ or {)
        todo!()
    }

    fn is_root_node(&self) -> bool {
        self.position_stack.is_empty()
    }
}
```

### Start Position State Tracking

```rust
struct PositionTracker {
    // ... existing fields ...
    current_indentation: usize,
    root_node_processed: bool,
    last_sequence_indicator_position: Option<Marker>,
}

impl PositionTracker {
    fn push_indentation_level(&mut self, level: usize) {
        self.indentation_stack.push(self.current_indentation);
        self.current_indentation = level;
    }

    fn pop_indentation_level(&mut self) -> usize {
        self.current_indentation = self.indentation_stack.pop()
            .unwrap_or(0);
        self.current_indentation
    }

    fn track_sequence_indicator(&mut self, position: Marker) {
        self.last_sequence_indicator_position = Some(position);
    }
}
```

### Edge Cases to Handle

1. **Empty Root Collections**
   ```yaml
   []  # Empty root flow sequence
   {}  # Empty root flow mapping
   ```
   - Start position is still at the opening delimiter
   - Special handling needed since there are no entries

2. **Mixed Indentation Styles**
   ```yaml
   items:
     - [        # Flow sequence with
         1,     # items on
         2      # separate lines
       ]
   ```
   - Must track both block and flow indentation rules
   - Need to handle line continuations properly

3. **Comments and Empty Lines**
   ```yaml
   # Comment
   
   - first  # Start position should be at the hyphen
   ```
   - Must skip comments and empty lines for start position
   - But must preserve them in source mapping

4. **Explicit Block Indicators**
   ```yaml
   ? block key
   : block value
   ```
   - Special handling for explicit block indicators
   - Start position rules differ from implicit keys

### Testing Start Positions

```rust
#[test]
fn test_root_flow_sequence_start() {
    let yaml = "[1, 2, 3]";
    let docs = PositionTrackedLoader::load_from_str(yaml).unwrap();
    let source_map = docs.build_source_map();
    
    let root_node = source_map.get_root_node();
    let position = source_map.get_node_position(root_node);
    
    assert_eq!(position.start.col(), 0);
    assert_eq!(position.start.line(), 0);
}

#[test]
fn test_nested_block_sequence_start() {
    let yaml = "items:\n  - [1, 2]";
    // Test that nested sequence starts at correct indentation
}

#[test]
fn test_mixed_style_positions() {
    let yaml = "flow: [{key:\n  - value}]";
    // Test handling of mixed block/flow styles
} 
```

## Implementation Critical Details

### State Management Requirements

The `PositionTracker` needs to maintain several pieces of state that weren't explicitly mentioned:

```rust
struct PositionTracker {
    // Existing fields
    position_stack: Vec<StackEntry>,
    node_positions: HashMap<NodeId, PositionSpan>,
    
    // New fields needed for style tracking
    style_map: HashMap<NodeId, NodeStyle>,
    indentation_stack: Vec<usize>,
    last_scalar_end: Option<Marker>,  // For tracking block sequence/mapping ends
    current_key_start: Option<Marker>, // For mapping key start positions
    current_value_start: Option<Marker>, // For mapping value start positions
    pending_style: Option<NodeStyle>,  // For style that needs to be applied to next node
}
```

### Event Order Guarantees

For correct style tracking, we rely on these event ordering guarantees from the parser:

1. For Flow Sequences:
   ```
   SequenceStart(style=Flow)
   [Scalar/Mapping/Sequence events for items]
   SequenceEnd
   ```

2. For Flow Mappings:
   ```
   MappingStart(style=Flow)
   [Scalar events for keys, followed by values]
   MappingEnd
   ```

3. For Block Sequences:
   ```
   SequenceStart(style=Block)
   [Scalar/Mapping/Sequence events for items, each preceded by indentation]
   SequenceEnd
   ```

4. For Block Mappings:
   ```
   MappingStart(style=Block)
   [Scalar events for keys, followed by values, with proper indentation]
   MappingEnd
   ```

### Critical Edge Cases

1. **Empty Collections with Comments**:
   ```yaml
   empty_flow: [  # Comment
   ]
   empty_block: # Comment
     - # Another comment
   ```
   - Must track comment positions separately
   - Comments shouldn't affect node spans
   - Empty collections still need valid spans

2. **Nested Style Inheritance**:
   ```yaml
   flow_outer: [
     block_inner:
       - item1
       - item2
   ]
   ```
   - Parent flow style affects child block style indentation
   - Need to track both styles for position calculation

3. **Multi-line Scalar Impact**:
   ```yaml
   items:
     - |
       multi-line
       content
       here
     - next item  # Position depends on previous scalar
   ```
   - Multi-line scalars affect next item's start position
   - Need to track last scalar's end position accurately

### Position Calculation Precedence

When calculating positions, follow this precedence order:

1. Style-specific rules (flow vs block)
2. Parent context rules (indentation, nesting)
3. Content-based adjustments (multi-line scalars, comments)
4. Default positioning rules

### Required Parser Modifications

The parser needs to be modified to provide:

1. Style information in events:
   ```rust
   pub enum Event {
       SequenceStart(AnchorId, Option<Tag>, TSequenceStyle, IndentInfo),
       MappingStart(AnchorId, Option<Tag>, TMappingStyle, IndentInfo),
       // ...
   }

   pub struct IndentInfo {
       level: usize,
       explicit: bool,  // Whether indentation was explicit or implied
   }
   ```

2. Additional position information:
   ```rust
   pub struct TokenInfo {
       position: Marker,
       indent_level: usize,
       line_start: bool,  // Whether token starts a new line
       preceded_by_comment: bool,
   }
   ```

### Source Map Integration

The source map needs to store style information:

```rust
impl SourceMap {
    // New methods needed
    pub fn get_node_style(&self, node_id: NodeId) -> Option<NodeStyle> {
        // Look up style from style_map
    }

    pub fn get_node_indent_level(&self, node_id: NodeId) -> Option<usize> {
        // Look up indent level
    }

    pub fn get_parent_style(&self, node_id: NodeId) -> Option<NodeStyle> {
        // Look up parent's style
    }
}
```

### Position Adjustment Rules

Detailed rules for position adjustments:

1. **Flow Sequence Start**:
   - Root: Use exact position of '['
   - Nested in Block: Add parent's indent + sequence indicator (2 spaces)
   - Nested in Flow: Use position after parent's delimiter

2. **Flow Sequence End**:
   - Always use position of ']'
   - For multi-line: Must be properly indented relative to start

3. **Block Sequence Start**:
   - Root: Use position of first '-'
   - Nested: Add parent's indent + 2 spaces
   - After key: Add parent's indent + key indent + 2 spaces

4. **Block Sequence End**:
   - Use last_scalar_end if available
   - Otherwise use last item's end position
   - Must account for multi-line content

5. **Flow Mapping Start/End**:
   - Similar to flow sequence rules
   - Must handle empty mappings properly
   - Must account for multi-line key/value pairs

6. **Block Mapping Start/End**:
   - Similar to block sequence rules
   - Must track both key and value positions
   - Must handle explicit key indicators (?)

### Required Helper Methods

```rust
impl PositionTracker {
    // Style management
    fn push_style(&mut self, style: NodeStyle);
    fn pop_style(&mut self) -> Option<NodeStyle>;
    fn current_style(&self) -> Option<&NodeStyle>;
    
    // Indentation management
    fn push_indent(&mut self, level: usize);
    fn pop_indent(&mut self) -> usize;
    fn current_indent(&self) -> usize;
    
    // Position calculation
    fn adjust_for_parent_style(&self, pos: Marker) -> Marker;
    fn calculate_content_based_end(&self, last_content: &Yaml) -> Marker;
    fn adjust_for_comments(&self, pos: Marker) -> Marker;
    
    // State tracking
    fn track_scalar_end(&mut self, end: Marker);
    fn track_key_position(&mut self, start: Marker);
    fn track_value_position(&mut self, start: Marker);
}
```

### Testing Requirements

Each test should verify:

1. Start position accuracy:
   - Correct line number
   - Correct column number
   - Accounts for indentation
   - Handles comments properly

2. End position accuracy:
   - Includes closing delimiter for flow style
   - Uses last content for block style
   - Handles multi-line content
   - Preserves indentation rules

3. Style preservation:
   - Style information is accessible
   - Style affects position calculations
   - Nested styles work correctly
   - Empty collections maintain style

4. Edge cases:
   - Comments don't affect positions
   - Empty collections have valid spans
   - Multi-line content is handled
   - Mixed styles work correctly

This additional information should help with implementation by providing:
- Concrete state management requirements
- Detailed event ordering assumptions
- Critical edge cases to handle
- Specific position calculation rules
- Required helper methods
- Comprehensive testing requirements 
