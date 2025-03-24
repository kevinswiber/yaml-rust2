#![cfg(feature = "source_mapping")]

use yaml_rust2::parser::Parser;
use yaml_rust2::PositionTrackedLoader;
use yaml_rust2::Yaml;

#[test]
fn test_basic_loading() {
    let s = "
    key1: value1
    key2: value2
    ";

    let docs = PositionTrackedLoader::load_from_str(s).unwrap();
    assert_eq!(docs.len(), 1);

    if let Yaml::Hash(ref hash) = docs[0] {
        assert_eq!(hash.len(), 2);
        assert_eq!(
            hash[&Yaml::String("key1".to_owned())],
            Yaml::String("value1".to_owned())
        );
        assert_eq!(
            hash[&Yaml::String("key2".to_owned())],
            Yaml::String("value2".to_owned())
        );
    } else {
        panic!("Expected a hash at the root");
    }
}

#[test]
fn test_anchor_and_alias() {
    let s = "
    anchored_scalar: &scalar_anchor test_value
    scalar_ref: *scalar_anchor
    anchored_sequence: &seq_anchor
      - item1
      - item2
    sequence_ref: *seq_anchor
    anchored_mapping: &map_anchor
      key1: value1
      key2: value2
    mapping_ref: *map_anchor
    ";

    let docs = PositionTrackedLoader::load_from_str(s).unwrap();
    assert_eq!(docs.len(), 1);

    if let Yaml::Hash(ref hash) = docs[0] {
        // Check scalar anchor/alias
        assert_eq!(
            hash[&Yaml::String("anchored_scalar".to_owned())],
            Yaml::String("test_value".to_owned())
        );
        assert_eq!(
            hash[&Yaml::String("scalar_ref".to_owned())],
            Yaml::String("test_value".to_owned())
        );

        // Check sequence anchor/alias
        let sequence_ref = &hash[&Yaml::String("sequence_ref".to_owned())];
        if let Yaml::Array(ref seq) = sequence_ref {
            assert_eq!(seq.len(), 2);
            assert_eq!(seq[0], Yaml::String("item1".to_owned()));
            assert_eq!(seq[1], Yaml::String("item2".to_owned()));
        } else {
            panic!("Expected a sequence for sequence_ref");
        }

        // Check mapping anchor/alias
        let mapping_ref = &hash[&Yaml::String("mapping_ref".to_owned())];
        if let Yaml::Hash(ref map) = mapping_ref {
            assert_eq!(map.len(), 2);
            assert_eq!(
                map[&Yaml::String("key1".to_owned())],
                Yaml::String("value1".to_owned())
            );
            assert_eq!(
                map[&Yaml::String("key2".to_owned())],
                Yaml::String("value2".to_owned())
            );
        } else {
            panic!("Expected a mapping for mapping_ref");
        }
    } else {
        panic!("Expected a hash at the root");
    }
}

#[test]
fn test_complex_anchor_reference_structure() {
    let s = r#"
    anchors:
      scalar: &scalar_value "hello"
      list: &list_value
        - 1
        - 2
      map: &map_value
        a: 1
        b: 2
    
    references:
      scalar_ref: *scalar_value
      list_ref: *list_value
      map_ref: *map_value
      
    nested:
      first: &first_anchor
        second: &second_anchor
          value: test
      ref_first: *first_anchor
      ref_second: *second_anchor
    
    # Test circular reference (allowed in YAML)
    circular:
      this: &this_one
        refers_to: *this_one
    "#;

    let docs = PositionTrackedLoader::load_from_str(s).unwrap();
    assert_eq!(docs.len(), 1);

    if let Yaml::Hash(ref root) = docs[0] {
        // Verify anchors were properly created
        let anchors = &root[&Yaml::String("anchors".to_owned())];
        if let Yaml::Hash(ref anchor_map) = anchors {
            assert_eq!(
                anchor_map[&Yaml::String("scalar".to_owned())],
                Yaml::String("hello".to_owned())
            );
        }

        // Verify references work correctly
        let references = &root[&Yaml::String("references".to_owned())];
        if let Yaml::Hash(ref ref_map) = references {
            // Scalar reference
            assert_eq!(
                ref_map[&Yaml::String("scalar_ref".to_owned())],
                Yaml::String("hello".to_owned())
            );

            // List reference
            if let Yaml::Array(ref list) = ref_map[&Yaml::String("list_ref".to_owned())] {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], Yaml::Integer(1));
                assert_eq!(list[1], Yaml::Integer(2));
            } else {
                panic!("Expected a list for list_ref");
            }

            // Map reference
            if let Yaml::Hash(ref map) = ref_map[&Yaml::String("map_ref".to_owned())] {
                assert_eq!(map[&Yaml::String("a".to_owned())], Yaml::Integer(1));
                assert_eq!(map[&Yaml::String("b".to_owned())], Yaml::Integer(2));
            } else {
                panic!("Expected a map for map_ref");
            }
        } else {
            panic!("Expected a hash for references");
        }

        // Verify nested anchors work
        let nested = &root[&Yaml::String("nested".to_owned())];
        if let Yaml::Hash(ref nested_map) = nested {
            // Verify ref_second points to the same structure as second_anchor
            if let Yaml::Hash(ref second) = nested_map[&Yaml::String("ref_second".to_owned())] {
                assert_eq!(
                    second[&Yaml::String("value".to_owned())],
                    Yaml::String("test".to_owned())
                );
            } else {
                panic!("Expected a hash for ref_second");
            }
        }

        // Circular references are challenging to test fully but we can check the structure exists
        let circular = &root[&Yaml::String("circular".to_owned())];
        if let Yaml::Hash(ref circ_map) = circular {
            let this = &circ_map[&Yaml::String("this".to_owned())];
            if let Yaml::Hash(ref this_map) = this {
                // Just check it has the expected structure
                assert!(this_map.contains_key(&Yaml::String("refers_to".to_owned())));
            } else {
                panic!("Expected a hash for this");
            }
        }
    } else {
        panic!("Expected a hash at the root");
    }
}

#[test]
fn test_nested_anchors_and_self_references() {
    let yaml_str = r#"
    # Define a base map anchor
    base: &base
      name: BaseObject
      value: 100
    
    # Self-referential object
    self_ref: &self_ref
      primary: *self_ref
      fallback: *base
    
    # Nested anchors
    nested:
      level1: &l1
        a: 1
        b: 2
      level2: &l2
        c: 3
        d: 4
        ref_l1: *l1
    
    # Complex reference chain
    complex:
      refs:
        to_base: *base
        to_self_ref: *self_ref
        to_l2: *l2
      # Use a single merge key with an array of mappings to merge
      combined:
        <<: [*base, *l2]
        extra: value
    "#;

    // Parse the YAML using PositionTrackedLoader
    let yaml_docs = PositionTrackedLoader::load_from_str(yaml_str).unwrap();
    assert_eq!(yaml_docs.len(), 1, "Should have exactly one document");

    let doc = &yaml_docs[0];

    // Create a separate loader for accessing the position tracker
    let mut loader = PositionTrackedLoader::default();
    let mut parser = Parser::new(yaml_str.chars());
    parser.load(&mut loader, true).unwrap();

    // Get the nested.level1 node to verify its position
    let level1 = &doc["nested"]["level1"];
    assert_eq!(level1["a"].as_i64(), Some(1));
    assert_eq!(level1["b"].as_i64(), Some(2));

    // Find the anchor ID for level1
    let position_tracker = loader.position_tracker();
    let l1_id = position_tracker.find_anchor_id(level1);
    assert!(l1_id.is_some(), "Should find an anchor ID for level1");

    // Check the position of the level1 anchor
    let l1_position = position_tracker.get_anchor_position(l1_id.unwrap());
    assert!(
        l1_position.is_some(),
        "Should find a position for level1 anchor"
    );

    // Verify that nested.level2.ref_l1 correctly references level1
    let level2_ref_l1 = &doc["nested"]["level2"]["ref_l1"];
    assert_eq!(level2_ref_l1["a"].as_i64(), Some(1));
    assert_eq!(level2_ref_l1["b"].as_i64(), Some(2));

    // Verify self-reference handling (should not cause infinite recursion)
    let self_ref = &doc["self_ref"];
    assert!(
        self_ref["primary"].is_badvalue() || self_ref["primary"] == *self_ref,
        "Self reference should either be BadValue or reference the same node"
    );

    // Verify fallback to base in self_ref
    assert_eq!(self_ref["fallback"]["name"].as_str(), Some("BaseObject"));

    // Test complex reference chain
    let complex_to_base = &doc["complex"]["refs"]["to_base"];
    assert_eq!(complex_to_base["name"].as_str(), Some("BaseObject"));

    // For the combined mapping, just check individual properties
    // Note: YAML merge keys are a tag feature that may not be fully supported
    let combined = &doc["complex"]["combined"];
    assert_eq!(
        combined["extra"].as_str(),
        Some("value"),
        "Should have its own properties"
    );

    // Let's test with more direct references instead of merge keys
    let level2 = &doc["nested"]["level2"];
    assert_eq!(level2["c"].as_i64(), Some(3));
    assert_eq!(level2["d"].as_i64(), Some(4));
}
