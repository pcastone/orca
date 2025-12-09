//! YAML file loading service
//!
//! Handles loading YAML files with checksum computation.

use crate::error::{OrcaError, Result};
use crate::models::YamlFileType;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tracing::debug;

/// Loaded YAML file with metadata
#[derive(Debug, Clone)]
pub struct LoadedYaml {
    /// Full file path
    pub file_path: String,
    /// Detected file type
    pub file_type: YamlFileType,
    /// SHA-256 content hash
    pub content_hash: String,
    /// Parsed content as JSON Value
    pub content: Value,
    /// File size in bytes
    pub file_size: i64,
}

/// YAML file loading service
pub struct YamlLoaderService;

impl YamlLoaderService {
    /// Create a new loader service
    pub fn new() -> Self {
        Self
    }

    /// Load a YAML file and return parsed content with hash
    pub fn load_file(&self, path: &Path) -> Result<LoadedYaml> {
        // Read file content
        let content = fs::read_to_string(path).map_err(|e| {
            OrcaError::Other(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let file_size = content.len() as i64;

        // Compute SHA-256 hash
        let content_hash = self.compute_hash(&content);

        // Detect file type
        let file_type = self.detect_file_type(path)?;

        // Parse YAML
        let parsed = self.parse_yaml(&content)?;

        debug!(
            path = %path.display(),
            hash = %content_hash,
            file_type = ?file_type,
            "Loaded YAML file"
        );

        Ok(LoadedYaml {
            file_path: path.to_string_lossy().to_string(),
            file_type,
            content_hash,
            content: parsed,
            file_size,
        })
    }

    /// Compute SHA-256 hash of content
    pub fn compute_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute hash directly from file path
    pub fn compute_file_hash(&self, path: &Path) -> Result<String> {
        let content = fs::read(path).map_err(|e| {
            OrcaError::Other(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Detect file type from path
    pub fn detect_file_type(&self, path: &Path) -> Result<YamlFileType> {
        let path_str = path.to_string_lossy().to_lowercase();

        // Check path components to determine type
        if path_str.contains("/workflows/") || path_str.contains("\\workflows\\") {
            Ok(YamlFileType::Workflow)
        } else if path_str.contains("/templates/patterns/")
            || path_str.contains("\\templates\\patterns\\")
        {
            Ok(YamlFileType::Pattern)
        } else if path_str.contains("/templates/prompts/")
            || path_str.contains("\\templates\\prompts\\")
        {
            Ok(YamlFileType::Prompt)
        } else if path_str.contains("/templates/tools/")
            || path_str.contains("\\templates\\tools\\")
        {
            Ok(YamlFileType::Tool)
        } else if path_str.contains("/templates/") || path_str.contains("\\templates\\") {
            Ok(YamlFileType::Template)
        } else {
            // Try to detect from content structure
            Err(OrcaError::Other(format!(
                "Cannot determine YAML file type for: {}",
                path.display()
            )))
        }
    }

    /// Parse YAML content to JSON Value
    fn parse_yaml(&self, content: &str) -> Result<Value> {
        serde_yaml::from_str(content)
            .map_err(|e| OrcaError::Other(format!("YAML parse error: {}", e)))
    }

    /// Check if a path is a valid YAML file (must exist)
    pub fn is_yaml_file(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        self.has_yaml_extension(path)
    }

    /// Check if a path has a YAML extension (doesn't check existence)
    pub fn has_yaml_extension(&self, path: &Path) -> bool {
        match path.extension() {
            Some(ext) => {
                let ext_str = ext.to_string_lossy().to_lowercase();
                ext_str == "yaml" || ext_str == "yml"
            }
            None => false,
        }
    }

    /// Extract the name/id from a YAML file for display
    pub fn extract_name(&self, content: &Value) -> Option<String> {
        content
            .get("name")
            .or_else(|| content.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

impl Default for YamlLoaderService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_compute_hash() {
        let loader = YamlLoaderService::new();
        let hash = loader.compute_hash("test content");

        // SHA-256 hash should be 64 hex characters
        assert_eq!(hash.len(), 64);

        // Same content should produce same hash
        let hash2 = loader.compute_hash("test content");
        assert_eq!(hash, hash2);

        // Different content should produce different hash
        let hash3 = loader.compute_hash("different content");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_detect_file_type() {
        let loader = YamlLoaderService::new();

        let workflow = PathBuf::from("/project/workflows/test.yaml");
        assert_eq!(
            loader.detect_file_type(&workflow).unwrap(),
            YamlFileType::Workflow
        );

        let pattern = PathBuf::from("/project/templates/patterns/react.yaml");
        assert_eq!(
            loader.detect_file_type(&pattern).unwrap(),
            YamlFileType::Pattern
        );

        let prompt = PathBuf::from("/project/templates/prompts/system.yaml");
        assert_eq!(
            loader.detect_file_type(&prompt).unwrap(),
            YamlFileType::Prompt
        );

        let tool = PathBuf::from("/project/templates/tools/bash.yaml");
        assert_eq!(
            loader.detect_file_type(&tool).unwrap(),
            YamlFileType::Tool
        );
    }

    #[test]
    fn test_is_yaml_extension() {
        // Test extension checking logic directly
        let loader = YamlLoaderService::new();

        // Test has_yaml_extension method
        assert!(loader.has_yaml_extension(Path::new("test.yaml")));
        assert!(loader.has_yaml_extension(Path::new("test.yml")));
        assert!(loader.has_yaml_extension(Path::new("test.YAML")));
        assert!(!loader.has_yaml_extension(Path::new("test.json")));
        assert!(!loader.has_yaml_extension(Path::new("test")));
    }
}
