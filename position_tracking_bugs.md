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

2. Add public accessors for private fields that need to be accessed:
   ```rust
   #[cfg(feature = "source_mapping")]
   pub fn get_node_from_all_nodes(&self, node_id: usize) -> Option<&Yaml> {
       self.all_nodes.get(&node_id)
   }
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

### Notes on Existing Implementation

The following issues need to be addressed in the existing implementation:

1. Private field `all_nodes` in `PositionTracker` requires accessor methods
2. Type mismatch between `usize` and `NodeId` in multiple methods
3. The `build` method interface changed to require a lookup function
4. Several methods in yaml.rs try to directly access `position_tracker.all_nodes`
5. The `u64.abs()` method does not exist, but is being called on content hash values
6. Type annotations are needed for several HashMaps

This refactoring is substantial but necessary to resolve the segmentation fault. The transition from raw pointers to NodeId-based tracking will significantly improve code safety and reliability.