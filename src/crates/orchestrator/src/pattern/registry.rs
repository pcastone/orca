//! Pattern registry for storing and managing pattern configurations
//!
//! Provides thread-safe storage and retrieval of pattern configurations,
//! supporting both programmatic registration and loading from YAML files.

use crate::config::{load_yaml_config, PatternConfig};
use crate::{OrchestratorError, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Thread-safe registry for pattern configurations
#[derive(Debug, Clone)]
pub struct PatternRegistry {
    /// Internal storage for patterns
    patterns: Arc<RwLock<HashMap<String, PatternConfig>>>,
}

impl PatternRegistry {
    /// Create a new empty pattern registry
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a pattern configuration
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the pattern
    /// * `config` - Pattern configuration to register
    ///
    /// # Returns
    /// * `Ok(())` if registration succeeded
    /// * `Err` if pattern ID already exists
    pub fn register(&self, id: impl Into<String>, config: PatternConfig) -> Result<()> {
        let id = id.into();
        let mut patterns = self.patterns.write().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire write lock: {}", e))
        })?;

        if patterns.contains_key(&id) {
            return Err(OrchestratorError::General(format!(
                "Pattern with ID '{}' already registered",
                id
            )));
        }

        patterns.insert(id, config);
        Ok(())
    }

    /// Register or update a pattern configuration
    ///
    /// Unlike `register()`, this will overwrite an existing pattern with the same ID.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the pattern
    /// * `config` - Pattern configuration to register
    pub fn register_or_update(&self, id: impl Into<String>, config: PatternConfig) -> Result<()> {
        let id = id.into();
        let mut patterns = self.patterns.write().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire write lock: {}", e))
        })?;

        patterns.insert(id, config);
        Ok(())
    }

    /// Get a pattern configuration by ID
    ///
    /// # Arguments
    /// * `id` - Pattern identifier to look up
    ///
    /// # Returns
    /// * `Some(PatternConfig)` if pattern exists
    /// * `None` if pattern not found
    pub fn get(&self, id: &str) -> Result<Option<PatternConfig>> {
        let patterns = self.patterns.read().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(patterns.get(id).cloned())
    }

    /// Check if a pattern is registered
    ///
    /// # Arguments
    /// * `id` - Pattern identifier to check
    pub fn contains(&self, id: &str) -> Result<bool> {
        let patterns = self.patterns.read().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(patterns.contains_key(id))
    }

    /// Remove a pattern from the registry
    ///
    /// # Arguments
    /// * `id` - Pattern identifier to remove
    ///
    /// # Returns
    /// * `Some(PatternConfig)` if pattern was removed
    /// * `None` if pattern not found
    pub fn remove(&self, id: &str) -> Result<Option<PatternConfig>> {
        let mut patterns = self.patterns.write().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire write lock: {}", e))
        })?;

        Ok(patterns.remove(id))
    }

    /// Get all registered pattern IDs
    ///
    /// # Returns
    /// Vector of pattern identifiers
    pub fn list_ids(&self) -> Result<Vec<String>> {
        let patterns = self.patterns.read().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(patterns.keys().cloned().collect())
    }

    /// Get all registered patterns
    ///
    /// # Returns
    /// Vector of (id, config) tuples
    pub fn list_all(&self) -> Result<Vec<(String, PatternConfig)>> {
        let patterns = self.patterns.read().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(patterns
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    /// Get number of registered patterns
    pub fn count(&self) -> Result<usize> {
        let patterns = self.patterns.read().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(patterns.len())
    }

    /// Clear all registered patterns
    pub fn clear(&self) -> Result<()> {
        let mut patterns = self.patterns.write().map_err(|e| {
            OrchestratorError::General(format!("Failed to acquire write lock: {}", e))
        })?;

        patterns.clear();
        Ok(())
    }

    /// Load a pattern from a YAML file and register it
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the pattern
    /// * `path` - Path to the YAML configuration file
    ///
    /// # Returns
    /// * `Ok(())` if pattern was loaded and registered
    /// * `Err` if loading failed or pattern ID already exists
    pub fn load_from_file<P: AsRef<Path>>(
        &self,
        id: impl Into<String>,
        path: P,
    ) -> Result<()> {
        let config: PatternConfig = load_yaml_config(path)?;
        self.register(id, config)
    }

    /// Load a pattern from a YAML file and register or update it
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the pattern
    /// * `path` - Path to the YAML configuration file
    pub fn load_from_file_or_update<P: AsRef<Path>>(
        &self,
        id: impl Into<String>,
        path: P,
    ) -> Result<()> {
        let config: PatternConfig = load_yaml_config(path)?;
        self.register_or_update(id, config)
    }

    /// Load multiple patterns from a directory
    ///
    /// Scans the directory for .yaml and .yml files and attempts to load each as a pattern.
    /// The pattern ID is derived from the filename (without extension).
    ///
    /// # Arguments
    /// * `dir` - Directory path to scan
    /// * `recursive` - Whether to scan subdirectories recursively
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of patterns successfully loaded
    /// * `Err` - If directory cannot be read
    pub fn load_from_directory<P: AsRef<Path>>(
        &self,
        dir: P,
        recursive: bool,
    ) -> Result<usize> {
        let dir = dir.as_ref();

        if !dir.is_dir() {
            return Err(OrchestratorError::General(format!(
                "Path is not a directory: {}",
                dir.display()
            )));
        }

        let mut count = 0;

        let entries = std::fs::read_dir(dir).map_err(|e| {
            OrchestratorError::General(format!(
                "Failed to read directory {}: {}",
                dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                OrchestratorError::General(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            if path.is_dir() && recursive {
                count += self.load_from_directory(&path, recursive)?;
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        if let Some(stem) = path.file_stem() {
                            if let Some(id) = stem.to_str() {
                                // Try to load the pattern, but don't fail the entire operation
                                // if one file fails
                                match self.load_from_file_or_update(id, &path) {
                                    Ok(_) => {
                                        tracing::debug!("Loaded pattern '{}' from {}", id, path.display());
                                        count += 1;
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to load pattern from {}: {}",
                                            path.display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(count)
    }
}

impl Default for PatternRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BasePatternSettings, ReactConfig};
    use std::collections::HashMap;

    fn create_test_pattern(id: &str) -> PatternConfig {
        PatternConfig::React(ReactConfig {
            base: BasePatternSettings {
                id: id.to_string(),
                description: Some(format!("Test pattern {}", id)),
                system_prompt: None,
                max_iterations: 10,
                custom: HashMap::new(),
            },
            tools: vec![],
            temperature: Some(0.7),
        })
    }

    #[test]
    fn test_new_registry() {
        let registry = PatternRegistry::new();
        assert_eq!(registry.count().unwrap(), 0);
    }

    #[test]
    fn test_register_pattern() {
        let registry = PatternRegistry::new();
        let pattern = create_test_pattern("test1");

        assert!(registry.register("test1", pattern).is_ok());
        assert_eq!(registry.count().unwrap(), 1);
    }

    #[test]
    fn test_register_duplicate_fails() {
        let registry = PatternRegistry::new();
        let pattern1 = create_test_pattern("test1");
        let pattern2 = create_test_pattern("test1");

        registry.register("test1", pattern1).unwrap();
        let result = registry.register("test1", pattern2);

        assert!(result.is_err());
    }

    #[test]
    fn test_register_or_update() {
        let registry = PatternRegistry::new();
        let pattern1 = create_test_pattern("test1");
        let pattern2 = create_test_pattern("test1_updated");

        registry.register_or_update("test1", pattern1).unwrap();
        assert_eq!(registry.count().unwrap(), 1);

        registry.register_or_update("test1", pattern2).unwrap();
        assert_eq!(registry.count().unwrap(), 1);

        let retrieved = registry.get("test1").unwrap().unwrap();
        match retrieved {
            PatternConfig::React(config) => {
                assert_eq!(config.base.id, "test1_updated");
            }
            _ => panic!("Expected React pattern"),
        }
    }

    #[test]
    fn test_get_pattern() {
        let registry = PatternRegistry::new();
        let pattern = create_test_pattern("test1");

        registry.register("test1", pattern).unwrap();

        let retrieved = registry.get("test1").unwrap();
        assert!(retrieved.is_some());

        let not_found = registry.get("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_contains() {
        let registry = PatternRegistry::new();
        let pattern = create_test_pattern("test1");

        registry.register("test1", pattern).unwrap();

        assert!(registry.contains("test1").unwrap());
        assert!(!registry.contains("nonexistent").unwrap());
    }

    #[test]
    fn test_remove_pattern() {
        let registry = PatternRegistry::new();
        let pattern = create_test_pattern("test1");

        registry.register("test1", pattern).unwrap();
        assert_eq!(registry.count().unwrap(), 1);

        let removed = registry.remove("test1").unwrap();
        assert!(removed.is_some());
        assert_eq!(registry.count().unwrap(), 0);

        let not_found = registry.remove("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_list_ids() {
        let registry = PatternRegistry::new();

        registry.register("test1", create_test_pattern("test1")).unwrap();
        registry.register("test2", create_test_pattern("test2")).unwrap();
        registry.register("test3", create_test_pattern("test3")).unwrap();

        let ids = registry.list_ids().unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"test1".to_string()));
        assert!(ids.contains(&"test2".to_string()));
        assert!(ids.contains(&"test3".to_string()));
    }

    #[test]
    fn test_list_all() {
        let registry = PatternRegistry::new();

        registry.register("test1", create_test_pattern("test1")).unwrap();
        registry.register("test2", create_test_pattern("test2")).unwrap();

        let all = registry.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_clear() {
        let registry = PatternRegistry::new();

        registry.register("test1", create_test_pattern("test1")).unwrap();
        registry.register("test2", create_test_pattern("test2")).unwrap();

        assert_eq!(registry.count().unwrap(), 2);

        registry.clear().unwrap();
        assert_eq!(registry.count().unwrap(), 0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let registry = PatternRegistry::new();
        let registry_clone = registry.clone();

        let handle = thread::spawn(move || {
            registry_clone.register("thread_test", create_test_pattern("thread_test")).unwrap();
        });

        handle.join().unwrap();

        assert!(registry.contains("thread_test").unwrap());
    }

    #[test]
    fn test_load_from_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        let mut temp_file = NamedTempFile::new().unwrap();
        let yaml = r#"
type: react
id: "test_react"
description: "Test ReAct pattern"
max_iterations: 15
tools: []
temperature: 0.8
"#;
        temp_file.write_all(yaml.as_bytes()).unwrap();

        let registry = PatternRegistry::new();
        registry.load_from_file("test_react", temp_file.path()).unwrap();

        let pattern = registry.get("test_react").unwrap().unwrap();
        match pattern {
            PatternConfig::React(config) => {
                assert_eq!(config.base.id, "test_react");
                assert_eq!(config.base.max_iterations, 15);
                assert_eq!(config.temperature, Some(0.8));
            }
            _ => panic!("Expected React pattern"),
        }
    }

    #[test]
    fn test_load_from_directory() {
        use tempfile::TempDir;
        use std::fs;

        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test YAML files
        let yaml1 = r#"
type: react
id: "pattern1"
max_iterations: 10
tools: []
temperature: 0.7
"#;
        fs::write(dir_path.join("pattern1.yaml"), yaml1).unwrap();

        let yaml2 = r#"
type: react
id: "pattern2"
max_iterations: 20
tools: []
temperature: 0.9
"#;
        fs::write(dir_path.join("pattern2.yml"), yaml2).unwrap();

        // Create a non-YAML file (should be ignored)
        fs::write(dir_path.join("readme.txt"), "not a pattern").unwrap();

        let registry = PatternRegistry::new();
        let count = registry.load_from_directory(dir_path, false).unwrap();

        assert_eq!(count, 2);
        assert!(registry.contains("pattern1").unwrap());
        assert!(registry.contains("pattern2").unwrap());
    }
}
