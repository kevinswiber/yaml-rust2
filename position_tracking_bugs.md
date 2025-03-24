# Position Tracking Bugs

## Segmentation Fault in Flow Collections

### Issue Description

There's a segmentation fault occurring in the `test_automatic_position_tracking_flow_collections` test. The issue appears to be caused by the use of raw pointers (`*const Yaml`) as keys in `HashMap` structures within the source mapping code.

### Root Causes

1. **Unsafe Pointer Usage**: The `build` method in `SourceMapBuilder` uses a `HashMap<*const Yaml, PositionSpan>` where raw pointers are used as keys. This is fundamentally unsafe since:
   - Raw pointers don't implement proper hashing or equality semantics
   - Pointers could be invalidated if memory is reallocated
   - There are no guarantees that the pointer will remain valid over time

2. **Flow Collection Detection Issues**: There are additional issues with detecting flow collections correctly:
   - Missing end positions for flow sequences
   - Incorrect start positions for flow mappings
   - Heuristic-based detection is not reliable

### Solution Approach

1. **Replace Pointer-Based Maps with NodeId-Based Maps**:
   - Replace `HashMap<*const Yaml, PositionSpan>` with `HashMap<NodeId, PositionSpan>`
   - Use the existing `NodeId` type from the `source_map` module
   - Ensure NodeIds are properly maintained across the loader and source map

2. **Improve Flow Collection Detection**:
   - Implement `is_flow_sequence` and `is_flow_mapping` methods in `PositionTracker`
   - These methods use heuristics based on position characteristics to detect flow-style collections
   - Update span detection to correctly track opening and closing delimiters

3. **Memory Safety Improvements**:
   - Remove all unsafe code blocks
   - Avoid raw pointer usage for node identity
   - Use content-based equality where needed

### Implementation Plan

1. Update the `build` method in `SourceMapBuilder` to use `NodeId` instead of raw pointers
2. Update any code that relies on pointer equality to use NodeId-based logic
3. Enhance the position tracking for flow collections with better start/end position detection
4. Add unit tests that specifically validate flow collection position tracking

## Implementation Guide

Based on our analysis and the compilation errors we've seen, here's a detailed guide for implementing the fix:

### Step 1: Fix the Source Map Builder Interface

1. Update the `build` method in `SourceMapBuilder` to accept `NodeId` instead of raw pointers:
   ```rust
   pub fn build(
       mut self,
       document: &Yaml,
       position_spans: &HashMap<NodeId, PositionSpan>,
       node_lookup: &impl Fn(&Yaml) -> Option<NodeId>,
   ) -> SourceMap<Yaml>
   ```

2. Replace the use of raw pointers in the helper function:
   ```rust
   fn register_nodes(
       builder: &mut SourceMapBuilder,
       node: &Yaml,
       position_spans: &HashMap<NodeId, PositionSpan>,
       node_lookup: &impl Fn(&Yaml) -> Option<NodeId>,
   ) -> Option<NodeId>
   ```

### Step 2: Update the PositionTracker Implementation

1. Fix the flow detection methods to conditionally compile with the source_mapping feature:
   ```rust
   #[cfg(feature = "source_mapping")]
   pub fn is_flow_sequence(&self, node_id: usize) -> bool { ... }

   #[cfg(feature = "source_mapping")]
   pub fn is_flow_mapping(&self, node_id: usize) -> bool { ... }
   ```

### Step 3: Update the PositionTrackedLoader Implementation

1. Modify the `collect_position_spans` method to work with `NodeId` instead of raw pointers:
   ```rust
   fn collect_position_spans(
       &self,
       node: &Yaml,
       spans: &mut HashMap<NodeId, PositionSpan>,
   )
   ```

2. Implement a node lookup function that maps from Yaml references to NodeIds:
   ```rust
   fn lookup_node_id(&self, node: &Yaml) -> Option<NodeId> {
       if let Some(position_tracker) = self.position_tracker.borrow().as_ref() {
           // Use content-based equality to find matching nodes
           // First try to find node by content using find_node_id
           if let Some(id) = position_tracker.find_node_id(node) {
               return Some(NodeId::new(id));
           }
           
           // Try to calculate a hash for the node's content
           let hash = PositionTracker::calculate_node_hash(node);
           if let Some(id) = position_tracker.get_node_hash_by_content(node) {
               return Some(NodeId::new(id));
           }
       }
       None
   }
   ```

3. Update the `build_source_map_for_document` method to use the new interface:
   ```rust
   fn build_source_map_for_document(&self, document_index: usize) -> Option<SourceMap<Yaml>> {
       // Get the document
       let document = self.documents.get(document_index)?;
       
       // Collect position spans
       let mut spans = HashMap::new();
       self.collect_position_spans(document, &mut spans);
       
       // Create a builder and build the source map
       let builder = SourceMapBuilder::new();
       let node_lookup = |node: &Yaml| self.lookup_node_id(node);
       Some(builder.build(document, &spans, &node_lookup))
   }
   ```

### Step 4: Update Test Helpers

1. Modify the `find_node_by_path` helper in automatic_position_tracking_test.rs to use value equality:
   ```rust
   // Compare by value instead of pointer equality
   if node == current {
       return Some((id, current));
   }
   ```

2. Add test cases specifically for flow collections:
   ```rust
   #[test]
   fn test_flow_collection_end_positions() {
       // Test to verify that flow collections have proper end positions
   }
   ```

3. Enhance error handling in the position tracking tests:
   ```rust
   if flow_mapping_loc.span.end.is_none() {
       // Provide more detailed error information
       panic!("End position missing for flow_mapping - this indicates an issue with flow collection detection");
   }
   ```

### Step 5: Comprehensive Testing

1. Test all features that depend on position tracking:
   ```bash
   cargo test --features=source_mapping
   ```

2. Add specific tests for edge cases:
   - Flow collections at root level
   - Nested flow collections
   - Flow collections with complex nested structures
   - Flow collections with anchors and aliases

### Implementation Progress

We've begun the implementation with the following changes:

1. ✅ Added `is_flow_sequence` and `is_flow_mapping` heuristic methods to `PositionTracker`
2. ✅ Modified the `source_map.rs` file to use `NodeId` instead of raw pointers in the `build` method
3. ✅ Updated the test helper in `automatic_position_tracking_test.rs` to use value equality instead of pointer equality
4. ✅ Created the `position_tracking_bugs.md` file to document the issue and solution

When attempting to compile the code, we encountered several compilation errors that highlight the extent of the changes needed:

1. Feature flag issues: The methods need proper conditional compilation with `#[cfg(feature = "source_mapping")]`
2. Type conversion issues: Converting between `usize` and `NodeId` requires explicit type handling
3. Private field access: The `all_nodes` field in `PositionTracker` is private and needs accessor methods
4. Method signature changes: The `build` method now requires a lookup function parameter

### Remaining Implementation Tasks

The following tasks still need to be completed to fully fix the issue:

1. ⬜ Add proper accessor methods for the `all_nodes` field in `PositionTracker`
2. ⬜ Update the `collect_position_spans` method to use `NodeId` instead of raw pointers
3. ⬜ Implement the `lookup_node_id` function in `PositionTrackedLoader`
4. ⬜ Fix type conversion between `usize` and `NodeId` throughout the codebase
5. ⬜ Update method signatures for `ensure_node_has_span` and related functions
6. ⬜ Fix the `u64.abs()` method calls (these are invalid since `u64` is already unsigned)
7. ⬜ Add proper type annotations for HashMaps throughout the code
8. ⬜ Test the changes with the full test suite to ensure they fix the segmentation fault

This refactoring is substantial but necessary to resolve the segmentation fault. The transition from raw pointers to NodeId-based tracking will significantly improve code safety and reliability.

### Key Insights from ROADMAP.md

According to the project's ROADMAP.md, the team has already addressed some pointer equality issues in the past:

1. They've enhanced the `PositionTracker` to store all nodes, not just anchored ones
2. They've added an `all_nodes` map to ensure consistent node instances throughout parsing
3. They've implemented content-based lookup methods like `find_node_id`

However, the source map building process still uses raw pointers, which is causing the current segmentation fault. Our changes build on the existing work by completely eliminating the use of raw pointers in favor of a more robust NodeId-based approach.