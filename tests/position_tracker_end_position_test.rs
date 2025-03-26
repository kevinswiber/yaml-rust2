use yaml_rust2::parser::{Event, Parser};
use yaml_rust2::position::PositionTracker;
use yaml_rust2::scanner::{Marker, TMappingStyle, TScalarStyle};
use yaml_rust2::style::TSequenceStyle;
use yaml_rust2::yaml::Yaml;
use yaml_rust2::AnchorId;

#[test]
fn test_last_entry_end_position() {
    // Create a position tracker
    let mut tracker = PositionTracker::new();

    // Create markers at different positions
    let mark1 = Marker::new(0, 1, 0);
    let mark2 = Marker::new(2, 1, 2);
    let mark3 = Marker::new(9, 1, 9);
    let mark4 = Marker::new(16, 1, 16);

    // Simulate processing events by pushing to position stack
    let event1 = Event::MappingStart(AnchorId::new(0), None, TMappingStyle::Flow);
    let span1 = tracker.process_event(&event1, mark1);
    println!(
        "span1: ({}, {}) to ({}, {})",
        span1.start.line(),
        span1.start.col(),
        span1.end.unwrap_or(Marker::new(0, 0, 0)).line(),
        span1.end.unwrap_or(Marker::new(0, 0, 0)).col()
    );
    println!("{:?}", tracker.peek().unwrap());
    // Process a scalar event (key)
    let event2 = Event::Scalar(
        "key1".to_string(),
        TScalarStyle::Plain,
        AnchorId::new(0),
        None,
    );
    let span2 = tracker.process_event(&event2, mark2);
    println!(
        "span2: ({}, {}) to ({}, {})",
        span2.start.line(),
        span2.start.col(),
        span2.end.unwrap_or(Marker::new(0, 0, 0)).line(),
        span2.end.unwrap_or(Marker::new(0, 0, 0)).col()
    );
    println!("{:?}", tracker.peek().unwrap());

    // Process another scalar event (value)
    let event3 = Event::Scalar(
        "value1".to_string(),
        TScalarStyle::Plain,
        AnchorId::new(0),
        None,
    );
    let span3 = tracker.process_event(&event3, mark3);
    println!(
        "span3: ({}, {}) to ({}, {})",
        span3.start.line(),
        span3.start.col(),
        span3.end.unwrap_or(Marker::new(0, 0, 0)).line(),
        span3.end.unwrap_or(Marker::new(0, 0, 0)).col()
    );
    println!("{:?}", tracker.peek().unwrap());

    let event4 = Event::MappingEnd;
    let span4 = tracker.process_event(&event4, mark4);
    println!(
        "span4: ({}, {}) to ({}, {})",
        span4.start.line(),
        span4.start.col(),
        span4.end.unwrap_or(Marker::new(0, 0, 0)).line(),
        span4.end.unwrap_or(Marker::new(0, 0, 0)).col()
    );

    // Now check if the last entry end position is correctly tracked
    assert_eq!(
        span4.end.unwrap().line(),
        1,
        "Last position should be on line 1"
    );
    assert_eq!(
        span4.end.unwrap().col(),
        16,
        "Last position should be at column 16"
    );
    assert_eq!(
        span4.end.unwrap().index(),
        16,
        "Last position should have index 16"
    );

    // Test with a block mapping
    let yaml = "
block_map:
  key1: value1
  key2: value2
";

    let mut tracker = PositionTracker::new();
    let mut parser = Parser::new(yaml.chars());

    // Process all events
    while let Ok((event, mark)) = parser.next_token() {
        tracker.process_event(&event, mark);

        // After processing a scalar value, check the last entry position
        if let Event::Scalar(value, _, _, _) = &event {
            if value == "value1" || value == "value2" {
                // After a value, the last entry position should be at the end of that value
                let last_pos = tracker.peek().map(|entry| entry.1).unwrap();
                assert!(
                    last_pos.col() > 0,
                    "Last position column should be positive after value"
                );
            }
        }
    }
}

#[test]
fn test_mapping_end_positions() {
    // Test with a flow mapping
    let yaml = "{key1: value1, key2: value2}";

    let mut tracker = PositionTracker::new();
    let mut parser = Parser::new(yaml.chars());
    let mut mapping_start_mark = Marker::new(0, 1, 0);
    let mut mapping_end_mark = Marker::new(0, 1, 27);

    // Process all events and capture mapping start/end positions
    while let Ok((event, mark)) = parser.next_token() {
        match &event {
            Event::MappingStart(_, _, _) => {
                mapping_start_mark = mark;
            }
            Event::MappingEnd => {
                mapping_end_mark = mark;
                // The end mark should be at the closing brace
                assert_eq!(
                    mapping_end_mark.col(),
                    yaml.len() - 1,
                    "Mapping end should be at the closing brace"
                );
            }
            _ => {}
        }

        tracker.process_event(&event, mark);
    }

    // Test with a block mapping
    let yaml = "
block_map:
  key1: value1
  key2: value2
";

    let mut tracker = PositionTracker::new();
    let mut parser = Parser::new(yaml.chars());

    // Process all events
    while let Ok((event, mark)) = parser.next_token() {
        tracker.process_event(&event, mark);

        // For MappingEnd events, verify we're using the last entry position
        if let Event::MappingEnd = &event {
            // The last entry position should be after the last value
            let last_pos = tracker.peek().map(|entry| entry.1).unwrap_or(mark);
            assert!(last_pos.line() > 0, "Last position line should be positive");
        }
    }
}
