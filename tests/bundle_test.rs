use okf::bundle::load_bundle;

#[test]
fn test_load_existing_bundle() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    assert_eq!(bundle.meta.id, "helix");
    assert_eq!(bundle.meta.version, Some("0.1.0".to_string()));
    assert!(bundle.tools.len() >= 2);
}

#[test]
fn test_load_bundle_with_multiple_tools() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    assert!(bundle.tools.contains_key("fs_read"));
    assert!(bundle.tools.contains_key("search"));
    assert_eq!(bundle.tools.len(), 2);
}

#[test]
fn test_load_specific_tool_fs_read() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    let tool = bundle.tools.get("fs_read").expect("fs_read tool not found");
    assert_eq!(tool.id, "fs_read");
    assert_eq!(tool.description, Some("Read a file from disk".to_string()));
}

#[test]
fn test_load_specific_tool_search() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    let tool = bundle.tools.get("search").expect("search tool not found");
    assert_eq!(tool.id, "search");
    assert_eq!(tool.description, Some("Search for files or content".to_string()));
}

#[test]
fn test_missing_bundle() {
    let result = load_bundle("bundles", "does_not_exist");
    assert!(result.is_err());
}

#[test]
fn test_bundle_meta_fields() {
    let bundle = load_bundle("bundles", "helix").expect("Failed to load helix bundle");

    assert_eq!(bundle.meta.id, "helix");
    assert!(bundle.meta.description.is_some());
    assert!(bundle.meta.version.is_some());
}
