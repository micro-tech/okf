use okf::bundle::load_bundle;

#[test]
fn test_load_tool_fs_read() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    let tool = bundle.tools.get("fs_read").expect("Tool fs_read not found");

    assert_eq!(tool.id, "fs_read");
    assert!(tool.schema.is_object());
    assert!(tool.schema.get("properties").is_some());
}

#[test]
fn test_load_tool_search() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    let tool = bundle.tools.get("search").expect("Tool search not found");

    assert_eq!(tool.id, "search");
    assert!(tool.schema.is_object());
    assert!(tool.schema.get("properties").is_some());
}

#[test]
fn test_missing_tool() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    assert!(bundle.tools.get("nope").is_none());
    assert!(bundle.tools.get("nonexistent").is_none());
}

#[test]
fn test_tool_has_required_schema_fields() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    for (id, tool) in &bundle.tools {
        assert!(!id.is_empty(), "Tool id should not be empty");
        assert!(tool.schema.is_object(), "Tool {} should have object schema", id);
    }
}
