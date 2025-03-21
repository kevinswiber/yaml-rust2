use yaml_rust2::yaml::{PositionTrackedLoader, Yaml, YamlLoader};

/// Test helper to compare Yaml nodes for equivalence
fn assert_yaml_eq(a: &Yaml, b: &Yaml) {
    match (a, b) {
        (Yaml::Real(a_val), Yaml::Real(b_val)) => assert_eq!(a_val, b_val),
        (Yaml::Integer(a_val), Yaml::Integer(b_val)) => assert_eq!(a_val, b_val),
        (Yaml::String(a_val), Yaml::String(b_val)) => assert_eq!(a_val, b_val),
        (Yaml::Boolean(a_val), Yaml::Boolean(b_val)) => assert_eq!(a_val, b_val),
        (Yaml::Array(a_val), Yaml::Array(b_val)) => {
            assert_eq!(a_val.len(), b_val.len(), "Arrays have different lengths");
            for (a_item, b_item) in a_val.iter().zip(b_val.iter()) {
                assert_yaml_eq(a_item, b_item);
            }
        }
        (Yaml::Hash(a_val), Yaml::Hash(b_val)) => {
            assert_eq!(a_val.len(), b_val.len(), "Hashes have different lengths");
            for (a_key, a_value) in a_val {
                let b_value = &b_val[a_key];
                assert_yaml_eq(a_value, b_value);
            }
        }
        (Yaml::Alias(a_val), Yaml::Alias(b_val)) => assert_eq!(a_val, b_val),
        (Yaml::Null, Yaml::Null) => (),
        (Yaml::BadValue, Yaml::BadValue) => (),
        _ => panic!("Yaml types don't match: {:?} vs {:?}", a, b),
    }
}

#[test]
fn test_basic_yaml() {
    let yaml = "
    # A simple YAML document
    scalar: value
    integer: 42
    boolean: true
    float: 3.14159
    null_value: ~
    list:
      - item1
      - item2
      - 3
    nested:
      key1: value1
      key2: value2
    ";

    let loader_result = YamlLoader::load_from_str(yaml).unwrap();
    let tracked_loader_result = PositionTrackedLoader::load_from_str(yaml).unwrap();

    assert_eq!(loader_result.len(), tracked_loader_result.len());
    for (_i, (a, b)) in loader_result
        .iter()
        .zip(tracked_loader_result.iter())
        .enumerate()
    {
        assert_yaml_eq(a, b);
    }
}

#[test]
fn test_anchors_and_aliases() {
    let yaml = "
    # YAML with anchors and aliases
    anchor1: &a1 scalar_value
    anchor2: &a2
      - list_item1
      - list_item2
    anchor3: &a3
      key1: value1
      key2: value2
    
    reference1: *a1  # Reference to scalar
    reference2: *a2  # Reference to list
    reference3: *a3  # Reference to mapping
    
    nested:
      ref1: *a1
      ref2: *a2
      ref3: *a3
    ";

    let loader_result = YamlLoader::load_from_str(yaml).unwrap();
    let tracked_loader_result = PositionTrackedLoader::load_from_str(yaml).unwrap();

    assert_eq!(loader_result.len(), tracked_loader_result.len());
    for (_i, (a, b)) in loader_result
        .iter()
        .zip(tracked_loader_result.iter())
        .enumerate()
    {
        assert_yaml_eq(a, b);
    }
}

#[test]
fn test_complex_structure() {
    let yaml = r#"
    # Complex YAML structure with multiple types and anchors
    simple_types:
      string: "a string"
      integer: 42
      float: 3.14159
      boolean: true
      null: ~
      
    anchors:
      scalar: &scalar_anchor "reusable value"
      list: &list_anchor
        - 1
        - 2
        - 3
      map: &map_anchor
        a: 1
        b: 2
    
    references:
      scalar_ref: *scalar_anchor
      list_ref: *list_anchor
      map_ref: *map_anchor
    
    # Complex nested structures with references
    nested:
      level1: &level1
        level2: &level2
          data: "nested value"
      ref_level1: *level1
      ref_level2: *level2
    
    # Reference used in list
    complex_list:
      - *scalar_anchor
      - *list_anchor
      - *map_anchor
      
    # List with internal references
    self_refs:
      - &ref1 "first"
      - &ref2 "second"
      - *ref1
      - *ref2
    
    # Circular references
    circular:
      this: &this_ref
        child:
          parent: *this_ref
    "#;

    let loader_result = YamlLoader::load_from_str(yaml).unwrap();
    let tracked_loader_result = PositionTrackedLoader::load_from_str(yaml).unwrap();

    assert_eq!(loader_result.len(), tracked_loader_result.len());
    for (_i, (a, b)) in loader_result
        .iter()
        .zip(tracked_loader_result.iter())
        .enumerate()
    {
        assert_yaml_eq(a, b);
    }
}

#[test]
fn test_multi_document() {
    let yaml = "---\nname: doc1\nvalue: 1\n---\nname: doc2\nvalue: 2\n---\nname: doc3\nanchor: &a3 test\nreference: *a3\n";

    let loader_result = YamlLoader::load_from_str(yaml).unwrap();
    let tracked_loader_result = PositionTrackedLoader::load_from_str(yaml).unwrap();

    assert_eq!(loader_result.len(), tracked_loader_result.len());
    for (_i, (a, b)) in loader_result
        .iter()
        .zip(tracked_loader_result.iter())
        .enumerate()
    {
        assert_yaml_eq(a, b);
    }
}
