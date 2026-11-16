//! Persistent cache of resolved application names.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// File name inside the application cache directory.
const FILE_NAME: &str = "labels.json";

/// Application names persisted between sessions, keyed by package name.
#[derive(Debug, Default)]
pub struct LabelCache {
    path: Option<PathBuf>,
    labels: HashMap<String, String>,
}

impl LabelCache {
    /// Loads cached names from `directory`.
    ///
    /// A missing or unreadable file yields an empty cache. The cache is a
    /// convenience, so a bad file must never prevent the application from
    /// starting.
    pub fn load(directory: &Path) -> Self {
        let path = directory.join(FILE_NAME);
        let labels = fs::read_to_string(&path)
            .ok()
            .and_then(|source| serde_json::from_str(&source).ok())
            .unwrap_or_default();

        Self {
            path: Some(path),
            labels,
        }
    }

    /// Creates a cache that is never written to disk.
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Returns the cached name for `package`.
    pub fn get(&self, package: &str) -> Option<&str> {
        self.labels.get(package).map(String::as_str)
    }

    /// Returns every cached package and name.
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }

    /// Stores a name and writes the cache file.
    pub fn insert(&mut self, package: &str, label: &str) -> Result<(), LabelCacheError> {
        if self.get(package) == Some(label) {
            return Ok(());
        }

        self.labels.insert(package.to_owned(), label.to_owned());
        self.save()
    }

    fn save(&self) -> Result<(), LabelCacheError> {
        let Some(path) = &self.path else {
            return Ok(());
        };

        let source =
            serde_json::to_string_pretty(&self.labels).map_err(|error| LabelCacheError {
                message: error.to_string(),
            })?;

        fs::write(path, source).map_err(|error| LabelCacheError {
            message: error.to_string(),
        })
    }
}

/// Failure while writing the application-name cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelCacheError {
    /// Operating-system or serialization message.
    pub message: String,
}

impl std::fmt::Display for LabelCacheError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LabelCacheError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "android-utility-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("create test directory");
        directory
    }

    #[test]
    fn missing_file_starts_empty() {
        let directory = temporary_directory("missing");
        let cache = LabelCache::load(&directory);
        assert!(cache.labels().is_empty());

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn stored_names_survive_reload() {
        let directory = temporary_directory("roundtrip");

        let mut cache = LabelCache::load(&directory);
        cache.insert("com.example", "Example").expect("write cache");

        let reloaded = LabelCache::load(&directory);
        assert_eq!(reloaded.get("com.example"), Some("Example"));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn writes_json_object_keyed_by_package() {
        let directory = temporary_directory("json");

        let mut cache = LabelCache::load(&directory);
        cache.insert("com.example", "Example").expect("write cache");

        let source = fs::read_to_string(directory.join(FILE_NAME)).expect("read cache");
        let parsed: HashMap<String, String> = serde_json::from_str(&source).expect("valid json");
        assert_eq!(
            parsed.get("com.example").map(String::as_str),
            Some("Example")
        );

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn disabled_cache_keeps_names_in_memory() {
        let mut cache = LabelCache::disabled();
        cache.insert("com.example", "Example").expect("write");
        assert_eq!(cache.get("com.example"), Some("Example"));
    }
}
