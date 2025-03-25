use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser};
use yaml_rust2::position::PositionSpan;
use yaml_rust2::scanner::{Marker, TMappingStyle};
use yaml_rust2::style::TSequenceStyle;
use yaml_rust2::ScanError;

// Custom test loader to capture events and their positions
struct TestLoader {
    events: Vec<(Event, PositionSpan)>,
    error: Option<ScanError>,
    // Stack to track flow collection start positions
    flow_collection_stack: Vec<(Event, Marker)>,
}

impl TestLoader {
    fn new() -> Self {
        TestLoader {
            events: Vec::new(),
            error: None,
            flow_collection_stack: Vec::new(),
        }
    }

    fn get_events(&self) -> &[(Event, PositionSpan)] {
        &self.events
    }

    fn on_event_impl(&mut self, ev: Event, mark: Marker) -> Result<(), ScanError> {
        match &ev {
            // For flow mapping start events, push to the stack
            Event::MappingStart(_, _, style) if *style == TMappingStyle::Flow => {
                self.flow_collection_stack.push((ev.clone(), mark));
                let span = PositionSpan {
                    start: mark,
                    end: None, // End position will be set when we encounter the end event
                };
                self.events.push((ev, span));
            }
            // For sequence start events, push to the stack
            // Note: SequenceStart doesn't have a style parameter, so we'll treat all as flow for testing
            Event::SequenceStart(_anchor_id, _tag, _style) => {
                self.flow_collection_stack.push((ev.clone(), mark));
                let span = PositionSpan {
                    start: mark,
                    end: None, // End position will be set when we encounter the end event
                };
                self.events.push((ev, span));
            }
            // For flow mapping end events, pop from the stack
            Event::MappingEnd => {
                if let Some((start_event, start_mark)) = self.flow_collection_stack.pop() {
                    if let Event::MappingStart(_, _, TMappingStyle::Flow) = start_event {
                        let span = PositionSpan {
                            start: start_mark,
                            end: Some(mark), // Set the end position
                        };
                        self.events.push((ev, span));
                    } else {
                        // Unexpected stack content
                        let span = PositionSpan {
                            start: mark,
                            end: Some(mark),
                        };
                        self.events.push((ev, span));
                    }
                } else {
                    // Stack is empty
                    let span = PositionSpan {
                        start: mark,
                        end: Some(mark),
                    };
                    self.events.push((ev, span));
                }
            }
            // For flow sequence end events, pop from the stack
            Event::SequenceEnd => {
                if let Some((start_event, start_mark)) = self.flow_collection_stack.pop() {
                    if let Event::SequenceStart(_anchor_id, _tag, _style) = start_event {
                        let span = PositionSpan {
                            start: start_mark,
                            end: Some(mark), // Set the end position
                        };
                        self.events.push((ev, span));
                    } else {
                        // Unexpected stack content
                        let span = PositionSpan {
                            start: mark,
                            end: Some(mark),
                        };
                        self.events.push((ev, span));
                    }
                } else {
                    // Stack is empty
                    let span = PositionSpan {
                        start: mark,
                        end: Some(mark),
                    };
                    self.events.push((ev, span));
                }
            }
            // For all other events, use the same marker for start and end
            _ => {
                let span = PositionSpan {
                    start: mark,
                    end: Some(mark),
                };
                self.events.push((ev, span));
            }
        }

        Ok(())
    }
}

impl MarkedEventReceiver for TestLoader {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        if self.error.is_some() {
            return;
        }
        if let Err(e) = self.on_event_impl(ev, mark) {
            self.error = Some(e);
        }
    }
}

#[test]
fn test_flow_collection_position_spans() {
    // Test with a simple flow mapping
    let s = "{key1: value1, key2: value2}";
    let mut parser = Parser::new(s.chars());
    let mut loader = TestLoader::new();

    // Process all events
    parser.load(&mut loader, true).unwrap();

    // Get the events
    let events = loader.get_events();

    // Find the MappingStart event
    let mapping_start = events
        .iter()
        .find(|(event, _)| matches!(event, Event::MappingStart(_, _, TMappingStyle::Flow)))
        .expect("Expected MappingStart event");

    // Check the MappingStart position
    assert_eq!(mapping_start.1.start.line(), 1);
    assert_eq!(mapping_start.1.start.col(), 0);

    // Find the MappingEnd event
    let mapping_end = events
        .iter()
        .find(|(event, _)| matches!(event, Event::MappingEnd))
        .expect("Expected MappingEnd event");

    // Check the MappingEnd position span
    assert_eq!(mapping_end.1.start.line(), 1);
    assert_eq!(mapping_end.1.start.col(), 0);
    assert!(mapping_end.1.end.is_some());
    assert_eq!(mapping_end.1.end.unwrap().line(), 1);
    assert_eq!(mapping_end.1.end.unwrap().col(), s.len() - 1);

    // Test with a nested flow collection
    let s2 = "{outer: {inner1: value1, inner2: [item1, item2]}}";
    let mut parser = Parser::new(s2.chars());
    let mut loader = TestLoader::new();

    // Process all events
    parser.load(&mut loader, true).unwrap();

    // Get the events
    let events = loader.get_events();

    // Find all mapping and sequence events
    let mapping_events = events
        .iter()
        .filter(|(event, _)| {
            matches!(
                event,
                Event::MappingStart(_, _, TMappingStyle::Flow)
                    | Event::MappingEnd
                    | Event::SequenceStart(_, _, TSequenceStyle::Flow)
                    | Event::SequenceEnd
            )
        })
        .collect::<Vec<_>>();

    // We should have 6 events: outer start, inner start, sequence start, sequence end, inner end, outer end
    assert_eq!(mapping_events.len(), 6, "Expected 6 flow collection events");

    // Check the outer MappingStart position
    let outer_start = mapping_events[0];
    match outer_start.0 {
        Event::MappingStart(_, _, TMappingStyle::Flow) => {
            assert_eq!(outer_start.1.start.line(), 1);
            assert_eq!(outer_start.1.start.col(), 0);
        }
        _ => panic!("Expected outer MappingStart event"),
    }

    // Check the inner MappingStart position
    let inner_start = mapping_events[1];
    match inner_start.0 {
        Event::MappingStart(_, _, TMappingStyle::Flow) => {
            assert_eq!(inner_start.1.start.line(), 1);
            assert_eq!(inner_start.1.start.col(), 8);
        }
        _ => panic!("Expected inner MappingStart event"),
    }

    // Check the SequenceStart position
    let sequence_start = mapping_events[2];
    match sequence_start.0 {
        Event::SequenceStart(_, _, TSequenceStyle::Flow) => {
            assert_eq!(sequence_start.1.start.line(), 1);
            assert_eq!(sequence_start.1.start.col(), 33);
        }
        _ => panic!("Expected SequenceStart event"),
    }

    // Check the SequenceEnd position
    let sequence_end = mapping_events[3];
    match sequence_end.0 {
        Event::SequenceEnd => {
            assert_eq!(sequence_end.1.start.line(), 1);
            assert_eq!(sequence_end.1.start.col(), 33);
            assert!(sequence_end.1.end.is_some());
            assert_eq!(sequence_end.1.end.unwrap().line(), 1);
            assert_eq!(sequence_end.1.end.unwrap().col(), 46);
        }
        _ => panic!("Expected SequenceEnd event"),
    }

    // Check the inner MappingEnd position
    let inner_end = mapping_events[4];
    match inner_end.0 {
        Event::MappingEnd => {
            assert_eq!(inner_end.1.start.line(), 1);
            assert_eq!(inner_end.1.start.col(), 8);
            assert!(inner_end.1.end.is_some());
            assert_eq!(inner_end.1.end.unwrap().line(), 1);
            assert_eq!(inner_end.1.end.unwrap().col(), 47);
        }
        _ => panic!("Expected inner MappingEnd event"),
    }

    // Check the outer MappingEnd position
    let outer_end = mapping_events[5];
    match outer_end.0 {
        Event::MappingEnd => {
            assert_eq!(outer_end.1.start.line(), 1);
            assert_eq!(outer_end.1.start.col(), 0);
            assert!(outer_end.1.end.is_some());
            assert_eq!(outer_end.1.end.unwrap().line(), 1);
            assert_eq!(outer_end.1.end.unwrap().col(), 48);
        }
        _ => panic!("Expected outer MappingEnd event"),
    }
}
