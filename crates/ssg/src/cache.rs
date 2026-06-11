use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildCache {
    pub version: String,
    #[serde(default)]
    pub environment_hash: String,
    pub entries: HashMap<String, CacheEntry>,
    #[serde(skip)]
    force_rebuild_all: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_hash: String,
    pub output_path: String,
    pub built_at: String,
}

impl BuildCache {
    pub fn load(environment_hash: &str) -> Self {
        let cache_path = Path::new(".build-cache/cache.json");

        if !cache_path.exists() {
            return Self::new(environment_hash);
        }

        let parsed: Option<Self> = fs::read_to_string(cache_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok());

        match parsed {
            Some(cache)
                if cache.version == env!("CARGO_PKG_VERSION")
                    && cache.environment_hash == environment_hash =>
            {
                cache
            }
            Some(old) => {
                println!("♻️  Cache invalidated (environment changed) - full rebuild");
                // Keep the old entries: their output paths are still needed to
                // clean up posts deleted since the last build.
                Self {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    environment_hash: environment_hash.to_string(),
                    entries: old.entries,
                    force_rebuild_all: true,
                }
            }
            None => {
                eprintln!("⚠️  Cache file unreadable - full rebuild");
                Self::new(environment_hash)
            }
        }
    }

    pub fn new(environment_hash: &str) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment_hash: environment_hash.to_string(),
            entries: HashMap::new(),
            force_rebuild_all: false,
        }
    }

    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(".build-cache")?;
        let json = serde_json::to_string_pretty(self)?;
        write_atomic(Path::new(".build-cache/cache.json"), &json)?;
        Ok(())
    }

    pub fn needs_rebuild(&self, path: &Path, current_hash: &str) -> bool {
        if self.force_rebuild_all {
            return true;
        }

        match self.entries.get(&normalize_path(path)) {
            None => true,
            Some(entry) => entry.file_hash != current_hash,
        }
    }

    pub fn update_entry(&mut self, path: &Path, hash: String, output: String) {
        self.entries.insert(
            normalize_path(path),
            CacheEntry {
                file_hash: hash,
                output_path: output,
                built_at: chrono::Utc::now().to_rfc3339(),
            },
        );
    }

    /// Removes entries whose source path is no longer in `existing_sources`
    /// and returns their output paths so the caller can delete stale files.
    pub fn prune_deleted(&mut self, existing_sources: &HashSet<String>) -> Vec<String> {
        let orphaned: Vec<String> = self
            .entries
            .keys()
            .filter(|key| !existing_sources.contains(*key))
            .cloned()
            .collect();

        orphaned
            .iter()
            .filter_map(|key| self.entries.remove(key))
            .map(|entry| entry.output_path)
            .collect()
    }
}

pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, contents)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

pub fn hash_file(path: &Path) -> Result<String> {
    let content = fs::read(path)?;
    let hash = blake3::hash(&content);
    Ok(hash.to_hex().to_string())
}

pub fn hash_directory(dir: &Path) -> Result<String> {
    use walkdir::WalkDir;

    let mut hasher = blake3::Hasher::new();
    let mut files: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    files.sort_by_key(|e| e.path().to_path_buf());

    for entry in files {
        let path = entry.path();
        if let Ok(content) = fs::read(path) {
            hasher.update(normalize_path(path).as_bytes());
            hasher.update(&content);
        }
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Combined hash of every input (besides the post files themselves) that
/// affects rendered output. A mismatch invalidates the whole cache.
pub fn compute_environment_hash(content_dir: &Path) -> Result<String> {
    use walkdir::WalkDir;

    let mut hasher = blake3::Hasher::new();

    // A new binary may render differently (shortcodes, templates logic),
    // so hash the executable itself rather than trusting the version number.
    match std::env::current_exe().and_then(fs::read) {
        Ok(binary) => {
            hasher.update(&binary);
        }
        Err(_) => {
            hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
        }
    }

    if Path::new("templates").exists() {
        hasher.update(hash_directory(Path::new("templates"))?.as_bytes());
    }

    for file in ["config.yaml", "manifest.json"] {
        let path = Path::new(file);
        if path.exists() {
            hasher.update(hash_file(path)?.as_bytes());
        }
    }

    let mut category_files: Vec<_> = WalkDir::new(content_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == ".category.yaml")
        .map(|e| e.path().to_path_buf())
        .collect();
    category_files.sort();

    for path in category_files {
        hasher.update(normalize_path(&path).as_bytes());
        hasher.update(hash_file(&path)?.as_bytes());
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "Hello, world!").unwrap();

        let hash1 = hash_file(file.path()).unwrap();
        let hash2 = hash_file(file.path()).unwrap();

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_cache_needs_rebuild() {
        let cache = BuildCache::new("env_hash");
        let path = Path::new("test.md");

        assert!(cache.needs_rebuild(path, "abc123"));
    }

    #[test]
    fn test_cache_update_entry() {
        let mut cache = BuildCache::new("env_hash");
        let path = Path::new("test.md");

        cache.update_entry(
            path,
            "abc123".to_string(),
            "dist/test/index.html".to_string(),
        );

        assert!(!cache.needs_rebuild(path, "abc123"));
        assert!(cache.needs_rebuild(path, "different_hash"));
    }

    #[test]
    fn test_prune_deleted_returns_orphaned_outputs() {
        let mut cache = BuildCache::new("env_hash");
        cache.update_entry(
            Path::new("content/posts/dev/kept.md"),
            "hash1".to_string(),
            "dist/dev/kept/index.html".to_string(),
        );
        cache.update_entry(
            Path::new("content/posts/dev/deleted.md"),
            "hash2".to_string(),
            "dist/dev/deleted/index.html".to_string(),
        );

        let existing: HashSet<String> =
            [normalize_path(Path::new("content/posts/dev/kept.md"))].into();
        let orphaned = cache.prune_deleted(&existing);

        assert_eq!(orphaned, vec!["dist/dev/deleted/index.html".to_string()]);
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.needs_rebuild(Path::new("content/posts/dev/deleted.md"), "hash2"));
        assert!(!cache.needs_rebuild(Path::new("content/posts/dev/kept.md"), "hash1"));
    }

    #[test]
    fn test_normalize_path_uses_forward_slashes() {
        assert_eq!(
            normalize_path(Path::new("content/posts/dev/test.md")),
            "content/posts/dev/test.md"
        );
    }
}
