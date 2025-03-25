use yaml_rust2::{
    parser::{Event, Tag},
    scanner::TMappingStyle,
    style::TSequenceStyle,
    AnchorId,
};

#[test]
fn test_event_with_style_information() {
    // Test sequence events with style
    let seq_flow = Event::SequenceStart(AnchorId::new(1), None, TSequenceStyle::Flow);
    let seq_block = Event::SequenceStart(AnchorId::new(2), None, TSequenceStyle::Block);
    
    // Test mapping events with style
    let map_flow = Event::MappingStart(AnchorId::new(3), None, TMappingStyle::Flow);
    let map_block = Event::MappingStart(AnchorId::new(4), None, TMappingStyle::Block);
    
    // Verify sequence styles
    if let Event::SequenceStart(_, _, style) = seq_flow {
        assert_eq!(style, TSequenceStyle::Flow);
    } else {
        panic!("Expected SequenceStart event");
    }
    
    if let Event::SequenceStart(_, _, style) = seq_block {
        assert_eq!(style, TSequenceStyle::Block);
    } else {
        panic!("Expected SequenceStart event");
    }
    
    // Verify mapping styles
    if let Event::MappingStart(_, _, style) = map_flow {
        assert_eq!(style, TMappingStyle::Flow);
    } else {
        panic!("Expected MappingStart event");
    }
    
    if let Event::MappingStart(_, _, style) = map_block {
        assert_eq!(style, TMappingStyle::Block);
    } else {
        panic!("Expected MappingStart event");
    }
}