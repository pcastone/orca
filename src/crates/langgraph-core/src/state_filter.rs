//! State history filtering support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;
use langgraph_checkpoint::{CheckpointMetadata, checkpoint::CheckpointSource};

/// Filter criteria for state history queries
///
/// This struct provides fine-grained filtering options when querying
/// checkpoint history, allowing you to filter by source, step range,
/// and custom metadata fields.
///
/// # Example
///
/// ```rust
/// use langgraph_core::StateHistoryFilter;
/// use langgraph_checkpoint::CheckpointSource;
///
/// let filter = StateHistoryFilter::new()
///     .with_source(CheckpointSource::Update)
///     .with_min_step(5)
///     .with_max_step(10);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateHistoryFilter {
    /// Filter by checkpoint source (Input, Loop, Update, Fork)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<CheckpointSource>,

    /// Filter by minimum step number (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_step: Option<i32>,

    /// Filter by maximum step number (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_step: Option<i32>,

    /// Custom metadata filters (key-value pairs that must match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,

    /// Filter by node that created the checkpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
}

impl StateHistoryFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by checkpoint source
    pub fn with_source(mut self, source: CheckpointSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Filter by minimum step number
    pub fn with_min_step(mut self, min_step: i32) -> Self {
        self.min_step = Some(min_step);
        self
    }

    /// Filter by maximum step number
    pub fn with_max_step(mut self, max_step: i32) -> Self {
        self.max_step = Some(max_step);
        self
    }

    /// Add a custom metadata filter
    pub fn with_metadata_field(mut self, key: impl Into<String>, value: Value) -> Self {
        let metadata = self.metadata.get_or_insert_with(HashMap::new);
        metadata.insert(key.into(), value);
        self
    }

    /// Filter by node that created the checkpoint
    pub fn with_node(mut self, node: impl Into<String>) -> Self {
        self.node = Some(node.into());
        self
    }

    /// Check if a checkpoint metadata matches this filter
    pub fn matches(&self, metadata: &CheckpointMetadata) -> bool {
        // Check source filter
        if let Some(ref source_filter) = self.source {
            if metadata.source.as_ref() != Some(source_filter) {
                return false;
            }
        }

        // Check step range
        if let Some(step) = metadata.step {
            if let Some(min_step) = self.min_step {
                if step < min_step {
                    return false;
                }
            }
            if let Some(max_step) = self.max_step {
                if step > max_step {
                    return false;
                }
            }
        } else {
            // If metadata has no step but filter requires one, no match
            if self.min_step.is_some() || self.max_step.is_some() {
                return false;
            }
        }

        // Check custom metadata fields
        if let Some(ref filter_metadata) = self.metadata {
            for (key, expected_value) in filter_metadata {
                if let Some(actual_value) = metadata.extra.get(key) {
                    if actual_value != expected_value {
                        return false;
                    }
                } else {
                    // Required metadata field not present
                    return false;
                }
            }
        }

        // Check node filter
        if let Some(ref node_filter) = self.node {
            if let Some(Value::String(node)) = metadata.extra.get("node") {
                if node != node_filter {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Convert to a HashMap for compatibility with existing checkpoint API
    pub fn to_hashmap(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();

        if let Some(ref source) = self.source {
            map.insert("source".to_string(), serde_json::to_value(source).unwrap());
        }

        if let Some(min_step) = self.min_step {
            map.insert("min_step".to_string(), Value::Number(min_step.into()));
        }

        if let Some(max_step) = self.max_step {
            map.insert("max_step".to_string(), Value::Number(max_step.into()));
        }

        if let Some(ref metadata) = self.metadata {
            for (key, value) in metadata {
                map.insert(format!("metadata.{}", key), value.clone());
            }
        }

        if let Some(ref node) = self.node {
            map.insert("node".to_string(), Value::String(node.clone()));
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use langgraph_checkpoint::checkpoint::CheckpointSource;

    #[test]
    fn test_filter_creation() {
        let filter = StateHistoryFilter::new()
            .with_source(CheckpointSource::Update)
            .with_min_step(5)
            .with_max_step(10)
            .with_node("process");

        assert_eq!(filter.source, Some(CheckpointSource::Update));
        assert_eq!(filter.min_step, Some(5));
        assert_eq!(filter.max_step, Some(10));
        assert_eq!(filter.node, Some("process".to_string()));
    }

    #[test]
    fn test_filter_matching() {
        let filter = StateHistoryFilter::new()
            .with_source(CheckpointSource::Loop)
            .with_min_step(5);

        let mut metadata = CheckpointMetadata::default();
        metadata.source = Some(CheckpointSource::Loop);
        metadata.step = Some(6);

        assert!(filter.matches(&metadata));

        metadata.step = Some(3);
        assert!(!filter.matches(&metadata));

        metadata.source = Some(CheckpointSource::Input);
        metadata.step = Some(6);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_metadata_filtering() {
        let filter = StateHistoryFilter::new()
            .with_metadata_field("status", Value::String("approved".to_string()));

        let mut metadata = CheckpointMetadata::default();
        metadata.extra.insert("status".to_string(), Value::String("approved".to_string()));

        assert!(filter.matches(&metadata));

        metadata.extra.insert("status".to_string(), Value::String("rejected".to_string()));
        assert!(!filter.matches(&metadata));
    }
}