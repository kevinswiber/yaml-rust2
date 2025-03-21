use yaml_rust2::parser::{Event, EventReceiver, MarkedEventReceiver, Parser};
use yaml_rust2::position::{PositionSpan, PositionTracker};
use yaml_rust2::scanner::Marker;
use yaml_rust2::yaml::Yaml;

/// A test receiver that collects events with position spans
struct PositionTestReceiver {
    events: Vec<(Event, PositionSpan)>,
    position_tracker: PositionTracker,
}

impl PositionTestReceiver {
    fn new() -> Self {
        PositionTestReceiver {
            events: Vec::new(),
            position_tracker: PositionTracker::new(),
        }
    }

    fn get_events(&self) -> &Vec<(Event, PositionSpan)> {
        &self.events
    }

    fn get_position_tracker(&self) -> &PositionTracker {
        &self.position_tracker
    }
}

impl MarkedEventReceiver for PositionTestReceiver {
    fn on_event(&mut self, event: Event, mark: Marker) {
        // Track the event in our position tracker for anchor tracking
        let _span = self.position_tracker.process_event(&event, mark);
        // But don't push it to events - we use on_positioned_event instead
    }

    fn on_positioned_event(&mut self, ev: Event, span: PositionSpan) {
        self.events.push((ev, span));
    }
}

/// Test that flow mappings have correct position spans.
#[test]
fn test_flow_mapping_positions() {
    let yaml = "{key1: value1, key2: value2}";
    let mut parser = Parser::new_from_str(yaml);
    let mut receiver = PositionTestReceiver::new();

    parser.load_with_positions(&mut receiver, false).unwrap();

    // Extract events we care about
    let events = receiver.get_events();

    // Find the MappingStart and MappingEnd events
    let mapping_start = events
        .iter()
        .find(|(ev, _)| matches!(ev, Event::MappingStart(_, _, _)))
        .expect("Failed to find MappingStart event");

    let mapping_end = events
        .iter()
        .find(|(ev, _)| matches!(ev, Event::MappingEnd))
        .unwrap();

    // The MappingStart event should have a position span with start at the opening '{'
    let mapping_start_span = &mapping_start.1;
    assert_eq!(
        mapping_start_span.start.col(),
        0,
        "MappingStart should point to the opening '{{'"
    );

    // The MappingEnd event should have a position span from the opening '{' to the closing '}'
    let mapping_end_span = &mapping_end.1;
    assert!(
        mapping_end_span.end.is_some(),
        "MappingEnd should have an end position"
    );
    let end_pos = mapping_end_span.end.unwrap();
    assert_eq!(
        end_pos.col(),
        27,
        "MappingEnd should point to the closing '}}'"
    );
}

/// Test that flow sequences have correct position spans.
#[test]
fn test_flow_sequence_positions() {
    let yaml = "[item1, item2, item3]";
    let mut parser = Parser::new_from_str(yaml);
    let mut receiver = PositionTestReceiver::new();

    parser.load_with_positions(&mut receiver, false).unwrap();

    // Extract events we care about
    let events = receiver.get_events();

    // Find the SequenceStart and SequenceEnd events
    let sequence_start = events
        .iter()
        .find(|(ev, _)| matches!(ev, Event::SequenceStart(_, _)))
        .unwrap();

    let sequence_end = events
        .iter()
        .find(|(ev, _)| matches!(ev, Event::SequenceEnd))
        .unwrap();

    // The SequenceStart event should have a position span with start at the opening '['
    let sequence_start_span = &sequence_start.1;
    assert_eq!(
        sequence_start_span.start.col(),
        0,
        "SequenceStart should point to the opening '['"
    );

    // The SequenceEnd event should have a position span from the opening '[' to the closing ']'
    let sequence_end_span = &sequence_end.1;
    assert!(
        sequence_end_span.end.is_some(),
        "SequenceEnd should have an end position"
    );
    let end_pos = sequence_end_span.end.unwrap();
    assert_eq!(
        end_pos.col(),
        20,
        "SequenceEnd should point to the closing ']'"
    );
}

/// Test nested flow collections have correct position spans.
#[test]
fn test_nested_flow_positions() {
    let yaml = "{key1: [item1, item2], key2: {nested: value}}";
    let mut parser = Parser::new_from_str(yaml);
    let mut receiver = PositionTestReceiver::new();

    parser.load_with_positions(&mut receiver, false).unwrap();

    // Extract events we care about
    let events = receiver.get_events();

    // Print all events with their positions for debugging
    println!("YAML: {}", yaml);
    println!("Length: {}", yaml.len());
    for (idx, (event, span)) in events.iter().enumerate() {
        println!(
            "Event {}: {:?} - Start: {} End: {:?}",
            idx,
            event,
            span.start.col(),
            span.end.map(|m| m.col())
        );
    }

    // We should find multiple MappingStart and SequenceEnd events
    let mapping_starts = events
        .iter()
        .filter(|(ev, _)| matches!(ev, Event::MappingStart(_, _, _)))
        .collect::<Vec<_>>();

    let sequence_starts: Vec<_> = events
        .iter()
        .filter(|(ev, _)| matches!(ev, Event::SequenceStart(_, _)))
        .collect();

    // Find the SequenceEnd and MappingEnd events
    let sequence_ends: Vec<_> = events
        .iter()
        .filter(|(ev, _)| matches!(ev, Event::SequenceEnd))
        .collect();

    let mapping_ends: Vec<_> = events
        .iter()
        .filter(|(ev, _)| matches!(ev, Event::MappingEnd))
        .collect();

    // Print filtered arrays to clarify structure
    println!("\nFiltered arrays:");
    println!("Mapping starts: {} items", mapping_starts.len());
    for (i, (ev, span)) in mapping_starts.iter().enumerate() {
        println!(
            "  mapping_starts[{}]: {:?} - Start: {} End: {:?}",
            i,
            ev,
            span.start.col(),
            span.end.map(|m| m.col())
        );
    }

    println!("Mapping ends: {} items", mapping_ends.len());
    for (i, (ev, span)) in mapping_ends.iter().enumerate() {
        println!(
            "  mapping_ends[{}]: {:?} - Start: {} End: {:?}",
            i,
            ev,
            span.start.col(),
            span.end.map(|m| m.col())
        );
    }

    // The outer mapping
    assert_eq!(
        mapping_starts[0].1.start.col(),
        0,
        "Outer mapping should start at the first character"
    );

    // The nested sequence - column number depends on the exact content
    assert_eq!(
        sequence_starts[0].1.start.col(),
        7,
        "Nested sequence should start at the right position"
    );

    // The nested mapping - column number depends on the exact content
    assert_eq!(
        mapping_starts[1].1.start.col(),
        29,
        "Nested mapping should start at the right position"
    );

    // Check the end positions too

    // The nested sequence end
    assert!(
        sequence_ends[0].1.end.is_some(),
        "Nested sequence should have an end position"
    );
    let seq_end_pos = sequence_ends[0].1.end.unwrap();
    assert_eq!(
        seq_end_pos.col(),
        20,
        "Nested sequence should end at the right position"
    );

    // The nested mapping end (mapping_ends[0] is the nested one because of depth-first traversal)
    assert!(
        mapping_ends[0].1.end.is_some(),
        "Nested mapping should have an end position"
    );
    let nested_map_end_pos = mapping_ends[0].1.end.unwrap();
    assert_eq!(
        nested_map_end_pos.col(),
        43,
        "Nested mapping should end at the right position"
    );

    // The outer mapping end (mapping_ends[1] is the outer one because of depth-first traversal)
    assert!(
        mapping_ends[1].1.end.is_some(),
        "Outer mapping should have an end position"
    );
    let outer_map_end_pos = mapping_ends[1].1.end.unwrap();
    assert_eq!(
        outer_map_end_pos.col(),
        44,
        "Outer mapping should end at the right position"
    );
}

#[test]
fn test_anchor_positions() {
    let yaml = "foo: &anchor bar\nalias: *anchor";
    let mut parser = Parser::new(yaml.chars());
    let mut receiver = PositionTestReceiver::new();

    let _ = parser.load_with_positions(&mut receiver, false);

    // Look for the anchor event (should be on first line)
    let events = receiver.get_events();
    let (event, span) = events.iter()
        .find(|(ev, _)| matches!(ev, Event::Scalar(s, _, anchor_id, _) if s == "bar" && *anchor_id > 0))
        .expect("Could not find anchor event");

    if let Event::Scalar(_, _, anchor_id, _) = event {
        assert_eq!(span.start.line(), 1);
        assert!(span.start.col() > 0);

        // Now look for the alias event (should be on second line)
        let (_alias_event, alias_span) = events
            .iter()
            .find(|(ev, _)| matches!(ev, Event::Alias(id) if *id == *anchor_id))
            .expect("Could not find alias event");

        assert_eq!(alias_span.start.line(), 2);
        assert!(alias_span.start.col() > 0);

        // Verify the anchor node is stored in the position tracker
        // Note: This requires modifying the Parser to expose its position_tracker,
        // which would be part of the full implementation. For now, we're just testing
        // the basic functionality.
    }
}

/// Test that anchors of different node types are properly tracked
#[test]
fn test_complex_anchor_tracking() {
    let yaml = "
sequence: &seq_anchor [1, 2, 3]
mapping: &map_anchor {key: value}
nested:
  - *seq_anchor
  - *map_anchor";

    // Create a custom position tracker and receiver
    let mut pos_tracker = PositionTracker::new();

    // Create a custom receiver that will use our position tracker
    struct AnchorTrackingReceiver {
        events: Vec<(Event, PositionSpan)>,
        position_tracker: PositionTracker,
    }

    impl AnchorTrackingReceiver {
        fn new(tracker: PositionTracker) -> Self {
            Self {
                events: Vec::new(),
                position_tracker: tracker,
            }
        }

        fn get_events(&self) -> &Vec<(Event, PositionSpan)> {
            &self.events
        }

        fn get_position_tracker(&self) -> &PositionTracker {
            &self.position_tracker
        }
    }

    impl MarkedEventReceiver for AnchorTrackingReceiver {
        fn on_event(&mut self, event: Event, mark: Marker) {
            // Process the event with our position tracker
            let span = self.position_tracker.process_event(&event, mark);
            self.events.push((event, span));
        }
    }

    // Parse the YAML using our custom receiver
    let mut parser = Parser::new(yaml.chars());
    let mut receiver = AnchorTrackingReceiver::new(pos_tracker);

    let _ = parser.load(&mut receiver, false);

    // Find the sequence anchor event
    let events = receiver.get_events();

    // Find the sequence with anchor
    let (seq_event, seq_span) = events
        .iter()
        .find(|(ev, _)| matches!(ev, Event::SequenceStart(anchor_id, _) if *anchor_id > 0))
        .expect("Could not find sequence with anchor");

    if let Event::SequenceStart(seq_anchor_id, _) = seq_event {
        println!("Found sequence with anchor ID: {}", seq_anchor_id);
        assert_eq!(seq_span.start.line(), 2);

        // Find the alias referencing the sequence anchor
        let (_alias_event, alias_span) = events
            .iter()
            .find(|(ev, _)| matches!(ev, Event::Alias(id) if *id == *seq_anchor_id))
            .expect("Could not find alias referring to sequence anchor");

        println!(
            "Found alias to sequence on line: {}",
            alias_span.start.line()
        );
        assert_eq!(alias_span.start.line(), 5);

        // Check that the position tracker stored the sequence
        let position_tracker = receiver.get_position_tracker();
        let node = position_tracker.get_anchor_node(*seq_anchor_id);
        assert!(
            node.is_some(),
            "Anchor node should be stored in position tracker"
        );

        if let Some(Yaml::Array(_)) = node {
            // It's an array, which is what we expect
        } else {
            panic!("Expected a sequence node");
        }
    }

    // Find the mapping with anchor
    let (map_event, map_span) = events
        .iter()
        .find(|(ev, _)| matches!(ev, Event::MappingStart(anchor_id, _, _) if *anchor_id > 0))
        .expect("Could not find mapping with anchor");

    if let Event::MappingStart(map_anchor_id, _, _) = map_event {
        println!("Found mapping with anchor ID: {}", map_anchor_id);
        assert_eq!(map_span.start.line(), 3);

        // Find the alias referencing the mapping anchor
        let (_alias_event, alias_span) = events
            .iter()
            .find(|(ev, _)| matches!(ev, Event::Alias(id) if *id == *map_anchor_id))
            .expect("Could not find alias referring to mapping anchor");

        println!(
            "Found alias to mapping on line: {}",
            alias_span.start.line()
        );
        assert_eq!(alias_span.start.line(), 6);

        // Check that the position tracker stored the mapping
        let position_tracker = receiver.get_position_tracker();
        let node = position_tracker.get_anchor_node(*map_anchor_id);
        assert!(
            node.is_some(),
            "Anchor node should be stored in position tracker"
        );

        if let Some(Yaml::Hash(_)) = node {
            // It's a hash, which is what we expect
        } else {
            panic!("Expected a mapping node");
        }
    }
}

#[test]
fn test_position_tracker_with_loader() {
    use yaml_rust2::parser::{MarkedEventReceiver, Parser};
    use yaml_rust2::position::PositionTracker;
    use yaml_rust2::yaml::Yaml;

    // Sample YAML with anchors and references
    let s = "
    anchored_scalar: &scalar_anchor test_value
    anchored_sequence: &seq_anchor
      - item1
      - item2
    anchored_mapping: &map_anchor
      key1: value1
      key2: value2
    
    # References to the anchors
    scalar_ref: *scalar_anchor
    sequence_ref: *seq_anchor
    mapping_ref: *map_anchor
    ";

    // Create custom loader that uses PositionTracker instead of anchor_map
    struct CustomLoader {
        position_tracker: PositionTracker,
        docs: Vec<Yaml>,
        // Only track what we need for the test
    }

    impl CustomLoader {
        fn new() -> Self {
            CustomLoader {
                position_tracker: PositionTracker::new(),
                docs: Vec::new(),
            }
        }

        // Similar to YamlLoader::insert_new_node but uses position_tracker
        fn insert_new_node(&mut self, node: (Yaml, usize)) {
            // When we have an alias, use position_tracker to get the referenced node
            if let (Yaml::Alias(id), _) = node {
                // Get the node from position_tracker instead of anchor_map
                if let Some(yaml) = self.position_tracker.get_anchor_yaml(id) {
                    // Process the resolved node
                    let actual_node = (yaml, 0);
                    self.process_node(actual_node);
                    return;
                }
            }

            // Process regular node
            self.process_node(node);
        }

        // Helper to process a node
        fn process_node(&mut self, node: (Yaml, usize)) {
            // Simplified implementation for testing
            self.docs.push(node.0.clone());
        }
    }

    impl MarkedEventReceiver for CustomLoader {
        fn on_event(&mut self, ev: yaml_rust2::parser::Event, mark: yaml_rust2::scanner::Marker) {
            // Process the event and update position_tracker
            let _span = self.position_tracker.process_event(&ev, mark);

            // Example of how to handle different events
            match ev {
                yaml_rust2::parser::Event::Scalar(value, _style, anchor_id, _tag) => {
                    // Example: Create Yaml node and insert it
                    let node = if anchor_id > 0 {
                        // Node with anchor
                        Yaml::String(value.clone())
                    } else {
                        // Regular node
                        Yaml::String(value)
                    };

                    self.insert_new_node((node, anchor_id));
                }
                yaml_rust2::parser::Event::Alias(anchor_id) => {
                    // For aliases, create a Yaml::Alias node
                    self.insert_new_node((Yaml::Alias(anchor_id), 0));
                }
                // Handle other events similarly
                _ => {}
            }
        }
    }

    // Create parser and custom loader
    let mut parser = Parser::new(s.chars());
    let mut loader = CustomLoader::new();

    // Use the load method to process events
    parser.load(&mut loader, true).unwrap();

    // Verify that we processed the document correctly
    assert!(!loader.docs.is_empty());

    // Verify that position_tracker has captured anchor nodes
    assert!(loader.position_tracker.get_anchor_yaml(1).is_some());
}
