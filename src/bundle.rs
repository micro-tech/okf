use crate::error::OkfError;
use crate::model::{Bundle, BundleFile, ToolFile};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::RwLock;
use std::time::SystemTime;
use tracing::{info, warn, debug, error};

/// Bundle ids come straight from the URL path, so they get a strict
/// whitelist: letters, digits, `-`, `_`. This kills `..`, `/`, `\` and
/// absolute paths in one shot (Reviewer item 1 — the real traversal bug).
pub fn validate_bundle_id(bundle_id: &str) -> Result<(), OkfError> {
    let valid = !bundle_id.is_empty()
        && bundle_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if valid {
        Ok(())
    } else {
        warn!(bundle_id = %bundle_id, "Rejected suspicious bundle id");
        Err(OkfError::InvalidBundleId(bundle_id.to_string()))
    }
}

/// Resolve `bundles_dir/<bundle_id>` to a canonical path and prove it stays
/// under the canonical bundles dir. The whitelist above already rejects
/// separators, but symlinks can still smuggle a path out — canonicalize
/// closes that hole too.
pub fn resolve_bundle_dir(bundles_dir: &str, bundle_id: &str) -> Result<PathBuf, OkfError> {
    validate_bundle_id(bundle_id)?;

    let canonical_base = Path::new(bundles_dir)
        .canonicalize()
        .map_err(OkfError::Io)?;
    let candidate = Path::new(bundles_dir).join(bundle_id);
    let canonical_bundle = candidate.canonicalize().map_err(|e| {
        // The common case: bundle doesn't exist. Don't leak the OS error;
        // the 500-trimming in error.rs handles the message, we just log it.
        debug!(bundles_dir = %bundles_dir, bundle_id = %bundle_id, error = %e, "Bundle dir not resolvable");
        if e.kind() == std::io::ErrorKind::NotFound {
            OkfError::BundleNotFound(bundle_id.to_string())
        } else {
            OkfError::Io(e)
        }
    })?;

    if !canonical_bundle.starts_with(&canonical_base) {
        error!(bundle_id = %bundle_id, "Bundle path escapes bundles directory");
        return Err(OkfError::InvalidBundle {
            bundle_id: bundle_id.to_string(),
            reason: "bundle path escapes bundles directory".to_string(),
        });
    }

    Ok(canonical_bundle)
}

/// Tool file names come from the bundle manifest (operator-controlled, not
/// URL input), but a sloppy manifest shouldn't be able to read
/// `/etc/passwd` either. Only plain relative file names allowed.
fn validate_tool_file_name(file: &str) -> Result<(), OkfError> {
    let ok = !file.is_empty()
        && Path::new(file)
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
    if ok {
        Ok(())
    } else {
        Err(OkfError::InvalidBundle {
            bundle_id: String::new(), // filled in by the caller
            reason: format!("tool file '{file}' must be a plain relative path"),
        })
    }
}

fn load_bundle_with_files(
    bundles_dir: &str,
    bundle_id: &str,
) -> Result<(Bundle, Vec<PathBuf>), OkfError> {
    debug!(bundles_dir = %bundles_dir, bundle_id = %bundle_id, "Loading bundle");

    let bundle_dir = resolve_bundle_dir(bundles_dir, bundle_id)?;
    let meta_path = bundle_dir.join("okf.yaml");

    // resolve_bundle_dir canonicalized, so this join can't escape either.
    let meta_str = fs::read_to_string(&meta_path).map_err(|e| {
        warn!(bundle_id = %bundle_id, error = %e, "Bundle metadata file not readable");
        if e.kind() == std::io::ErrorKind::NotFound {
            OkfError::BundleNotFound(bundle_id.to_string())
        } else {
            OkfError::Io(e)
        }
    })?;
    let mut source_files = vec![meta_path];
    // The bundle directory itself goes on the watch list: its mtime bumps
    // whenever entries directly inside it are added or removed. (Each
    // tool file's parent directory is watched separately below — the
    // tools live in tools/, a level down, and unlinking tools/t1.yaml
    // bumps tools/' mtime, not the bundle dir's. Without the parent-dir
    // watch, deleting a tool file leaves the cache serving the phantom
    // tool indefinitely — the old code failed loudly here, and the cache
    // must not fail silently.)
    source_files.push(bundle_dir.clone());

    let bundle_file: BundleFile = serde_yaml::from_str(&meta_str)?;

    // Validate bundle metadata
    if bundle_file.bundle.id.trim().is_empty() {
        return Err(OkfError::InvalidBundle {
            bundle_id: bundle_id.to_string(),
            reason: "bundle id is empty or missing".to_string(),
        });
    }

    // Detect duplicate tool IDs in the bundle definition
    let mut seen_tool_ids = std::collections::HashSet::new();
    for tool_ref in &bundle_file.tools {
        if !seen_tool_ids.insert(&tool_ref.id) {
            error!(bundle_id = %bundle_id, tool_id = %tool_ref.id, "Duplicate tool ID detected in bundle");
            return Err(OkfError::DuplicateToolId {
                bundle_id: bundle_id.to_string(),
                id: tool_ref.id.clone(),
            });
        }
    }

    let mut tools = HashMap::new();

    for tool_ref in &bundle_file.tools {
        if let Err(mut e) = validate_tool_file_name(&tool_ref.file) {
            // validate_tool_file_name can't know the bundle id; stamp it here.
            if let OkfError::InvalidBundle { bundle_id: id, .. } = &mut e {
                *id = bundle_id.to_string();
            }
            return Err(e);
        }
        let tool_path: PathBuf = bundle_dir.join(&tool_ref.file);

        if !tool_path.exists() {
            error!(
                bundle_id = %bundle_id,
                tool_id = %tool_ref.id,
                file = %tool_ref.file,
                "Tool file missing"
            );
            return Err(OkfError::MissingToolFile {
                bundle_id: bundle_id.to_string(),
                tool_id: tool_ref.id.clone(),
                file: tool_ref.file.clone(),
            });
        }

        let tool_str = fs::read_to_string(&tool_path)?;
        let tool_file: ToolFile = serde_yaml::from_str(&tool_str)?;
        let tool_def = tool_file.tool;

        // Validate tool definition
        if tool_def.id.trim().is_empty() {
            return Err(OkfError::InvalidToolDefinition {
                bundle_id: bundle_id.to_string(),
                tool_id: tool_ref.id.clone(),
                reason: "tool id is empty or missing".to_string(),
            });
        }

        // Optional: ensure the tool id matches the reference id in the bundle manifest
        if tool_def.id != tool_ref.id {
            return Err(OkfError::InvalidToolDefinition {
                bundle_id: bundle_id.to_string(),
                tool_id: tool_ref.id.clone(),
                reason: format!(
                    "tool id '{}' in file does not match declared id '{}' in bundle manifest",
                    tool_def.id, tool_ref.id
                ),
            });
        }

        // Schema is optional; when present it must be a JSON object.
        if let Some(schema) = &tool_def.schema {
            if !schema.is_object() {
                return Err(OkfError::InvalidToolDefinition {
                    bundle_id: bundle_id.to_string(),
                    tool_id: tool_def.id.clone(),
                    reason: "schema must be a JSON object when present".to_string(),
                });
            }
        }

        // Watch the tool file's parent directory as well as the file itself:
        // deleting (or adding) a tool file bumps the *parent dir's* mtime,
        // which is what invalidates the cache on removal. The bundle dir
        // alone doesn't see it — tools/ is a level down.
        if let Some(parent) = tool_path.parent() {
            source_files.push(parent.to_path_buf());
        }
        source_files.push(tool_path);
        tools.insert(tool_def.id.clone(), tool_def);
    }

    info!(
        bundle_id = %bundle_id,
        tool_count = tools.len(),
        "Successfully loaded bundle"
    );

    Ok((
        Bundle {
            meta: bundle_file.bundle,
            tools,
        },
        source_files,
    ))
}

pub fn load_bundle(bundles_dir: &str, bundle_id: &str) -> Result<Bundle, OkfError> {
    load_bundle_with_files(bundles_dir, bundle_id).map(|(bundle, _)| bundle)
}

struct CacheEntry {
    bundle: Bundle,
    /// Canonical paths of every file the bundle was loaded from.
    /// The hot path stats these instead of re-reading anything.
    source_files: Vec<PathBuf>,
    newest_mtime: SystemTime,
}

fn newest_mtime_of(files: &[PathBuf]) -> Option<SystemTime> {
    files
        .iter()
        .filter_map(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
        .max()
}

/// Any stat failure or any mtime newer than what we cached means "reload".
/// Coarse-mtime filesystems could theoretically miss a sub-tick edit;
/// the explicit `/v1/reload` endpoint (Phase 2) is the guaranteed path.
fn entry_is_stale(entry: &CacheEntry) -> bool {
    match newest_mtime_of(&entry.source_files) {
        Some(newest) => newest > entry.newest_mtime,
        None => true,
    }
}

/// mtime-based bundle cache (Reviewer item 6): the hot path is N+1 stat()
/// calls and zero YAML parsing, instead of re-reading every file per
/// request. Bundles are small and edits are rare — stat is the right price.
pub struct BundleCache {
    // A poisoned lock means a previous holder panicked mid-update; the
    // cache is suspect, so crashing loudly beats serving stale data.
    entries: RwLock<HashMap<String, CacheEntry>>,
}

impl BundleCache {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, bundles_dir: &str, bundle_id: &str) -> Result<Bundle, OkfError> {
        // Fast path under a read lock: clone the bundle out, drop the lock.
        let hit = {
            let entries = self.entries.read().expect("bundle cache lock poisoned");
            entries
                .get(bundle_id)
                .filter(|entry| !entry_is_stale(entry))
                .map(|entry| entry.bundle.clone())
        };
        if let Some(bundle) = hit {
            debug!(bundle_id = %bundle_id, "Bundle cache hit");
            return Ok(bundle);
        }

        // Slow path: load from disk with no lock held (two concurrent
        // misses may parse twice — harmless, last write wins), then store.
        debug!(bundle_id = %bundle_id, "Bundle cache miss");
        let (bundle, source_files) = load_bundle_with_files(bundles_dir, bundle_id)?;
        let newest_mtime = newest_mtime_of(&source_files).unwrap_or_else(SystemTime::now);
        self.entries.write().expect("bundle cache lock poisoned").insert(
            bundle_id.to_string(),
            CacheEntry {
                bundle: bundle.clone(),
                source_files,
                newest_mtime,
            },
        );
        Ok(bundle)
    }

    /// Explicit eviction — the `/v1/reload` endpoint (Phase 2) will call this.
    pub fn invalidate(&self, bundle_id: &str) {
        self.entries
            .write()
            .expect("bundle cache lock poisoned")
            .remove(bundle_id);
    }
}

impl Default for BundleCache {
    fn default() -> Self {
        Self::new()
    }
}
