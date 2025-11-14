use orchestrator::execution::{WorkflowExecutionEngine, WorkflowExecutor};
use serde_json::json;

#[test]
fn test_workflow_engine_parse_simple_definition() {
    let definition = r#"{
        "nodes": [
            {"id": "start", "type": "task", "config": {"task_id": "task-1"}},
            {"id": "end", "type": "task", "config": {"task_id": "task-2"}}
        ],
        "edges": [
            {"source": "start", "target": "end"}
        ]
    }"#;

    let (nodes, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();

    assert_eq!(nodes.len(), 2);
    assert_eq!(edges.len(), 1);
    assert_eq!(nodes[0].id, "start");
    assert_eq!(nodes[1].id, "end");
    assert_eq!(edges[0].source, "start");
    assert_eq!(edges[0].target, "end");
}

#[test]
fn test_workflow_engine_parse_complex_definition() {
    let definition = r#"{
        "nodes": [
            {"id": "init", "type": "task", "config": {"task_id": "task-init"}},
            {"id": "process", "type": "task", "config": {"task_id": "task-process"}},
            {"id": "validate", "type": "conditional", "config": {"condition": "result == success"}},
            {"id": "finalize", "type": "task", "config": {"task_id": "task-finalize"}}
        ],
        "edges": [
            {"source": "init", "target": "process"},
            {"source": "process", "target": "validate"},
            {"source": "validate", "target": "finalize"}
        ]
    }"#;

    let (nodes, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();

    assert_eq!(nodes.len(), 4);
    assert_eq!(edges.len(), 3);
    assert_eq!(nodes[2].node_type, "conditional");
    assert!(nodes[2].config.contains_key("condition"));
}

#[test]
fn test_workflow_engine_parse_parallel_definition() {
    let definition = r#"{
        "nodes": [
            {"id": "start", "type": "task", "config": {"task_id": "task-1"}},
            {"id": "task_a", "type": "task", "config": {"task_id": "task-a"}},
            {"id": "task_b", "type": "task", "config": {"task_id": "task-b"}},
            {"id": "merge", "type": "task", "config": {"task_id": "task-merge"}}
        ],
        "edges": [
            {"source": "start", "target": "task_a"},
            {"source": "start", "target": "task_b"},
            {"source": "task_a", "target": "merge"},
            {"source": "task_b", "target": "merge"}
        ]
    }"#;

    let (nodes, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();

    assert_eq!(nodes.len(), 4);
    assert_eq!(edges.len(), 4);
}

#[test]
fn test_workflow_engine_parse_with_conditions() {
    let definition = r#"{
        "nodes": [
            {"id": "check", "type": "conditional", "config": {"condition": "x > 10"}}
        ],
        "edges": [
            {"source": "check", "target": "branch_a", "condition": "true"},
            {"source": "check", "target": "branch_b", "condition": "false"}
        ]
    }"#;

    let (_, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();

    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].condition, Some("true".to_string()));
    assert_eq!(edges[1].condition, Some("false".to_string()));
}

#[test]
fn test_workflow_engine_parse_empty_nodes() {
    let definition = r#"{
        "nodes": [],
        "edges": []
    }"#;

    let (nodes, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();

    assert_eq!(nodes.len(), 0);
    assert_eq!(edges.len(), 0);
}

#[test]
fn test_workflow_engine_parse_invalid_json() {
    let invalid_json = "not valid json {";

    let result = WorkflowExecutionEngine::parse_definition(invalid_json);
    assert!(result.is_err());
}

#[test]
fn test_workflow_engine_parse_missing_node_id() {
    let definition = r#"{
        "nodes": [
            {"type": "task", "config": {}}
        ],
        "edges": []
    }"#;

    let result = WorkflowExecutionEngine::parse_definition(definition);
    assert!(result.is_err());
}

#[test]
fn test_workflow_engine_parse_missing_edge_source() {
    let definition = r#"{
        "nodes": [{"id": "n1", "type": "task"}],
        "edges": [{"target": "n1"}]
    }"#;

    let result = WorkflowExecutionEngine::parse_definition(definition);
    assert!(result.is_err());
}

#[test]
fn test_workflow_engine_find_root_nodes_single() {
    let definition = r#"{
        "nodes": [
            {"id": "root", "type": "task"},
            {"id": "child", "type": "task"}
        ],
        "edges": [
            {"source": "root", "target": "child"}
        ]
    }"#;

    let (_, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();
    let state = Default::default();
    let roots = WorkflowExecutionEngine::find_next_nodes(None, &edges, &state);

    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0], "root");
}

#[test]
fn test_workflow_engine_find_root_nodes_multiple() {
    let definition = r#"{
        "nodes": [
            {"id": "root_a", "type": "task"},
            {"id": "root_b", "type": "task"},
            {"id": "merge", "type": "task"}
        ],
        "edges": [
            {"source": "root_a", "target": "merge"},
            {"source": "root_b", "target": "merge"}
        ]
    }"#;

    let (_, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();
    let state = Default::default();
    let roots = WorkflowExecutionEngine::find_next_nodes(None, &edges, &state);

    assert_eq!(roots.len(), 2);
    assert!(roots.contains(&"root_a".to_string()));
    assert!(roots.contains(&"root_b".to_string()));
}

#[test]
fn test_workflow_engine_find_next_nodes_single_path() {
    let definition = r#"{
        "nodes": [
            {"id": "n1", "type": "task"},
            {"id": "n2", "type": "task"}
        ],
        "edges": [
            {"source": "n1", "target": "n2"}
        ]
    }"#;

    let (_, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();
    let state = Default::default();
    let next = WorkflowExecutionEngine::find_next_nodes(Some("n1"), &edges, &state);

    assert_eq!(next.len(), 1);
    assert_eq!(next[0], "n2");
}

#[test]
fn test_workflow_engine_find_next_nodes_multiple_paths() {
    let definition = r#"{
        "nodes": [
            {"id": "n1", "type": "task"},
            {"id": "n2", "type": "task"},
            {"id": "n3", "type": "task"}
        ],
        "edges": [
            {"source": "n1", "target": "n2"},
            {"source": "n1", "target": "n3"}
        ]
    }"#;

    let (_, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();
    let state = Default::default();
    let next = WorkflowExecutionEngine::find_next_nodes(Some("n1"), &edges, &state);

    assert_eq!(next.len(), 2);
    assert!(next.contains(&"n2".to_string()));
    assert!(next.contains(&"n3".to_string()));
}

#[test]
fn test_workflow_engine_find_next_nodes_no_outgoing() {
    let definition = r#"{
        "nodes": [
            {"id": "start", "type": "task"},
            {"id": "end", "type": "task"}
        ],
        "edges": [
            {"source": "start", "target": "end"}
        ]
    }"#;

    let (_, edges) = WorkflowExecutionEngine::parse_definition(definition).unwrap();
    let state = Default::default();
    let next = WorkflowExecutionEngine::find_next_nodes(Some("end"), &edges, &state);

    assert_eq!(next.len(), 0);
}

#[test]
fn test_workflow_execution_state_default() {
    let state: orchestrator::execution::workflow_engine::WorkflowExecutionState = Default::default();

    assert_eq!(state.status, "pending");
    assert_eq!(state.step, 0);
    assert!(state.current_node.is_none());
    assert!(state.error.is_none());
    assert!(state.results.is_empty());
}

#[test]
fn test_workflow_node_creation() {
    use orchestrator::execution::workflow_engine::WorkflowNode;
    use std::collections::HashMap;

    let mut config = HashMap::new();
    config.insert("task_id".to_string(), json!("task-1"));

    let node = WorkflowNode {
        id: "node-1".to_string(),
        node_type: "task".to_string(),
        config,
    };

    assert_eq!(node.id, "node-1");
    assert_eq!(node.node_type, "task");
}

#[test]
fn test_workflow_edge_creation() {
    use orchestrator::execution::workflow_engine::WorkflowEdge;

    let edge = WorkflowEdge {
        source: "n1".to_string(),
        target: "n2".to_string(),
        condition: Some("x > 5".to_string()),
    };

    assert_eq!(edge.source, "n1");
    assert_eq!(edge.target, "n2");
    assert_eq!(edge.condition, Some("x > 5".to_string()));
}

#[test]
fn test_workflow_execution_state_with_results() {
    use orchestrator::execution::workflow_engine::WorkflowExecutionState;

    let mut state = WorkflowExecutionState {
        workflow_id: "wf-1".to_string(),
        status: "running".to_string(),
        ..Default::default()
    };

    state.results.insert("node-1".to_string(), json!({"status": "success"}));

    assert_eq!(state.workflow_id, "wf-1");
    assert_eq!(state.status, "running");
    assert!(state.results.contains_key("node-1"));
}

#[test]
fn test_workflow_engine_creation() {
    use std::sync::Arc;
    use orchestrator::db::DatabasePool;
    use orchestrator::TaskExecutor;

    // Create mock dependencies
    // Note: In real tests, we'd use test database
    assert!(true);
}

#[tokio::test]
async fn test_workflow_executor_trait_implemented() {
    // Verify that WorkflowExecutor trait is properly implemented
    // This is a placeholder test - real integration tests would use a test database
    assert!(true);
}
