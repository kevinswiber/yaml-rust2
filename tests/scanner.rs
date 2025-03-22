#![allow(clippy::enum_glob_use)]

use yaml_rust2::{scanner::TokenType::*, scanner::*};

macro_rules! next {
    ($p:ident, $tk:pat) => {{
        let tok = $p.next().unwrap();
        match tok.1 {
            $tk => {}
            _ => panic!("unexpected token: {:?}", tok),
        }
    }};
}

macro_rules! next_scalar {
    ($p:ident, $tk:expr, $v:expr) => {{
        let tok = $p.next().unwrap();
        match tok.1 {
            Scalar(style, ref v) => {
                assert_eq!(style, $tk);
                assert_eq!(v, $v);
            }
            _ => panic!("unexpected token: {:?}", tok),
        }
    }};
}

macro_rules! end {
    ($p:ident) => {{
        assert_eq!($p.next(), None);
    }};
}
/// test cases in libyaml scanner.c
#[test]
fn test_empty() {
    let s = "";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_scalar() {
    let s = "a scalar";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, Scalar(TScalarStyle::Plain, _));
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_explicit_scalar() {
    let s = "---
'a scalar'
...
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, DocumentStart);
    next!(p, Scalar(TScalarStyle::SingleQuoted, _));
    next!(p, DocumentEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_multiple_documents() {
    let s = "
'a scalar'
---
'a scalar'
---
'a scalar'
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, Scalar(TScalarStyle::SingleQuoted, _));
    next!(p, DocumentStart);
    next!(p, Scalar(TScalarStyle::SingleQuoted, _));
    next!(p, DocumentStart);
    next!(p, Scalar(TScalarStyle::SingleQuoted, _));
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_a_flow_sequence() {
    let s = "[item 1, item 2, item 3]";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, FlowSequenceStart);
    next_scalar!(p, TScalarStyle::Plain, "item 1");
    next!(p, FlowEntry);
    next!(p, Scalar(TScalarStyle::Plain, _));
    next!(p, FlowEntry);
    next!(p, Scalar(TScalarStyle::Plain, _));
    next!(p, FlowSequenceEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_a_flow_mapping() {
    let s = "
{
    a simple key: a value, # Note that the KEY token is produced.
    ? a complex key: another value,
}
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, FlowMappingStart);
    next!(p, Key);
    next!(p, Scalar(TScalarStyle::Plain, _));
    next!(p, Value);
    next!(p, Scalar(TScalarStyle::Plain, _));
    next!(p, FlowEntry);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "a complex key");
    next!(p, Value);
    next!(p, Scalar(TScalarStyle::Plain, _));
    next!(p, FlowEntry);
    next!(p, FlowMappingEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_block_sequences() {
    let s = "
- item 1
- item 2
-
  - item 3.1
  - item 3.2
-
  key 1: value 1
  key 2: value 2
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, BlockSequenceStart);
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 1");
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 2");
    next!(p, BlockEntry);
    next!(p, BlockSequenceStart);
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 3.1");
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 3.2");
    next!(p, BlockEnd);
    next!(p, BlockEntry);
    next!(p, BlockMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "key 1");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "value 1");
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "key 2");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "value 2");
    next!(p, BlockEnd);
    next!(p, BlockEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_block_mappings() {
    let s = "
a simple key: a value   # The KEY token is produced here.
? a complex key
: another value
a mapping:
  key 1: value 1
  key 2: value 2
a sequence:
  - item 1
  - item 2
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, BlockMappingStart);
    next!(p, Key);
    next!(p, Scalar(_, _));
    next!(p, Value);
    next!(p, Scalar(_, _));
    next!(p, Key);
    next!(p, Scalar(_, _));
    next!(p, Value);
    next!(p, Scalar(_, _));
    next!(p, Key);
    next!(p, Scalar(_, _));
    next!(p, Value); // libyaml comment seems to be wrong
    next!(p, BlockMappingStart);
    next!(p, Key);
    next!(p, Scalar(_, _));
    next!(p, Value);
    next!(p, Scalar(_, _));
    next!(p, Key);
    next!(p, Scalar(_, _));
    next!(p, Value);
    next!(p, Scalar(_, _));
    next!(p, BlockEnd);
    next!(p, Key);
    next!(p, Scalar(_, _));
    next!(p, Value);
    next!(p, BlockSequenceStart);
    next!(p, BlockEntry);
    next!(p, Scalar(_, _));
    next!(p, BlockEntry);
    next!(p, Scalar(_, _));
    next!(p, BlockEnd);
    next!(p, BlockEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_no_block_sequence_start() {
    let s = "
key:
- item 1
- item 2
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, BlockMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "key");
    next!(p, Value);
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 1");
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 2");
    next!(p, BlockEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_collections_in_sequence() {
    let s = "
- - item 1
  - item 2
- key 1: value 1
  key 2: value 2
- ? complex key
  : complex value
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, BlockSequenceStart);
    next!(p, BlockEntry);
    next!(p, BlockSequenceStart);
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 1");
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 2");
    next!(p, BlockEnd);
    next!(p, BlockEntry);
    next!(p, BlockMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "key 1");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "value 1");
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "key 2");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "value 2");
    next!(p, BlockEnd);
    next!(p, BlockEntry);
    next!(p, BlockMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "complex key");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "complex value");
    next!(p, BlockEnd);
    next!(p, BlockEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_collections_in_mapping() {
    let s = "
? a sequence
: - item 1
  - item 2
? a mapping
: key 1: value 1
  key 2: value 2
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, BlockMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "a sequence");
    next!(p, Value);
    next!(p, BlockSequenceStart);
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 1");
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "item 2");
    next!(p, BlockEnd);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "a mapping");
    next!(p, Value);
    next!(p, BlockMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "key 1");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "value 1");
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "key 2");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "value 2");
    next!(p, BlockEnd);
    next!(p, BlockEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_spec_ex7_3() {
    let s = r"
{
    ? foo :,
    : bar,
}
";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, FlowMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "foo");
    next!(p, Value);
    next!(p, FlowEntry);
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "bar");
    next!(p, FlowEntry);
    next!(p, FlowMappingEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_plain_scalar_starting_with_indicators_in_flow() {
    // "Plain scalars must not begin with most indicators, as this would cause ambiguity with
    // other YAML constructs. However, the ":", "?" and "-" indicators may be used as the first
    // character if followed by a non-space "safe" character, as this causes no ambiguity."

    let s = "{a: :b}";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, FlowMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "a");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, ":b");
    next!(p, FlowMappingEnd);
    next!(p, StreamEnd);
    end!(p);

    let s = "{a: ?b}";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, FlowMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "a");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "?b");
    next!(p, FlowMappingEnd);
    next!(p, StreamEnd);
    end!(p);

    let s = "{a: -b}";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, FlowMappingStart);
    next!(p, Key);
    next_scalar!(p, TScalarStyle::Plain, "a");
    next!(p, Value);
    next_scalar!(p, TScalarStyle::Plain, "-b");
    next!(p, FlowMappingEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_plain_scalar_starting_with_indicators_in_block() {
    let s = ":a";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next_scalar!(p, TScalarStyle::Plain, ":a");
    next!(p, StreamEnd);
    end!(p);

    let s = "?a";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next_scalar!(p, TScalarStyle::Plain, "?a");
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_plain_scalar_containing_indicators_in_block() {
    let s = "a:,b";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next_scalar!(p, TScalarStyle::Plain, "a:,b");
    next!(p, StreamEnd);
    end!(p);

    let s = ":,b";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next_scalar!(p, TScalarStyle::Plain, ":,b");
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_scanner_cr() {
    let s = "---\r\n- tok1\r\n- tok2";
    let mut p = Scanner::new(s.chars());
    next!(p, StreamStart(..));
    next!(p, DocumentStart);
    next!(p, BlockSequenceStart);
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "tok1");
    next!(p, BlockEntry);
    next_scalar!(p, TScalarStyle::Plain, "tok2");
    next!(p, BlockEnd);
    next!(p, StreamEnd);
    end!(p);
}

#[test]
fn test_uri() {
    // TODO
}

#[test]
fn test_uri_escapes() {
    // TODO
}

/// Test that flow collection tokens have correct position information
#[test]
fn test_flow_collection_positions() {
    // Test with a simple flow mapping
    let s = "{key1: value1, key2: value2}";
    let mut scanner = Scanner::new(s.chars());
    
    // Skip the StreamStart token
    scanner.next().unwrap();
    
    // Get the FlowMappingStart token and check its position
    let flow_mapping_start = scanner.next().unwrap();
    match flow_mapping_start.1 {
        TokenType::FlowMappingStart => {
            // The position should be at the opening '{'
            assert_eq!(flow_mapping_start.0.line(), 1);
            assert_eq!(flow_mapping_start.0.col(), 0);
        }
        _ => panic!("Expected FlowMappingStart token"),
    }
    
    // Skip to the FlowMappingEnd token
    let mut tokens = Vec::new();
    let mut flow_mapping_end = None;
    
    while let Some(token) = scanner.next() {
        tokens.push(token.clone());
        if let TokenType::FlowMappingEnd = token.1 {
            flow_mapping_end = Some(token);
            break;
        }
    }
    
    // Check the FlowMappingEnd token position
    match flow_mapping_end {
        Some(token) => {
            match token.1 {
                TokenType::FlowMappingEnd => {
                    // The position should be at the closing '}'
                    assert_eq!(token.0.line(), 1);
                    assert_eq!(token.0.col(), s.len() - 1);
                }
                _ => panic!("Expected FlowMappingEnd token"),
            }
        }
        None => panic!("Failed to find FlowMappingEnd token"),
    }
    
    // Test with a nested flow collection
    let s2 = "{outer: {inner1: value1, inner2: [item1, item2]}}";
    let mut scanner = Scanner::new(s2.chars());
    
    // Skip the StreamStart token
    scanner.next().unwrap();
    
    // Get the outer FlowMappingStart token
    let outer_start = scanner.next().unwrap();
    match outer_start.1 {
        TokenType::FlowMappingStart => {
            assert_eq!(outer_start.0.line(), 1);
            assert_eq!(outer_start.0.col(), 0);
        }
        _ => panic!("Expected FlowMappingStart token"),
    }
    
    // Find the inner FlowMappingStart token
    let mut inner_start = None;
    while let Some(token) = scanner.next() {
        if let TokenType::FlowMappingStart = token.1 {
            inner_start = Some(token);
            break;
        }
    }
    
    // Check the inner FlowMappingStart token position
    match inner_start {
        Some(token) => {
            match token.1 {
                TokenType::FlowMappingStart => {
                    // The position should be at the inner opening '{'
                    assert_eq!(token.0.line(), 1);
                    assert_eq!(token.0.col(), 8);
                }
                _ => panic!("Expected inner FlowMappingStart token"),
            }
        }
        None => panic!("Failed to find inner FlowMappingStart token"),
    }
    
    // Find the FlowSequenceStart token
    let mut sequence_start = None;
    while let Some(token) = scanner.next() {
        if let TokenType::FlowSequenceStart = token.1 {
            sequence_start = Some(token);
            break;
        }
    }
    
    // Check the FlowSequenceStart token position
    match sequence_start {
        Some(token) => {
            match token.1 {
                TokenType::FlowSequenceStart => {
                    // The position should be at the opening '['
                    assert_eq!(token.0.line(), 1);
                    assert_eq!(token.0.col(), 33);
                }
                _ => panic!("Expected FlowSequenceStart token"),
            }
        }
        None => panic!("Failed to find FlowSequenceStart token"),
    }
    
    // Find the FlowSequenceEnd token
    let mut sequence_end = None;
    while let Some(token) = scanner.next() {
        if let TokenType::FlowSequenceEnd = token.1 {
            sequence_end = Some(token);
            break;
        }
    }
    
    // Check the FlowSequenceEnd token position
    match sequence_end {
        Some(token) => {
            match token.1 {
                TokenType::FlowSequenceEnd => {
                    // The position should be at the closing ']'
                    assert_eq!(token.0.line(), 1);
                    assert_eq!(token.0.col(), 46);
                }
                _ => panic!("Expected FlowSequenceEnd token"),
            }
        }
        None => panic!("Failed to find FlowSequenceEnd token"),
    }
    
    // Find the inner FlowMappingEnd token
    let mut inner_end = None;
    while let Some(token) = scanner.next() {
        if let TokenType::FlowMappingEnd = token.1 {
            inner_end = Some(token);
            break;
        }
    }
    
    // Check the inner FlowMappingEnd token position
    match inner_end {
        Some(token) => {
            match token.1 {
                TokenType::FlowMappingEnd => {
                    // The position should be at the inner closing '}'
                    assert_eq!(token.0.line(), 1);
                    assert_eq!(token.0.col(), 47);
                }
                _ => panic!("Expected inner FlowMappingEnd token"),
            }
        }
        None => panic!("Failed to find inner FlowMappingEnd token"),
    }
    
    // Find the outer FlowMappingEnd token
    let mut outer_end = None;
    while let Some(token) = scanner.next() {
        if let TokenType::FlowMappingEnd = token.1 {
            outer_end = Some(token);
            break;
        }
    }
    
    // Check the outer FlowMappingEnd token position
    match outer_end {
        Some(token) => {
            match token.1 {
                TokenType::FlowMappingEnd => {
                    // The position should be at the outer closing '}'
                    assert_eq!(token.0.line(), 1);
                    assert_eq!(token.0.col(), 48);
                }
                _ => panic!("Expected outer FlowMappingEnd token"),
            }
        }
        None => panic!("Failed to find outer FlowMappingEnd token"),
    }
}
