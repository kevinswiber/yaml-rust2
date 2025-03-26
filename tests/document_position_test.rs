#[cfg(test)]
mod tests {
    use yaml_rust2::parser::{Event, MarkedEventReceiver, Parser};
    use yaml_rust2::scanner::Marker;
    #[cfg(feature = "source_mapping")]
    use yaml_rust2::position_tracked_loader::PositionTrackedLoader;
    use yaml_rust2::AnchorId;

    // Simple event receiver that logs events with their positions
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
    fn test_document_positions() {
        // Test implicit document (no document start marker)
        let implicit_yaml = r#"key: value
another: value
"#;

        let mut implicit_receiver = PositionTestReceiver::new();
        let mut implicit_parser = Parser::new(implicit_yaml.chars());
        implicit_parser.load(&mut implicit_receiver, true).unwrap();

        println!("Events for implicit document:");
        for (i, (event, mark)) in implicit_receiver.get_events().iter().enumerate() {
            println!("  {}: {:?} at line {}, col {}", i, event, mark.line(), mark.col());
        }

        // Test explicit document (with document start marker)
        let explicit_yaml = r#"---
key: value
another: value
"#;

        let mut explicit_receiver = PositionTestReceiver::new();
        let mut explicit_parser = Parser::new(explicit_yaml.chars());
        explicit_parser.load(&mut explicit_receiver, true).unwrap();

        println!("Events for explicit document:");
        for (i, (event, mark)) in explicit_receiver.get_events().iter().enumerate() {
            println!("  {}: {:?} at line {}, col {}", i, event, mark.line(), mark.col());
        }

        // Find the DocumentStart event in both cases
        let implicit_doc_start = implicit_receiver
            .get_events()
            .iter()
            .find(|(event, _)| matches!(event, Event::DocumentStart))
            .map(|(_, mark)| mark.col())
            .unwrap();

        let explicit_doc_start = explicit_receiver
            .get_events()
            .iter()
            .find(|(event, _)| matches!(event, Event::DocumentStart))
            .map(|(_, mark)| mark.col())
            .unwrap();

        // The implicit document start should be at column 0, just like explicit document start
        assert_eq!(implicit_doc_start, 0, "Implicit DocumentStart should be at column 0");
        assert_eq!(explicit_doc_start, 0, "Explicit DocumentStart should be at column 0");
    }

    #[test]
    fn test_document_positions_with_comment() {
        // Test implicit document with comment (no document start marker)
        let implicit_yaml = r#"# This is a comment
key: value
another: value
"#;

        let mut implicit_receiver = PositionTestReceiver::new();
        let mut implicit_parser = Parser::new(implicit_yaml.chars());
        implicit_parser.load(&mut implicit_receiver, true).unwrap();

        println!("Events for implicit document with comment:");
        for (i, (event, mark)) in implicit_receiver.get_events().iter().enumerate() {
            println!("  {}: {:?} at line {}, col {}", i, event, mark.line(), mark.col());
        }

        // Test explicit document with comment (with document start marker)
        let explicit_yaml = r#"# This is a comment
---
key: value
another: value
"#;

        let mut explicit_receiver = PositionTestReceiver::new();
        let mut explicit_parser = Parser::new(explicit_yaml.chars());
        explicit_parser.load(&mut explicit_receiver, true).unwrap();

        println!("Events for explicit document with comment:");
        for (i, (event, mark)) in explicit_receiver.get_events().iter().enumerate() {
            println!("  {}: {:?} at line {}, col {}", i, event, mark.line(), mark.col());
        }

        // Find the DocumentStart event in both cases
        let implicit_doc_start = implicit_receiver
            .get_events()
            .iter()
            .find(|(event, _)| matches!(event, Event::DocumentStart))
            .map(|(_, mark)| (mark.line(), mark.col()))
            .unwrap();

        let explicit_doc_start = explicit_receiver
            .get_events()
            .iter()
            .find(|(event, _)| matches!(event, Event::DocumentStart))
            .map(|(_, mark)| (mark.line(), mark.col()))
            .unwrap();

        // For documents with a leading comment, DocumentStart should be at line 2, column 0
        // Line 2 because line numbers are 1-based and we have one comment line before content
        assert_eq!(
            implicit_doc_start, 
            (2, 0), 
            "Implicit DocumentStart with comment should be at line 2, column 0"
        );
        assert_eq!(
            explicit_doc_start, 
            (2, 0), 
            "Explicit DocumentStart with comment should be at line 2, column 0"
        );
    }
}