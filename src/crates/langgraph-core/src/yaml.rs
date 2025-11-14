//! YAML-based graph definitions

use crate::error::Result;
use crate::graph::{ChannelType, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level YAML graph definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlGraphDef {
    /// Graph name
    pub name: String,

    /// Graph description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// State channel definitions
    #[serde(default)]
    pub channels: HashMap<String, YamlChannelDef>,

    /// Node definitions
    pub nodes: HashMap<String, YamlNodeDef>,

    /// Edge definitions
    pub edges: Vec<YamlEdgeDef>,

    /// Entry point node
    pub entry: String,

    /// Checkpoint configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<YamlCheckpointDef>,
}

/// Channel definition in YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlChannelDef {
    /// Channel type
    #[serde(rename = "type")]
    pub channel_type: ChannelType,

    /// Reducer function name (for BinaryOp channels)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reducer: Option<String>,
}

/// Node definition in YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlNodeDef {
    /// Handler function path (e.g., "my_module::my_function")
    pub handler: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Channels this node reads from
    #[serde(default)]
    pub reads: Vec<String>,

    /// Channels this node writes to
    #[serde(default)]
    pub writes: Vec<String>,
}

/// Edge definition in YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlEdgeDef {
    /// Direct edge
    Direct {
        from: NodeId,
        to: NodeId,
    },

    /// Conditional edge
    Conditional {
        from: NodeId,
        condition: String,
        branches: HashMap<String, NodeId>,
    },
}

/// Checkpoint configuration in YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlCheckpointDef {
    /// Enable checkpointing
    #[serde(default)]
    pub enabled: bool,

    /// Checkpoint backend type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,

    /// Backend-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

impl YamlGraphDef {
    /// Load graph definition from a YAML file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the YAML file
    ///
    /// # Returns
    ///
    /// Parsed graph definition
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    /// Parse graph definition from a YAML string
    ///
    /// # Arguments
    ///
    /// * `yaml` - YAML string
    ///
    /// # Returns
    ///
    /// Parsed graph definition
    pub fn from_str(yaml: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    /// Validate the graph definition
    ///
    /// # Returns
    ///
    /// Ok if valid, Err with description if invalid
    pub fn validate(&self) -> Result<()> {
        // Check entry point exists
        if !self.nodes.contains_key(&self.entry) && self.entry != "__start__" {
            return Err(crate::error::GraphError::Validation(format!(
                "Entry point '{}' does not exist",
                self.entry
            )));
        }

        // Check all edge targets exist
        for edge in &self.edges {
            match edge {
                YamlEdgeDef::Direct { from, to } => {
                    if !self.nodes.contains_key(from) && from != "__start__" {
                        return Err(crate::error::GraphError::Validation(format!(
                            "Edge source '{}' does not exist",
                            from
                        )));
                    }
                    if !self.nodes.contains_key(to) && to != "__end__" {
                        return Err(crate::error::GraphError::Validation(format!(
                            "Edge target '{}' does not exist",
                            to
                        )));
                    }
                }
                YamlEdgeDef::Conditional {
                    from, branches, ..
                } => {
                    if !self.nodes.contains_key(from) && from != "__start__" {
                        return Err(crate::error::GraphError::Validation(format!(
                            "Edge source '{}' does not exist",
                            from
                        )));
                    }
                    for to in branches.values() {
                        if !self.nodes.contains_key(to) && to != "__end__" {
                            return Err(crate::error::GraphError::Validation(format!(
                                "Branch target '{}' does not exist",
                                to
                            )));
                        }
                    }
                }
            }
        }

        // Check node names are unique
        let mut names = std::collections::HashSet::new();
        for name in self.nodes.keys() {
            if !names.insert(name) {
                return Err(crate::error::GraphError::Validation(format!(
                    "Duplicate node name: {}",
                    name
                )));
            }
        }

        Ok(())
    }

    /// Convert to a YAML string
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_yaml() {
        let yaml = r#"
name: simple_graph
description: A simple test graph
entry: process

nodes:
  process:
    handler: "process_handler"
    description: "Process the input"

edges:
  - from: "__start__"
    to: "process"
  - from: "process"
    to: "__end__"
"#;

        let graph = YamlGraphDef::from_str(yaml).unwrap();
        assert_eq!(graph.name, "simple_graph");
        assert_eq!(graph.entry, "process");
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 2);

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_parse_conditional_yaml() {
        let yaml = r#"
name: conditional_graph
entry: router

nodes:
  router:
    handler: "router_handler"
  path_a:
    handler: "path_a_handler"
  path_b:
    handler: "path_b_handler"

edges:
  - from: "__start__"
    to: "router"
  - from: "router"
    condition: "route_condition"
    branches:
      a: "path_a"
      b: "path_b"
  - from: "path_a"
    to: "__end__"
  - from: "path_b"
    to: "__end__"
"#;

        let graph = YamlGraphDef::from_str(yaml).unwrap();
        assert_eq!(graph.name, "conditional_graph");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 4);

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_yaml_validation_fails_missing_node() {
        let yaml = r#"
name: invalid_graph
entry: nonexistent

nodes:
  process:
    handler: "process_handler"

edges:
  - from: "__start__"
    to: "process"
"#;

        let graph = YamlGraphDef::from_str(yaml).unwrap();
        assert!(graph.validate().is_err());
    }

    #[test]
    fn test_yaml_with_channels() {
        let yaml = r#"
name: stateful_graph
entry: process

channels:
  messages:
    type: topic
  counter:
    type: binary_op
    reducer: "sum"

nodes:
  process:
    handler: "process_handler"
    reads:
      - messages
    writes:
      - counter

edges:
  - from: "__start__"
    to: "process"
  - from: "process"
    to: "__end__"
"#;

        let graph = YamlGraphDef::from_str(yaml).unwrap();
        assert_eq!(graph.channels.len(), 2);
        assert_eq!(graph.channels["messages"].channel_type, ChannelType::Topic);
        assert_eq!(graph.channels["counter"].channel_type, ChannelType::BinaryOp);
        assert_eq!(graph.channels["counter"].reducer, Some("sum".to_string()));
    }
}
