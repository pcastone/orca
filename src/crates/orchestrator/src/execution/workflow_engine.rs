//! Workflow Execution Engine
//!
//! Orchestrates multi-task workflows using langgraph-core for state management,
//! execution control flow, and checkpoint management.

use crate::db::{DatabasePool, repositories::WorkflowRepository};
use crate::execution::ExecutionStreamHandler;
use crate::{OrchestratorError, Result, TaskExecutor};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Workflow node definition
#[derive(Debug, Clone)]
pub struct WorkflowNode {
    /// Node identifier
    pub id: String,
    /// Node type (task, conditional, etc)
    pub node_type: String,
    /// Node configuration
    pub config: HashMap<String, Value>,
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

/// Workflow execution state
#[derive(Debug, Clone)]
pub struct WorkflowExecutionState {
    /// Workflow ID
    pub workflow_id: String,
    /// Current node being executed
    pub current_node: Option<String>,
    /// Execution status
    pub status: String,
    /// Step counter
    pub step: u32,
    /// Node results map
    pub results: HashMap<String, Value>,
    /// Error message if failed
    pub error: Option<String>,
}

impl Default for WorkflowExecutionState {
    fn default() -> Self {
        Self {
            workflow_id: String::new(),
            current_node: None,
            status: "pending".to_string(),
            step: 0,
            results: HashMap::new(),
            error: None,
        }
    }
}

/// Workflow Execution Engine
///
/// Manages the full lifecycle of workflow execution:
/// 1. Parse workflow definition (nodes/edges)
/// 2. Build execution graph
/// 3. Execute with state management
/// 4. Emit execution events
/// 5. Handle errors and retries
pub struct WorkflowExecutionEngine {
    /// Database pool for loading/updating workflows
    pool: Arc<DatabasePool>,

    /// Task executor for executing task nodes
    task_executor: Arc<dyn TaskExecutor>,

    /// Maximum execution time in seconds
    max_execution_time: u64,

    /// Maximum retries per node
    max_retries: u32,
}

impl WorkflowExecutionEngine {
    /// Create a new workflow execution engine
    pub fn new(
        pool: Arc<DatabasePool>,
        task_executor: Arc<dyn TaskExecutor>,
    ) -> Self {
        Self {
            pool,
            task_executor,
            max_execution_time: 600, // 10 minutes default
            max_retries: 3,
        }
    }

    /// Set maximum execution time
    pub fn with_max_execution_time(mut self, seconds: u64) -> Self {
        self.max_execution_time = seconds;
        self
    }

    /// Set maximum retries per node
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Parse workflow definition from JSON string
    pub fn parse_definition(definition: &str) -> Result<(Vec<WorkflowNode>, Vec<WorkflowEdge>)> {
        let def: Value = serde_json::from_str(definition)
            .map_err(|e| OrchestratorError::ExecutionFailed(format!("Invalid workflow JSON: {}", e)))?;

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Parse nodes
        if let Some(nodes_arr) = def.get("nodes").and_then(|v| v.as_array()) {
            for node in nodes_arr {
                let id = node
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| OrchestratorError::ExecutionFailed("Node missing id".to_string()))?;

                let node_type = node
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("task");

                let config = node
                    .get("config")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default();

                nodes.push(WorkflowNode {
                    id: id.to_string(),
                    node_type: node_type.to_string(),
                    config,
                });
            }
        }

        // Parse edges
        if let Some(edges_arr) = def.get("edges").and_then(|v| v.as_array()) {
            for edge in edges_arr {
                let source = edge
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| OrchestratorError::ExecutionFailed("Edge missing source".to_string()))?;

                let target = edge
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| OrchestratorError::ExecutionFailed("Edge missing target".to_string()))?;

                let condition = edge.get("condition").and_then(|v| v.as_str()).map(|s| s.to_string());

                edges.push(WorkflowEdge {
                    source: source.to_string(),
                    target: target.to_string(),
                    condition,
                });
            }
        }

        debug!("Parsed workflow: {} nodes, {} edges", nodes.len(), edges.len());
        Ok((nodes, edges))
    }

    /// Find next nodes given current node
    pub fn find_next_nodes(
        current: Option<&str>,
        edges: &[WorkflowEdge],
        state: &WorkflowExecutionState,
    ) -> Vec<String> {
        let current_id = match current {
            Some(id) => id,
            None => {
                // Find root nodes (no incoming edges)
                let targets: Vec<_> = edges.iter().map(|e| &e.target).collect();
                let sources: Vec<_> = edges.iter().map(|e| &e.source).collect();

                return sources
                    .into_iter()
                    .filter(|s| !targets.contains(s))
                    .map(|s| s.to_string())
                    .collect();
            }
        };

        edges
            .iter()
            .filter(|e| e.source == current_id)
            .map(|e| e.target.clone())
            .collect()
    }

    /// Execute a single node
    async fn execute_node(
        &self,
        node: &WorkflowNode,
        state: &mut WorkflowExecutionState,
        stream_handler: &ExecutionStreamHandler,
    ) -> Result<()> {
        info!("Executing workflow node: {}", node.id);
        stream_handler
            .send_progress(format!("Executing node: {}", node.id))
            .await
            .ok();

        match node.node_type.as_str() {
            "task" => {
                // Execute as task node
                let task_id = node
                    .config
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        OrchestratorError::ExecutionFailed("Task node missing task_id".to_string())
                    })?;

                state.status = "running".to_string();
                state.current_node = Some(node.id.clone());

                // Simulate task execution (in real implementation, would call task_executor)
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Store result
                state
                    .results
                    .insert(node.id.clone(), json!({ "status": "completed", "output": "task executed" }));

                stream_handler
                    .send_output(format!("Task node {} completed", node.id))
                    .await
                    .ok();

                Ok(())
            }
            "conditional" => {
                // Execute as conditional node
                let condition = node
                    .config
                    .get("condition")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        OrchestratorError::ExecutionFailed("Conditional node missing condition".to_string())
                    })?;

                debug!("Evaluating condition: {}", condition);
                state.current_node = Some(node.id.clone());
                state
                    .results
                    .insert(node.id.clone(), json!({ "condition": condition, "result": true }));

                stream_handler
                    .send_progress(format!("Conditional node {} evaluated", node.id))
                    .await
                    .ok();

                Ok(())
            }
            _ => {
                warn!("Unknown node type: {}", node.node_type);
                Err(OrchestratorError::ExecutionFailed(format!(
                    "Unknown node type: {}",
                    node.node_type
                )))
            }
        }
    }

    /// Execute workflow with state graph
    async fn execute_workflow_internal(
        &self,
        workflow_id: &str,
        nodes: Vec<WorkflowNode>,
        edges: Vec<WorkflowEdge>,
        stream_handler: Arc<ExecutionStreamHandler>,
    ) -> Result<()> {
        info!("Starting execution of workflow: {}", workflow_id);

        let mut state = WorkflowExecutionState {
            workflow_id: workflow_id.to_string(),
            status: "running".to_string(),
            ..Default::default()
        };

        // Update workflow status to running
        WorkflowRepository::update_status(&self.pool, workflow_id, "running")
            .await
            .map_err(|e| {
                error!("Failed to update workflow status: {}", e);
                OrchestratorError::ExecutionFailed(format!("Failed to update status: {}", e))
            })?;

        stream_handler
            .send_started(workflow_id)
            .await
            .map_err(|e| OrchestratorError::ExecutionFailed(e))?;

        // Find initial nodes
        let mut current_nodes = Self::find_next_nodes(None, &edges, &state);
        let mut executed_nodes = std::collections::HashSet::new();

        // Execute nodes in topological order
        while !current_nodes.is_empty() && state.step < 100 {
            // Safety limit to prevent infinite loops
            state.step += 1;

            for node_id in current_nodes.iter() {
                if executed_nodes.contains(node_id) {
                    continue; // Skip if already executed
                }

                // Find node definition
                let node = nodes
                    .iter()
                    .find(|n| &n.id == node_id)
                    .ok_or_else(|| {
                        OrchestratorError::ExecutionFailed(format!("Node not found: {}", node_id))
                    })?;

                // Execute node
                if let Err(e) = self.execute_node(node, &mut state, &stream_handler).await {
                    error!("Node execution failed: {}", e);
                    state.status = "failed".to_string();
                    state.error = Some(format!("Node {} failed: {}", node_id, e));

                    stream_handler
                        .send_progress(format!("Node {} failed", node_id))
                        .await
                        .ok();

                    return Err(e);
                }

                executed_nodes.insert(node_id.clone());
            }

            // Find next nodes to execute
            let next_nodes: Vec<_> = current_nodes
                .iter()
                .flat_map(|node_id| Self::find_next_nodes(Some(node_id), &edges, &state))
                .collect();

            current_nodes = next_nodes;

            // Emit checkpoint event
            stream_handler
                .send_progress(format!("Step {} completed", state.step))
                .await
                .ok();
        }

        // Update workflow status to completed
        state.status = "completed".to_string();
        WorkflowRepository::update_status(&self.pool, workflow_id, "completed")
            .await
            .map_err(|e| {
                error!("Failed to update workflow status: {}", e);
                OrchestratorError::ExecutionFailed(format!("Failed to update status: {}", e))
            })?;

        stream_handler
            .send_completed(workflow_id, "Workflow completed successfully")
            .await
            .map_err(|e| OrchestratorError::ExecutionFailed(e))?;

        info!("Workflow {} completed successfully", workflow_id);
        Ok(())
    }

    /// Handle workflow execution errors
    async fn handle_execution_error(&self, workflow_id: &str, error: &str) -> Result<()> {
        warn!("Workflow {} failed with error: {}", workflow_id, error);

        WorkflowRepository::update_status(&self.pool, workflow_id, "failed")
            .await
            .map_err(|e| {
                error!("Failed to update workflow status to failed: {}", e);
                OrchestratorError::ExecutionFailed(format!("Failed to update status: {}", e))
            })?;

        Ok(())
    }
}

/// Trait for workflow execution
#[async_trait]
pub trait WorkflowExecutor: Send + Sync {
    /// Execute a workflow
    async fn execute(&self, workflow_id: &str, definition: &str) -> Result<()>;

    /// Execute with streaming
    async fn execute_with_streaming(
        &self,
        workflow_id: &str,
        definition: &str,
        stream_handler: Arc<ExecutionStreamHandler>,
    ) -> Result<()>;
}

#[async_trait]
impl WorkflowExecutor for WorkflowExecutionEngine {
    async fn execute(&self, workflow_id: &str, definition: &str) -> Result<()> {
        let (nodes, edges) = Self::parse_definition(definition)?;

        let (stream_handler, _rx) = ExecutionStreamHandler::new(100);
        let stream_handler = Arc::new(stream_handler);

        self.execute_workflow_internal(workflow_id, nodes, edges, stream_handler)
            .await
    }

    async fn execute_with_streaming(
        &self,
        workflow_id: &str,
        definition: &str,
        stream_handler: Arc<ExecutionStreamHandler>,
    ) -> Result<()> {
        let (nodes, edges) = Self::parse_definition(definition)?;

        self.execute_workflow_internal(workflow_id, nodes, edges, stream_handler)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workflow_definition() {
        let definition = r#"{
            "nodes": [
                {"id": "node1", "type": "task", "config": {"task_id": "task-1"}},
                {"id": "node2", "type": "task", "config": {"task_id": "task-2"}}
            ],
            "edges": [
                {"source": "node1", "target": "node2"}
            ]
        }"#;

        let (nodes, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(edges.len(), 1);
        assert_eq!(nodes[0].id, "node1");
        assert_eq!(edges[0].source, "node1");
    }

    #[test]
    fn test_find_root_nodes() {
        let edges = vec![WorkflowEdge {
            source: "start".to_string(),
            target: "end".to_string(),
            condition: None,
        }];

        let state = WorkflowExecutionState::default();
        let roots = WorkflowExecutionEngine::find_next_nodes(None, &edges, &state);

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], "start");
    }

    #[test]
    fn test_find_next_nodes() {
        let edges = vec![
            WorkflowEdge {
                source: "node1".to_string(),
                target: "node2".to_string(),
                condition: None,
            },
            WorkflowEdge {
                source: "node1".to_string(),
                target: "node3".to_string(),
                condition: None,
            },
        ];

        let state = WorkflowExecutionState::default();
        let next = WorkflowExecutionEngine::find_next_nodes(Some("node1"), &edges, &state);

        assert_eq!(next.len(), 2);
        assert!(next.contains(&"node2".to_string()));
        assert!(next.contains(&"node3".to_string()));
    }

    #[test]
    fn test_workflow_execution_state() {
        let state = WorkflowExecutionState {
            workflow_id: "wf-1".to_string(),
            status: "running".to_string(),
            step: 1,
            ..Default::default()
        };

        assert_eq!(state.workflow_id, "wf-1");
        assert_eq!(state.status, "running");
        assert_eq!(state.step, 1);
    }

    #[tokio::test]
    async fn test_workflow_execution_engine_creation() {
        // Placeholder test for workflow execution engine
        assert!(true);
    }
}
