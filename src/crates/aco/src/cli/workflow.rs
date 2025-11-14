//! Workflow-specific CLI command handling
//!
//! Provides enhanced workflow command handlers with definition file parsing,
//! graph validation, and execution streaming.

use crate::error::{AcoError, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};

/// Workflow definition structure
#[derive(Debug, Clone)]
pub struct WorkflowDefinition {
    /// Workflow nodes
    pub nodes: Vec<WorkflowNode>,
    /// Edges connecting nodes
    pub edges: Vec<WorkflowEdge>,
}

/// Workflow node definition
#[derive(Debug, Clone)]
pub struct WorkflowNode {
    /// Node ID (unique identifier)
    pub id: String,
    /// Node type (task, conditional, etc)
    pub node_type: String,
    /// Node configuration
    pub config: Value,
}

/// Workflow edge definition
#[derive(Debug, Clone)]
pub struct WorkflowEdge {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Optional condition for conditional edges
    pub condition: Option<String>,
}

/// Load workflow definition from file
pub async fn load_workflow_definition(path: &Path) -> Result<WorkflowDefinition> {
    info!("Loading workflow definition from: {:?}", path);

    if !path.exists() {
        return Err(AcoError::Config(format!(
            "Workflow definition file not found: {:?}",
            path
        )));
    }

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| AcoError::Config(format!("Failed to read file: {}", e)))?;

    // Determine format based on file extension
    let definition = if path.extension().and_then(|s| s.to_str()) == Some("yaml")
        || path.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        parse_yaml_definition(&content)?
    } else {
        parse_json_definition(&content)?
    };

    validate_definition(&definition)?;

    Ok(definition)
}

/// Parse JSON workflow definition
fn parse_json_definition(content: &str) -> Result<WorkflowDefinition> {
    let json: Value = serde_json::from_str(content)
        .map_err(|e| AcoError::Config(format!("Invalid JSON: {}", e)))?;

    extract_definition_from_json(&json)
}

/// Parse YAML workflow definition
fn parse_yaml_definition(content: &str) -> Result<WorkflowDefinition> {
    // For simplicity, treat YAML similar to JSON
    // In production, would use serde_yaml crate
    debug!("Parsing YAML definition (treating as JSON)");

    let json: Value = serde_json::from_str(content)
        .map_err(|e| AcoError::Config(format!("Invalid YAML/JSON: {}", e)))?;

    extract_definition_from_json(&json)
}

/// Extract workflow definition from JSON value
fn extract_definition_from_json(json: &Value) -> Result<WorkflowDefinition> {
    let nodes_array = json
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AcoError::Config("Missing 'nodes' array".to_string()))?;

    let edges_array = json
        .get("edges")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AcoError::Config("Missing 'edges' array".to_string()))?;

    let mut nodes = Vec::new();
    for node_json in nodes_array {
        let id = node_json
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AcoError::Config("Node missing 'id'".to_string()))?;

        let node_type = node_json
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("task");

        let config = node_json
            .get("config")
            .cloned()
            .unwrap_or(json!({}));

        nodes.push(WorkflowNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            config,
        });
    }

    let mut edges = Vec::new();
    for edge_json in edges_array {
        let source = edge_json
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AcoError::Config("Edge missing 'source'".to_string()))?;

        let target = edge_json
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AcoError::Config("Edge missing 'target'".to_string()))?;

        let condition = edge_json.get("condition").and_then(|v| v.as_str()).map(|s| s.to_string());

        edges.push(WorkflowEdge {
            source: source.to_string(),
            target: target.to_string(),
            condition,
        });
    }

    Ok(WorkflowDefinition { nodes, edges })
}

/// Validate workflow definition structure
pub fn validate_definition(definition: &WorkflowDefinition) -> Result<()> {
    // Check for empty workflow
    if definition.nodes.is_empty() {
        return Err(AcoError::Config(
            "Workflow has no nodes".to_string(),
        ));
    }

    // Check for duplicate node IDs
    let mut node_ids = std::collections::HashSet::new();
    for node in &definition.nodes {
        if !node_ids.insert(&node.id) {
            return Err(AcoError::Config(format!(
                "Duplicate node ID: {}",
                node.id
            )));
        }
    }

    // Check for invalid edges
    for edge in &definition.edges {
        if !node_ids.contains(&edge.source) {
            return Err(AcoError::Config(format!(
                "Edge references non-existent source node: {}",
                edge.source
            )));
        }

        if !node_ids.contains(&edge.target) {
            return Err(AcoError::Config(format!(
                "Edge references non-existent target node: {}",
                edge.target
            )));
        }
    }

    // Check for cycles (simplified check for obvious cycles)
    if has_cycle(&definition.nodes, &definition.edges) {
        return Err(AcoError::Config(
            "Workflow contains cycles".to_string(),
        ));
    }

    info!("Workflow definition validated: {} nodes, {} edges",
        definition.nodes.len(),
        definition.edges.len());

    Ok(())
}

/// Check if workflow has cycles (simplified using DFS)
fn has_cycle(nodes: &[WorkflowNode], edges: &[WorkflowEdge]) -> bool {
    use std::collections::{HashMap, HashSet};

    // Build adjacency list
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for node in nodes {
        graph.entry(node.id.clone()).or_insert_with(Vec::new);
    }

    for edge in edges {
        graph
            .entry(edge.source.clone())
            .or_insert_with(Vec::new)
            .push(edge.target.clone());
    }

    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for node in nodes {
        if !visited.contains(&node.id) {
            if dfs_has_cycle(&node.id, &graph, &mut visited, &mut rec_stack) {
                return true;
            }
        }
    }

    false
}

/// DFS helper for cycle detection
fn dfs_has_cycle(
    node: &str,
    graph: &std::collections::HashMap<String, Vec<String>>,
    visited: &mut std::collections::HashSet<String>,
    rec_stack: &mut std::collections::HashSet<String>,
) -> bool {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                if dfs_has_cycle(neighbor, graph, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(neighbor) {
                return true;
            }
        }
    }

    rec_stack.remove(node);
    false
}

/// Convert workflow definition to JSON for API call
pub fn definition_to_json(definition: &WorkflowDefinition) -> Value {
    let nodes = definition
        .nodes
        .iter()
        .map(|n| {
            json!({
                "id": n.id,
                "type": n.node_type,
                "config": n.config
            })
        })
        .collect::<Vec<_>>();

    let edges = definition
        .edges
        .iter()
        .map(|e| {
            let mut edge = json!({
                "source": e.source,
                "target": e.target
            });

            if let Some(condition) = &e.condition {
                edge["condition"] = json!(condition);
            }

            edge
        })
        .collect::<Vec<_>>();

    json!({
        "nodes": nodes,
        "edges": edges
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_workflow() {
        let definition = WorkflowDefinition {
            nodes: vec![],
            edges: vec![],
        };

        let result = validate_definition(&definition);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_single_node() {
        let definition = WorkflowDefinition {
            nodes: vec![WorkflowNode {
                id: "node1".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            }],
            edges: vec![],
        };

        let result = validate_definition(&definition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_duplicate_node_ids() {
        let definition = WorkflowDefinition {
            nodes: vec![
                WorkflowNode {
                    id: "node1".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
                WorkflowNode {
                    id: "node1".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
            ],
            edges: vec![],
        };

        let result = validate_definition(&definition);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_source_node() {
        let definition = WorkflowDefinition {
            nodes: vec![WorkflowNode {
                id: "node1".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            }],
            edges: vec![WorkflowEdge {
                source: "invalid".to_string(),
                target: "node1".to_string(),
                condition: None,
            }],
        };

        let result = validate_definition(&definition);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_target_node() {
        let definition = WorkflowDefinition {
            nodes: vec![WorkflowNode {
                id: "node1".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            }],
            edges: vec![WorkflowEdge {
                source: "node1".to_string(),
                target: "invalid".to_string(),
                condition: None,
            }],
        };

        let result = validate_definition(&definition);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_linear_workflow() {
        let definition = WorkflowDefinition {
            nodes: vec![
                WorkflowNode {
                    id: "node1".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
                WorkflowNode {
                    id: "node2".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
                WorkflowNode {
                    id: "node3".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
            ],
            edges: vec![
                WorkflowEdge {
                    source: "node1".to_string(),
                    target: "node2".to_string(),
                    condition: None,
                },
                WorkflowEdge {
                    source: "node2".to_string(),
                    target: "node3".to_string(),
                    condition: None,
                },
            ],
        };

        let result = validate_definition(&definition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_parallel_workflow() {
        let definition = WorkflowDefinition {
            nodes: vec![
                WorkflowNode {
                    id: "start".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
                WorkflowNode {
                    id: "task1".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
                WorkflowNode {
                    id: "task2".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
                WorkflowNode {
                    id: "merge".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
            ],
            edges: vec![
                WorkflowEdge {
                    source: "start".to_string(),
                    target: "task1".to_string(),
                    condition: None,
                },
                WorkflowEdge {
                    source: "start".to_string(),
                    target: "task2".to_string(),
                    condition: None,
                },
                WorkflowEdge {
                    source: "task1".to_string(),
                    target: "merge".to_string(),
                    condition: None,
                },
                WorkflowEdge {
                    source: "task2".to_string(),
                    target: "merge".to_string(),
                    condition: None,
                },
            ],
        };

        let result = validate_definition(&definition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_definition_to_json() {
        let definition = WorkflowDefinition {
            nodes: vec![WorkflowNode {
                id: "node1".to_string(),
                node_type: "task".to_string(),
                config: json!({"param": "value"}),
            }],
            edges: vec![],
        };

        let json = definition_to_json(&definition);
        assert!(json.get("nodes").is_some());
        assert!(json.get("edges").is_some());
    }

    #[test]
    fn test_definition_to_json_with_condition() {
        let definition = WorkflowDefinition {
            nodes: vec![
                WorkflowNode {
                    id: "check".to_string(),
                    node_type: "conditional".to_string(),
                    config: json!({}),
                },
                WorkflowNode {
                    id: "yes".to_string(),
                    node_type: "task".to_string(),
                    config: json!({}),
                },
            ],
            edges: vec![WorkflowEdge {
                source: "check".to_string(),
                target: "yes".to_string(),
                condition: Some("true".to_string()),
            }],
        };

        let json = definition_to_json(&definition);
        let edges = json.get("edges").and_then(|e| e.as_array()).unwrap();
        assert_eq!(edges[0].get("condition"), Some(&json!("true")));
    }
}
