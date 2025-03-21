use yaml_rust2::yaml::{self, loader};

/// Verify that the default loader implementation is selected based on the feature flag
#[test]
#[cfg(not(feature = "position_tracked_loader"))]
fn test_default_loader() {
    // When position_tracked_loader is not enabled,
    // the DefaultLoader should be YamlLoader
    let yaml = "key: value";

    // Tests that DefaultLoader is YamlLoader by checking that the types match
    use yaml_rust2::yaml::{PositionTrackedLoader, YamlLoader};
    let yaml_loader_result = YamlLoader::load_from_str(yaml).unwrap();
    let default_loader_result = loader::load_from_str(yaml).unwrap();

    // Function to help verify the types are equivalent (not needed at runtime)
    fn assert_same_type<T>(_: &T, _: &T) {}

    // This would not compile if DefaultLoader is not YamlLoader
    // We don't call this function, but the type checker will verify it compiles
    fn verify_type() {
        let a = YamlLoader::load_from_str("").unwrap_or_default();
        let b = loader::DefaultLoader::load_from_str("").unwrap_or_default();
        assert_same_type(&a, &b);
    }

    // Verify the results are the same
    assert_eq!(yaml_loader_result.len(), default_loader_result.len());
    assert_eq!(
        yaml_loader_result[0]["key"].as_str(),
        default_loader_result[0]["key"].as_str()
    );
}

/// Verify that the default loader implementation is selected based on the feature flag
#[test]
#[cfg(feature = "position_tracked_loader")]
fn test_position_tracked_loader() {
    // When position_tracked_loader is enabled,
    // the DefaultLoader should be PositionTrackedLoader
    let yaml = "key: value";

    // Tests that DefaultLoader is PositionTrackedLoader by checking that the types match
    use yaml_rust2::yaml::{PositionTrackedLoader, YamlLoader};
    let position_tracked_loader_result = PositionTrackedLoader::load_from_str(yaml).unwrap();
    let default_loader_result = loader::load_from_str(yaml).unwrap();

    // Function to help verify the types are equivalent (not needed at runtime)
    fn assert_same_type<T>(_: &T, _: &T) {}

    // This would not compile if DefaultLoader is not PositionTrackedLoader
    // We don't call this function, but the type checker will verify it compiles
    fn verify_type() {
        let a = PositionTrackedLoader::load_from_str("").unwrap_or_default();
        let b = loader::DefaultLoader::load_from_str("").unwrap_or_default();
        assert_same_type(&a, &b);
    }

    // Verify the results are the same
    assert_eq!(
        position_tracked_loader_result.len(),
        default_loader_result.len()
    );
    assert_eq!(
        position_tracked_loader_result[0]["key"].as_str(),
        default_loader_result[0]["key"].as_str()
    );
}

/// This test always runs regardless of the feature flag
#[test]
fn test_loader_functions() {
    // Verify that the re-exported loader functions work correctly
    let yaml = "foo: bar";

    // Use the re-exported function from the yaml module
    let result_from_yaml = yaml::load_from_str(yaml).unwrap();

    // Use the function from the loader module
    let result_from_loader = loader::load_from_str(yaml).unwrap();

    // Results should be equivalent
    assert_eq!(result_from_yaml.len(), result_from_loader.len());
    assert_eq!(
        result_from_yaml[0]["foo"].as_str(),
        result_from_loader[0]["foo"].as_str()
    );

    // And they should contain the expected data
    assert_eq!(result_from_yaml[0]["foo"].as_str().unwrap(), "bar");
}
