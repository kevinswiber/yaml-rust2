# Position Tracking Roadmap for yaml-rust2

## Current Implementation Analysis

Currently, position information for flow mappings is tracked and propagated from the scanner to the loader as follows:

1. **Scanner Level**:
   - `Scanner` maintains a `flow_mapping_positions` HashMap that stores Marker positions for flow mappings
   - When a flow mapping starts (`{`), `store_flow_mapping_position()` generates a unique ID and stores the position
   - The FlowMappingStart token includes this ID as `Option<usize>`

2. **Parser Level**:
   - The Parser receives tokens from the Scanner
   - When it sees a `FlowMappingStart` token with a position ID, it creates a `MappingStart` event that includes this ID
   - The Parser exposes `get_flow_mapping_position()` and `flow_mapping_positions()` methods that delegate to the scanner

3. **Loader Level**:
   - Before parsing, `parse_yaml_with_options()` fetches flow mapping positions from the parser
   - Positions are converted and stored in the MarkedLoader's `flow_mapping_positions` HashMap
   - When processing a `MappingStart` event, the loader checks for a flow mapping position ID and uses it if available

This approach works but introduces complexity through ID management and multiple position lookups.

## Detailed Analysis: YAML Constructs Requiring Multiple Position Markers

After analyzing the YAML specification and the current parser implementation, we've identified several YAML constructs that require tracking positions from multiple tokens for accurate position information:

### 1. Block Mappings

**Issue**: A block mapping's true span starts with the first key and ends with the last character of the last value. The current implementation only tracks:
- The position of the `BlockMappingStart` token (which follows the colon of the first key)
- The position of the `BlockEnd` token (which is often just a line with less indentation)

**YAML Example**:
```yaml
key1: value1
key2: value2
```
The mapping should span from the 'k' in `key1` to the last '2' in `value2`.

### 2. Block Sequences

**Issue**: Block sequences should span from the first dash to the last character of the last entry, not to the indentation change below.

**YAML Example**:
```yaml
- entry1
- entry2
```
The sequence should span from the first `-` to the last '2' in `entry2`.

### 3. Flow Mappings

**Issue**: Flow mappings should have their position information precisely track the opening and closing braces.

**YAML Example**:
```yaml
{key1: value1, key2: value2}
```
The mapping should span from the opening brace `{` to the closing brace `}`.

### 4. Flow Sequences

**Issue**: Similarly, flow sequences should have their position information track the opening and closing brackets.

**YAML Example**:
```yaml
[item1, item2, item3]
```
The sequence should span from the opening bracket `[` to the closing bracket `]`.

### 5. Multi-line Scalar Values

**Issue**: Scalar values can span multiple lines with different styles (literal `|`, folded `>`, or plain).

**YAML Example**:
```yaml
description: |
  This is a multi-line
  description with preserved newlines
  that spans several lines.
```

The scalar should span from 'T' in "This" to the last period.

### 6. Anchors and Aliases

**Issue**: Anchors (`&something`) and aliases (`*something`) need positions for both the marker and the name.

### 7. Flow Collections with Complex Nesting

**Issue**: Flow-style collections can have complex nesting patterns that need position tracking at each level.

**YAML Example**:
```yaml
{key1: [item1, {nested_key: nested_value}], key2: value2}
```

### 8. Tags

**Issue**: Tags need positions for both the tag symbol and tag name.

**YAML Example**:
```yaml
!!str "string value"
!custom customValue
```

### 9. Document Start/End Markers

**Issue**: Document markers `---` and `...` need positions for the entire marker.

## Alternatives to Consider

### Option 1: Direct Position Propagation in Events

Instead of using IDs and a separate hashmap, position information could be directly included in events:

```rust
// Current approach
Event::MappingStart(usize, Option<Tag>, TMappingStyle, Option<usize>) // last param is position_id

// Alternative: Include the marker position directly
Event::MappingStart(usize, Option<Tag>, TMappingStyle, Option<Marker>) // last param is position
```

**Pros**:
- No need for separate lookup tables and ID generation
- Positions travel directly with their events
- Eliminates the need to pass positions separately

**Cons**:
- Changes the Event API
- Requires implementing Clone for Marker (already done)
- Adds memory overhead for each Event

### Option 2: Enhanced MarkedEventReceiver

The `MarkedEventReceiver` interface could be extended to provide more context:

```rust
pub trait EnhancedMarkedEventReceiver {
    // Current approach - single marker
    fn on_event(&mut self, ev: Event, mark: Marker);
    
    // Alternative - start and end markers for block-level constructs
    fn on_block_event(&mut self, ev: Event, start_mark: Marker, end_mark: Marker);
    
    // Alternative - specific handler for flow mappings
    fn on_flow_mapping_start(&mut self, anchor_id: usize, tag: Option<Tag>, mark: Marker);
}
```

**Pros**:
- Provides more precise position information for different construct types
- Maintains backward compatibility with existing code
- Could be implemented incrementally

**Cons**:
- More complex API
- Requires changes to all receivers
- Multiple callback methods to maintain

### Option 3: Unified Token Positions in Scanner-to-Parser Layer

Modify the scanner to include complete position information in all tokens and ensure it's preserved through parsing. Extend the model to track both start and end positions for constructs that span multiple tokens:

```rust
// Structure to hold both start and end positions
pub struct EventPositions {
    start: Marker,
    end: Option<Marker>, // Optional for events that don't have a clear end yet
}

// Then in parser.rs, preserve both the position and the token type when creating events
fn process_token(&mut self, token: Token) -> (Event, EventPositions) {
    let Token(mark, token_type) = token;
    match token_type {
        TokenType::FlowMappingStart(_) => {
            // Create event with the exact marker from the token
            (Event::MappingStart(anchor_id, tag, TMappingStyle::Flow, None), 
             EventPositions { start: mark, end: None })
        }
        // Other token types...
    }
}
```

**Pros**:
- Uses existing structures
- Consistent handling for all token types
- No need for separate position tracking
- Can accurately track both start and end positions

**Cons**:
- Requires parser to track multiple positions during parsing
- May require refactoring parsing logic

## Position Tracking Requirements

Based on the analysis above, we've established the following concrete requirements for position tracking:

1. **Flow Mappings**: 
   - Start position: Must point to the opening brace `{`
   - End position: Must point to the closing brace `}`

2. **Flow Sequences**:
   - Start position: Must point to the opening bracket `[`
   - End position: Must point to the closing bracket `]`

3. **Block Mappings**:
   - Start position: Must point to the first character of the first key
   - End position: Must point to the last character of the last value (not the indentation change below)

4. **Block Sequences**:
   - Start position: Must point to the first dash `-` of the first entry
   - End position: Must point to the last character of the last entry (not the indentation change below)

5. **Scalars**:
   - Start position: Must point to the first character of the value
   - End position: Must point to the last character of the value

## Detailed Implementation Plan for Flow Collections Position Tracking

This section outlines a step-by-step implementation plan for adding accurate position tracking for flow mappings and flow sequences. We'll take an incremental approach that considers all three components: scanner, parser, and loader.

### Stage 1: Define Position Structures (yaml-rust2)

1. **Create `PositionSpan` Structure**:
   ```rust
   /// A start and end position for a YAML construct
   #[derive(Clone, Copy, PartialEq, Debug, Eq)]
   pub struct PositionSpan {
       /// The start position of the construct
       pub start: Marker,
       /// The end position of the construct (if known)
       pub end: Option<Marker>,
   }

   impl PositionSpan {
       pub fn new(start: Marker) -> Self {
           PositionSpan { start, end: None }
       }
       
       pub fn with_end(start: Marker, end: Marker) -> Self {
           PositionSpan { start, end: Some(end) }
       }
       
       pub fn set_end(&mut self, end: Marker) {
           self.end = Some(end);
       }
   }
   ```

2. **Modify Event Struct to Include Position Spans**:
   ```rust
   // Add position fields to specific Event variants
   pub enum Event {
       // ... existing variants
       // Update these variants with position spans:
       FlowSequenceStart(usize, Option<Tag>, PositionSpan),
       FlowSequenceEnd(PositionSpan),
       FlowMappingStart(usize, Option<Tag>, PositionSpan),
       FlowMappingEnd(PositionSpan),
       // ... other variants
   }
   ```

3. **Enhance `MarkedEventReceiver` Interface**:
   ```rust
   pub trait MarkedEventReceiver {
       // Keep the original method for backward compatibility
       fn on_event(&mut self, ev: Event, mark: Marker);
       
       // Add a new method for events with position spans
       fn on_positioned_event(&mut self, ev: Event, span: PositionSpan) {
           // Default implementation calls the original method with the start position
           self.on_event(ev, span.start);
       }
   }
   ```

### Stage 2: Update Scanner Implementation (yaml-rust2)

1. **Remove Flow Mapping Positions Map**:
   - Remove `flow_mapping_positions` HashMap 
   - Remove `flow_mapping_id_counter`
   - Remove `store_flow_mapping_position()`

2. **Modify `TokenType::FlowMappingStart` and `TokenType::FlowSequenceStart`**:
   - Remove `Option<usize>` from `FlowMappingStart`
   - Keep the tokens as is, since we'll track positions in the parser

3. **Ensure Scanner Keeps Exact Positions**:
   - Verify that token positions for `FlowMappingStart`, `FlowMappingEnd`, `FlowSequenceStart`, and `FlowSequenceEnd` are accurate
   - For `FlowMappingStart`, position should be exactly at `{`
   - For `FlowMappingEnd`, position should be exactly at `}`
   - Similar for sequence tokens with `[` and `]`

### Stage 3: Enhance Parser with Position Tracking (yaml-rust2)

1. **Add Position Tracking Stack to Parser**:
   ```rust
   pub struct Parser<T> {
       // ... existing fields
       
       // Stack to track open constructs and their positions
       position_stack: Vec<(State, Marker)>,
   }
   ```

2. **Update Flow Collections Parsing Logic**:
   
   For `flow_mapping_key` and related functions:
   ```rust
   fn flow_mapping_start(&mut self, token: Token) -> ParseResult {
       let Token(start_mark, _) = token;
       
       // Push position to stack to track it for the end token
       self.position_stack.push((State::FlowMappingFirstKey, start_mark));
       
       self.state = State::FlowMappingFirstKey;
       Ok((
           Event::MappingStart(anchor_id, tag, TMappingStyle::Flow),
           PositionSpan::new(start_mark)
       ))
   }
   
   fn flow_mapping_end(&mut self, token: Token) -> ParseResult {
       let Token(end_mark, _) = token;
       
       // Pop position from stack to get start position
       let (_, start_mark) = self.position_stack.pop().unwrap();
       
       self.state = self.pop_state();
       Ok((
           Event::MappingEnd,
           PositionSpan::with_end(start_mark, end_mark)
       ))
   }
   ```
   
   Apply similar changes for flow sequences.

3. **Update Parser's `load` Method**:
   Modify the method to use the new `on_positioned_event` method of `MarkedEventReceiver` when available.

### Stage 4: Update marked-yaml Loader (marked-yaml)

1. **Remove Flow Mapping Positions Map**:
   - Remove `flow_mapping_positions` from `MarkedLoader`
   - Remove `set_flow_mapping_positions()` method

2. **Implement `on_positioned_event` Method**:
   ```rust
   impl MarkedEventReceiver for MarkedLoader {
       // ... existing on_event method
       
       fn on_positioned_event(&mut self, ev: Event, span: PositionSpan) {
           // Short-circuit if the state stack is in error
           if let Error(_) = self.state_stack.last().unwrap() {
               return;
           }
           
           // Convert YAML markers to our Marker type
           let start_mark = self.marker(span.start);
           let end_mark = span.end.map(|m| self.marker(m));
           
           let curstate = self.state_stack.pop().unwrap();
           
           // Process event, using both start and end positions to create accurate spans
           let newstate = match ev {
               // ... handle other events
               
               Event::MappingStart(_, _, style) if style == TMappingStyle::Flow => {
                   // Create span with accurate start position (at '{')
                   // End position will be filled in when MappingEnd is encountered
                   MappingWaitingOnKey(start_mark, MarkedMappingHash::new(), true)
               }
               
               Event::MappingEnd if matches!(curstate, MappingWaitingOnKey(_, _, true)) => {
                   // For flow mappings, use the provided end position (at '}')
                   if let MappingWaitingOnKey(start_mark, map, true) = curstate {
                       let span = Span::new_with_marks(start_mark, end_mark.unwrap_or(start_mark));
                       // Create the mapping node with the complete span
                       let node = Node::from(MarkedMappingNode::new(span, map));
                       // ... rest of existing logic
                   } else {
                       unreachable!()
                   }
               }
               
               // Handle other events similarly, using the appropriate positions
               // ...
           };
           
           // ... rest of existing logic
       }
   }
   ```

3. **Update `parse_yaml_with_options` Method**:
   Remove the flow mapping positions extraction and conversion logic.

### Stage 5: Testing and Validation

1. **Create Test Cases for Flow Collections**:
   - Test simple flow mappings
   - Test nested flow mappings
   - Test flow sequences
   - Test combinations of flow mappings and sequences
   - Verify position spans are correct in all cases

2. **Update Existing Tests**:
   Ensure existing tests pass with the new position tracking approach.

3. **Performance Testing**:
   Compare performance with the previous implementation.

## Phase Implementation Strategy

### Phase 1: Flow Mapping Positions (Current)

- Track positions of flow mapping start tokens (completed)
- Propagate position IDs through parser to loader
- Use positions to correctly mark opening braces of flow mappings

### Phase 2: Unified Position Tracking Model

- Define a comprehensive position tracking model
- Create `EventPositions` structure to hold start and end markers
- Modify parser to track end positions for all constructs
- Ensure end markers for block mappings and sequences point to the last character of the last value, not just to the indentation change

### Phase 3: Implementation for All YAML Constructs

- Apply the unified position tracking to all YAML constructs:
  - Block mappings with accurate end positions
  - Block sequences with accurate end positions
  - Flow mappings with proper bracket positions
  - Flow sequences with proper bracket positions
  - Multi-line scalars with full span tracking
  - Flow collections with all nesting levels
  - Tags, anchors, and aliases
  - Document delimiters

### Phase 4: Enhanced Position Propagation

- Remove the position_id parameter and the flow_mapping_positions HashMap
- Modify `MarkedEventReceiver` to receive complete position information
- Update the MarkedLoader to use the direct position information

### Phase 5: API Consolidation and Documentation

- Clean up and simplify the position tracking API
- Document the position tracking system comprehensively
- Provide examples for common use cases

## Implementation Progress

### Stage 1: Basic Position Tracking ✅

- Added `PositionSpan` structure to represent start/end positions
- Added `PositionTracker` to manage position spans for YAML constructs
- Enhanced the `MarkedEventReceiver` trait to include position spans
- Updated the Parser to track positions for YAML constructs
- Added tests to verify position tracking functionality

### Stage 2: Remove flow_mapping_positions HashMap ✅

- Removed the `flow_mapping_positions` HashMap from Scanner
- Removed position_id parameter from TokenType::FlowMappingStart
- Removed the `store_flow_mapping_position`, `get_flow_mapping_position`, and `flow_mapping_positions` methods
- Removed position_id parameter from Event::MappingStart
- Updated the Parser and MarkedLoader to use the new implementation
- Updated tests to verify the new implementation

### Stage 3: Improved Anchor Position Tracking ✅

- Enhanced `PositionTracker` to track anchor/alias positions
- Added `track_anchor` method to store anchor positions by ID
- Added `get_anchor_position` method to retrieve anchor positions
- Updated the event processing logic to handle anchor events with proper position tracking
- Added tests to verify the anchor position tracking functionality
- Ensured backward compatibility with existing code

### Stage 4: Remove anchor_map (in progress)

- Extended `PositionTracker` to store both anchor positions and node content
- Added `store_anchor_node` and `get_anchor_node` methods to manage anchor nodes
- Updated the parser to store scalar anchor nodes in the position tracker
- Enhanced alias handling to check for nodes in the position tracker
- Added tests to verify the enhanced anchor node tracking
- Maintained backward compatibility with existing code during the transition

## Future Considerations

- Tracking indentation levels more precisely
- Capturing comments and their positions
- Tracking positions for anchors and aliases
- Optimizing memory usage of position information 