use okf::bundle::{load_bundle, validate_bundle_id};
use okf::error::OkfError;
use std::fs;
use std::path::PathBuf;

fn is_invalid_bundle_id(err: &OkfError) -> bool {
    matches!(err, OkfError::InvalidBundleId(_))
}

#[test]
fn test_validate_bundle_id_accepts_sane_ids() {
    // GIVEN ordinary bundle ids,
    // THEN validation passes:
    for id in ["helix", "grok-cli", "shared", "a1", "A-_9", "x"] {
        assert!(validate_bundle_id(id).is_ok(), "id '{id}' should be valid");
    }
}

#[test]
fn test_validate_bundle_id_rejects_traversal_and_junk() {
    // GIVEN hostile or malformed ids,
    // THEN every single one is rejected (never resolved to a path):
    for id in [
        "",
        ".",
        "..",
        "../x",
        "x/..",
        "a/b",
        "a\\b",
        "/etc",
        "C:\\win",
        "a b",
        "a:b",
        "a*b",
        "hélix",
    ] {
        let err = validate_bundle_id(id).expect_err(&format!("id '{id}' should be rejected"));
        assert!(is_invalid_bundle_id(&err), "id '{id}' gave wrong error: {err}");
    }
}

#[test]
fn test_load_bundle_rejects_traversal_ids() {
    // GIVEN traversal-style ids handed to the loader,
    // THEN the load fails with InvalidBundleId — the filesystem is never
    // consulted for these (no reads outside bundles/):
    for id in ["..", "../config", "helix/../shared", "..%2f.."] {
        let err = load_bundle("bundles", id).expect_err(&format!("id '{id}' should fail"));
        assert!(
            is_invalid_bundle_id(&err),
            "id '{id}' gave wrong error: {err}"
        );
    }
}

#[test]
fn test_load_bundle_rejects_symlink_escape() {
    // GIVEN a bundle dir that is a symlink pointing outside bundles/:
    let base = std::env::temp_dir().join(format!("okf-traversal-{}", std::process::id()));
    let bundles = base.join("bundles");
    let outside = base.join("outside");
    fs::create_dir_all(bundles.join("tools")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    // A real bundle outside the root, reachable only via the symlink.
    fs::write(
        outside.join("okf.yaml"),
        "bundle:\n  id: \"evil\"\n  version: \"0.1.0\"\ntools: []\n",
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, bundles.join("sneaky")).unwrap();

    // WHEN we load through the symlink,
    // THEN canonicalization catches it and the load fails:
    let err = load_bundle(bundles.to_str().unwrap(), "sneaky").expect_err("symlink escape should fail");
    assert!(
        !matches!(err, OkfError::BundleNotFound(_)),
        "expected escape rejection, got: {err}"
    );

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_load_bundle_rejects_tool_file_escape_in_manifest() {
    // GIVEN a bundle whose manifest points a tool file at ../../evil.yaml:
    let base = std::env::temp_dir().join(format!("okf-toolfile-{}", std::process::id()));
    let bundle_dir = base.join("bundles").join("tricky");
    fs::create_dir_all(bundle_dir.join("tools")).unwrap();
    fs::write(
        bundle_dir.join("okf.yaml"),
        "bundle:\n  id: \"tricky\"\n  version: \"0.1.0\"\ntools:\n  - id: \"t\"\n    file: \"../../evil.yaml\"\n",
    )
    .unwrap();

    // WHEN we load it,
    // THEN the manifest escape is rejected (no read outside the bundle dir):
    let err = load_bundle(
        base.join("bundles").to_str().unwrap(),
        "tricky",
    )
    .expect_err("tool file escape should fail");
    assert!(
        matches!(err, OkfError::InvalidBundle { .. }),
        "expected InvalidBundle, got: {err}"
    );

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_resolve_bundle_dir_stays_canonical() {
    // Sanity: a legit id resolves under the bundles dir.
    let dir = okf::bundle::resolve_bundle_dir("bundles", "helix").unwrap();
    let dir_str = dir.to_string_lossy();
    assert!(dir_str.ends_with("helix"), "unexpected resolution: {dir_str}");
    // And it must be absolute (canonicalized).
    assert!(PathBuf::from(dir_str.as_ref()).is_absolute());
}
