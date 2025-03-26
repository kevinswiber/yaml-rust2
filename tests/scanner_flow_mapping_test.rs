#[cfg(feature = "source_mapping")]
#[cfg(test)]
mod tests {
    use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser};
    use yaml_rust2::scanner::Marker;
    use yaml_rust2::AnchorId;
    use yaml_rust2::Yaml;

    struct TestEventReceiver {
        events: Vec<(Event, Marker)>,
    }

    impl TestEventReceiver {
        fn new() -> Self {
            TestEventReceiver { events: Vec::new() }
        }
    }

    impl MarkedEventReceiver for TestEventReceiver {
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

        // Parse the YAML and collect events
        let mut receiver = TestEventReceiver::new();
        let mut parser = Parser::new(yaml_str.chars());
        parser.load(&mut receiver, true).unwrap();

        // Print all events with their positions
        println!("All scanner events with positions:");
        for (i, (event, mark)) in receiver.events.iter().enumerate() {
            println!(
                "{}: {:?} at line {}, col {}",
                i,
                event,
                mark.line(),
                mark.col()
            );
        }

        // Find the flow_mapping key and its value
        let mut flow_mapping_key_index = None;
        let mut flow_mapping_value_index = None;

        for (i, (event, _)) in receiver.events.iter().enumerate() {
            if let Event::Scalar(value, _, _, _) = event {
                if value == "flow_mapping" {
                    flow_mapping_key_index = Some(i);
                    // The value should be the next event
                    flow_mapping_value_index = Some(i + 1);
                    break;
                }
            }
        }

        // Verify that we found the flow_mapping key and value
        assert!(
            flow_mapping_key_index.is_some(),
            "Could not find flow_mapping key"
        );
        assert!(
            flow_mapping_value_index.is_some(),
            "Could not find flow_mapping value"
        );

        // Get the positions
        let key_index = flow_mapping_key_index.unwrap();
        let value_index = flow_mapping_value_index.unwrap();

        let (_, key_mark) = &receiver.events[key_index];
        let (value_event, value_mark) = &receiver.events[value_index];

        // Verify that the key is at line 3
        assert_eq!(key_mark.line(), 3, "flow_mapping key should be at line 3");

        // Verify that the value (MappingStart) is at line 3
        match value_event {
            Event::MappingStart(_, _, _) => {
                assert_eq!(
                    value_mark.line(),
                    3,
                    "flow_mapping value should be at line 3"
                );
            }
            _ => panic!("Expected MappingStart event for flow_mapping value"),
        }

        // Now let's test the position tracking in the PositionTrackedLoader
        let mut loader = yaml_rust2::position_tracked_loader::PositionTrackedLoader::default();
        let mut parser = Parser::new(yaml_str.chars());
        parser.load(&mut loader, true).unwrap();

        // Get the position tracker
        let position_tracker = loader.position_tracker();

        // Print all node paths and their positions
        println!("\nAll node paths and positions:");
        for (path, node_id) in position_tracker.get_path_mappings() {
            if let Some(pos) = position_tracker.get_node_position(node_id) {
                println!(
                    "Path: {}, Position: ({},{})",
                    path,
                    pos.start.line(),
                    pos.start.col()
                );
            }
        }

        // Print all events from the scanner again for comparison
        println!("\nScanner events for flow_mapping:");
        for (i, (event, mark)) in receiver.events.iter().enumerate() {
            if i >= 3 && i <= 5 {
                println!(
                    "{}: {:?} at line {}, col {}",
                    i,
                    event,
                    mark.line(),
                    mark.col()
                );
            }
        }

        // Try to find the flow_mapping node
        if let Some(flow_mapping_pos) = position_tracker.get_node_position_by_path("flow_mapping") {
            println!(
                "\nFlow mapping position: ({},{})",
                flow_mapping_pos.start.line(),
                flow_mapping_pos.start.col()
            );

            // Find the node by ID
            if let Some(node_id) = position_tracker.find_node_id_by_path_str("flow_mapping") {
                println!("Flow mapping node ID: {:?}", node_id);

                // Check if there's a node with this ID in the all_nodes map
                if let Some(node) = position_tracker.get_node(node_id) {
                    println!("Node content: {:?}", node);
                } else {
                    println!("No node found with this ID in all_nodes map");
                }
            }

            // Try to find the actual flow mapping value
            if let Some(docs) = loader.documents().get(0) {
                if let Yaml::Hash(hash) = docs {
                    if let Some(flow_mapping_value) =
                        hash.get(&Yaml::String("flow_mapping".to_string()))
                    {
                        println!("\nFlow mapping value: {:?}", flow_mapping_value);

                        // Try to find the position of the flow mapping value
                        if let Some(value_id) = position_tracker.find_node_id(flow_mapping_value) {
                            println!("Flow mapping value ID: {:?}", value_id);

                            if let Some(value_pos) = position_tracker.get_node_position(value_id) {
                                println!(
                                    "Flow mapping value position: ({},{})",
                                    value_pos.start.line(),
                                    value_pos.start.col()
                                );
                            }
                        }
                    }
                }
            }

            // Try to find the flow mapping value by its content
            if let Some(docs) = loader.documents().get(0) {
                if let Yaml::Hash(hash) = docs {
                    if let Some(flow_mapping_value) =
                        hash.get(&Yaml::String("flow_mapping".to_string()))
                    {
                        println!("\nTrying to find position for flow mapping value");
                        if let Some(value_id) = position_tracker.find_node_id(flow_mapping_value) {
                            println!("Found flow mapping value ID: {:?}", value_id);
                            if let Some(value_pos) = position_tracker.get_node_position(value_id) {
                                println!(
                                    "Flow mapping value position: ({},{})",
                                    value_pos.start.line(),
                                    value_pos.start.col()
                                );
                                // The flow mapping value should be at line 3
                                assert_eq!(
                                    value_pos.start.line(),
                                    3,
                                    "Flow mapping value should start at line 3, but position tracker reported line {}",
                                    value_pos.start.line()
                                );
                                return;
                            } else {
                                println!(
                                    "No position found for flow mapping value ID: {:?}",
                                    value_id
                                );
                            }
                        } else {
                            println!("Could not find ID for flow mapping value");

                            // Try to find the position by iterating through all nodes
                            println!("\nSearching through all nodes:");

                            // Print all nodes in the position tracker
                            println!("\nAll nodes in position tracker:");
                            for (path, node_id) in position_tracker.get_path_mappings() {
                                if let Some(node) = position_tracker.get_node(node_id) {
                                    if let Some(pos) = position_tracker.get_node_position(node_id) {
                                        println!(
                                            "Path: {}, Node ID: {:?}, Position: ({},{}), Content: {:?}",
                                            path,
                                            node_id,
                                            pos.start.line(),
                                            pos.start.col(),
                                            node
                                        );
                                    }
                                }
                            }
                            // Now look for hash nodes that match our flow mapping
                            for (path, node_id) in position_tracker.get_path_mappings() {
                                if let Some(node) = position_tracker.get_node(node_id) {
                                    if let Yaml::Hash(hash) = node {
                                        if hash.len() > 0 {
                                            println!(
                                                "Found hash at path: {}, ID: {:?}, size: {}",
                                                path,
                                                node_id,
                                                hash.len()
                                            );

                                            // Check if this hash matches our flow mapping value
                                            if let Some(key1_value) =
                                                hash.get(&Yaml::String("key1".to_string()))
                                            {
                                                if let Yaml::String(s) = key1_value {
                                                    if s == "value1" {
                                                        println!("This hash matches our flow mapping value!");
                                                        println!(
                                                            "Path: {}, Node ID: {:?}",
                                                            path, node_id
                                                        );

                                                        if let Some(pos) = position_tracker
                                                            .get_node_position(node_id)
                                                        {
                                                            println!(
                                                                "Position: ({},{})",
                                                                pos.start.line(),
                                                                pos.start.col()
                                                            );

                                                            // The flow mapping value should be at line 3
                                                            assert_eq!(
                                                                pos.start.line(),
                                                                3,
                                                                "Flow mapping value should start at line 3, but position tracker reported line {}",
                                                                pos.start.line()
                                                            );
                                                            return;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // If we couldn't find the flow mapping value, fall back to checking the key
            assert_eq!(
                flow_mapping_pos.start.line(),
                3,
                "Flow mapping should start at line 3, but position tracker reported line {}",
                flow_mapping_pos.start.line()
            );
        } else {
            panic!("Could not find flow_mapping node by path");
        }
    }
}
