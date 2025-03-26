#[cfg(test)]
mod tests {
    use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser};
    use yaml_rust2::scanner::Marker;
    use yaml_rust2::AnchorId;

    struct PositionTestReceiver {
        events: Vec<(Event, Marker)>,
    }

    impl PositionTestReceiver {
        fn new() -> Self {
            PositionTestReceiver { events: Vec::new() }
        }

        fn get_events(&self) -> &[(Event, Marker)] {
            &self.events
        }
    }

    impl MarkedEventReceiver for PositionTestReceiver {
        fn on_event(&mut self, ev: Event, mark: Marker) {
            self.events.push((ev, mark));
        }
    }

    #[test]
    fn test_scanner_flow_mapping_position() {
        // This is the same YAML string used in the automatic_position_tracking_test
        let yaml_str = r#"
# Test with flow collections
flow_mapping: {key1: value1, key2: [item1, item2, {nested_key: nested_value}]}
mixed:
  block_key: {flow_key: flow_value}
  flow_seq: [1, 2, 3]
"#;

        let mut receiver = PositionTestReceiver::new();
        let mut parser = Parser::new(yaml_str.chars());
        parser.load(&mut receiver, true).unwrap();

        // Print all events with their positions for debugging
        println!("All scanner events with positions:");
        for (i, (event, mark)) in receiver.get_events().iter().enumerate() {
            println!(
                "{}: {:?} at line {}, col {}",
                i,
                event,
                mark.line(),
                mark.col()
            );
        }

        // Find the MappingStart event for the flow mapping
        let flow_mapping_event_index = receiver
            .get_events()
            .iter()
            .enumerate()
            .find(|(_, (event, _))| matches!(event, Event::MappingStart(_, _, _)))
            .map(|(i, _)| i);

        if let Some(index) = flow_mapping_event_index {
            let (event, mark) = &receiver.get_events()[index];
            if let Event::MappingStart(_, _, style) = event {
                println!(
                    "Found flow mapping at line {}, col {}, style: {:?}",
                    mark.line(),
                    mark.col(),
                    style
                );

                // Check if there's a scalar event before this that might be the key
                if index > 0 {
                    let (prev_event, prev_mark) = &receiver.get_events()[index - 1];
                    if let Event::Scalar(value, _, _, _) = prev_event {
                        println!(
                            "Previous event: Scalar({}) at line {}, col {}",
                            value,
                            prev_mark.line(),
                            prev_mark.col()
                        );
                    }
                }

                // The flow mapping should be at line 3
                assert_eq!(
                    mark.line(),
                    3,
                    "Flow mapping should start at line 3, but scanner reported line {}",
                    mark.line()
                );
            }
        } else {
            panic!("Could not find MappingStart event for flow mapping");
        }
    }
}
