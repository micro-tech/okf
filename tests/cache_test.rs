use okf::bundle::BundleCache;
use std::fs;
use std::path::PathBuf;

const MANIFEST: &str = r#"bundle:
  id: "tbundle"
  description: "cache test bundle"
  version: "0.1.0"
tools:
  - id: "t1"
    file: "tools/t1.yaml"
"#;

fn tool_yaml(description: &str) -> String {
    format!(
        "tool:\n  id: \"t1\"\n  description: \"{description}\"\n  schema:\n    type: \"object\"\n"
    )
}

struct Fixture {
    base: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let base = std::env::temp_dir().join(format!("okf-cache-{}-{}", std::process::id(), name));
        let bundle_dir = base.join("bundles").join("tbundle").join("tools");
        fs::create_dir_all(&bundle_dir).unwrap();
        let fx = Self { base };
        fx.write_manifest();
        fx.write_tool("original");
        fx
    }

    fn bundles_dir(&self) -> String {
        self.base.join("bundles").to_str().unwrap().to_string()
    }

    fn write_manifest(&self) {
        fs::write(self.base.join("bundles").join("tbundle").join("okf.yaml"), MANIFEST).unwrap();
    }

    fn write_tool(&self, description: &str) {
        fs::write(self.tool_path(), tool_yaml(description)).unwrap();
    }

    fn tool_path(&self) -> PathBuf {
        self.base.join("bundles").join("tbundle").join("tools").join("t1.yaml")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn description_of(cache: &BundleCache, bundles_dir: &str) -> String {
    cache
        .get(bundles_dir, "tbundle")
        .expect("bundle should load")
        .tools
        .get("t1")
        .expect("t1 should exist")
        .description
        .clone()
        .unwrap()
}

#[test]
fn test_cache_serves_repeated_reads() {
    // GIVEN a warm cache:
    let fx = Fixture::new("repeat");
    let cache = BundleCache::new();
    assert_eq!(description_of(&cache, &fx.bundles_dir()), "original");

    // WHEN we read again with no changes,
    // THEN we get the same data (served from cache, no re-parse):
    assert_eq!(description_of(&cache, &fx.bundles_dir()), "original");
}

#[test]
fn test_cache_invalidates_on_file_change() {
    // GIVEN a warm cache:
    let fx = Fixture::new("mtime");
    let cache = BundleCache::new();
    assert_eq!(description_of(&cache, &fx.bundles_dir()), "original");

    // WHEN the tool file changes on disk (mtime advances),
    // THEN the next read picks up the new content:
    fx.write_tool("updated");
    assert_eq!(description_of(&cache, &fx.bundles_dir()), "updated");
}

#[test]
fn test_cache_invalidate_forces_reload() {
    // GIVEN a warm cache:
    let fx = Fixture::new("explicit");
    let cache = BundleCache::new();
    assert_eq!(description_of(&cache, &fx.bundles_dir()), "original");

    // WHEN we explicitly invalidate and the file changed,
    // THEN the next read reloads from disk:
    fx.write_tool("reloaded");
    cache.invalidate("tbundle");
    assert_eq!(description_of(&cache, &fx.bundles_dir()), "reloaded");
}

#[test]
fn test_cache_invalidates_when_tool_file_is_deleted() {
    // GIVEN a warm cache serving a bundle with one tool:
    let fx = Fixture::new("delete");
    let cache = BundleCache::new();
    assert_eq!(description_of(&cache, &fx.bundles_dir()), "original");

    // WHEN the tool file is deleted from disk (manifest untouched),
    // THEN the next read fails loudly instead of serving the phantom tool:
    // the tool file's parent directory mtime bumps on the unlink (unlink
    // only touches the immediate parent, not the bundle dir), which
    // invalidates the cache, and the reload hits the missing-file check in
    // load_bundle_with_files.
    fs::remove_file(fx.tool_path()).unwrap();
    assert!(cache.get(&fx.bundles_dir(), "tbundle").is_err());
}

#[test]
fn test_cache_missing_bundle_is_not_cached_as_hit() {
    // GIVEN a cache:
    let fx = Fixture::new("miss");
    let cache = BundleCache::new();

    // WHEN the bundle doesn't exist,
    // THEN every read errors (misses are never cached as hits):
    assert!(cache.get(&fx.bundles_dir(), "nope").is_err());
    assert!(cache.get(&fx.bundles_dir(), "nope").is_err());
}
