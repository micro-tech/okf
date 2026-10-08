use okf::bundle::load_bundle;

#[test]
fn test_schema_has_required_fields() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    let tool = bundle.tools.get("fs_read").expect("Tool fs_read not found");

    let schema = &tool.schema;

    assert!(schema.get("type").is_some());
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_fs_read_schema_structure() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");
    let tool = bundle.tools.get("fs_read").unwrap();

    let schema = &tool.schema;
    let props = schema.get("properties").unwrap();

    assert!(props.get("path").is_some());
    // Check required array if present
    if let Some(required) = schema.get("required") {
        assert!(required.is_array());
    }
}

#[test]
fn test_search_schema_structure() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");
    let tool = bundle.tools.get("search").unwrap();

    let schema = &tool.schema;
    let props = schema.get("properties").unwrap();

    assert!(props.get("query").is_some());
}

#[test]
fn test_all_tools_have_valid_schemas() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    for (name, tool) in &bundle.tools {
        assert!(tool.schema.is_object(), "Schema for {} is not an object", name);
        assert!(tool.schema.get("type").is_some(), "Schema for {} missing 'type'", name);
    }
}
