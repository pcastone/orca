// Tests for workflow CLI command handling
// Task 019: Implement Workflow CLI Commands

use aco::cli::workflow::{WorkflowDefinition, WorkflowNode, WorkflowEdge, validate_definition, definition_to_json};
use serde_json::json;

#[test]
fn test_workflow_node_creation() {
    let node = WorkflowNode {
        id: "node-1".to_string(),
        node_type: "task".to_string(),
        config: json!({"timeout": 30}),
    };

    assert_eq!(node.id, "node-1");
    assert_eq!(node.node_type, "task");
}

#[test]
fn test_workflow_edge_creation() {
    let edge = WorkflowEdge {
        source: "node-1".to_string(),
        target: "node-2".to_string(),
        condition: None,
    };

    assert_eq!(edge.source, "node-1");
    assert_eq!(edge.target, "node-2");
    assert!(edge.condition.is_none());
}

#[test]
fn test_workflow_edge_with_condition() {
    let edge = WorkflowEdge {
        source: "check".to_string(),
        target: "success".to_string(),
        condition: Some("result == true".to_string()),
    };

    assert!(edge.condition.is_some());
    assert_eq!(edge.condition.unwrap(), "result == true");
}

#[test]
fn test_workflow_definition_creation() {
    let nodes = vec![
        WorkflowNode {
            id: "start".to_string(),
            node_type: "task".to_string(),
            config: json!({}),
        },
        WorkflowNode {
            id: "end".to_string(),
            node_type: "task".to_string(),
            config: json!({}),
        },
    ];

    let edges = vec![
        WorkflowEdge {
            source: "start".to_string(),
            target: "end".to_string(),
            condition: None,
        },
    ];

    let definition = WorkflowDefinition { nodes, edges };

    assert_eq!(definition.nodes.len(), 2);
    assert_eq!(definition.edges.len(), 1);
}

#[test]
fn test_validate_definition_single_node() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "node1".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
        ],
        edges: vec![],
    };

    let result = validate_definition(&definition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_definition_linear_workflow() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "n1".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "n2".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "n3".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
        ],
        edges: vec![
            WorkflowEdge {
                source: "n1".to_string(),
                target: "n2".to_string(),
                condition: None,
            },
            WorkflowEdge {
                source: "n2".to_string(),
                target: "n3".to_string(),
                condition: None,
            },
        ],
    };

    let result = validate_definition(&definition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_definition_parallel_workflow() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "start".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "task_a".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "task_b".to_string(),
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
                target: "task_a".to_string(),
                condition: None,
            },
            WorkflowEdge {
                source: "start".to_string(),
                target: "task_b".to_string(),
                condition: None,
            },
            WorkflowEdge {
                source: "task_a".to_string(),
                target: "merge".to_string(),
                condition: None,
            },
            WorkflowEdge {
                source: "task_b".to_string(),
                target: "merge".to_string(),
                condition: None,
            },
        ],
    };

    let result = validate_definition(&definition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_definition_conditional_workflow() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "check".to_string(),
                node_type: "conditional".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "if_true".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "if_false".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
        ],
        edges: vec![
            WorkflowEdge {
                source: "check".to_string(),
                target: "if_true".to_string(),
                condition: Some("true".to_string()),
            },
            WorkflowEdge {
                source: "check".to_string(),
                target: "if_false".to_string(),
                condition: Some("false".to_string()),
            },
        ],
    };

    let result = validate_definition(&definition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_definition_empty_workflow() {
    let definition = WorkflowDefinition {
        nodes: vec![],
        edges: vec![],
    };

    let result = validate_definition(&definition);
    assert!(result.is_err());
}

#[test]
fn test_validate_definition_duplicate_node_ids() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "node".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "node".to_string(),
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
fn test_validate_definition_invalid_source_reference() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "node1".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
        ],
        edges: vec![
            WorkflowEdge {
                source: "nonexistent".to_string(),
                target: "node1".to_string(),
                condition: None,
            },
        ],
    };

    let result = validate_definition(&definition);
    assert!(result.is_err());
}

#[test]
fn test_validate_definition_invalid_target_reference() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "node1".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
        ],
        edges: vec![
            WorkflowEdge {
                source: "node1".to_string(),
                target: "nonexistent".to_string(),
                condition: None,
            },
        ],
    };

    let result = validate_definition(&definition);
    assert!(result.is_err());
}

#[test]
fn test_definition_to_json_single_node() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "node1".to_string(),
                node_type: "task".to_string(),
                config: json!({"key": "value"}),
            },
        ],
        edges: vec![],
    };

    let json = definition_to_json(&definition);

    assert!(json.get("nodes").is_some());
    assert!(json.get("edges").is_some());

    let nodes = json.get("nodes").unwrap().as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].get("id"), Some(&json!("node1")));
}

#[test]
fn test_definition_to_json_with_edges() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "n1".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "n2".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
        ],
        edges: vec![
            WorkflowEdge {
                source: "n1".to_string(),
                target: "n2".to_string(),
                condition: None,
            },
        ],
    };

    let json = definition_to_json(&definition);

    let edges = json.get("edges").unwrap().as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].get("source"), Some(&json!("n1")));
    assert_eq!(edges[0].get("target"), Some(&json!("n2")));
}

#[test]
fn test_definition_to_json_with_conditions() {
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "check".to_string(),
                node_type: "conditional".to_string(),
                config: json!({}),
            },
            WorkflowNode {
                id: "success".to_string(),
                node_type: "task".to_string(),
                config: json!({}),
            },
        ],
        edges: vec![
            WorkflowEdge {
                source: "check".to_string(),
                target: "success".to_string(),
                condition: Some("status == ok".to_string()),
            },
        ],
    };

    let json = definition_to_json(&definition);

    let edges = json.get("edges").unwrap().as_array().unwrap();
    assert!(edges[0].get("condition").is_some());
    assert_eq!(edges[0].get("condition"), Some(&json!("status == ok")));
}

#[test]
fn test_workflow_node_types() {
    let node_types = vec!["task", "conditional", "start", "end"];

    for node_type in node_types {
        let node = WorkflowNode {
            id: format!("node-{}", node_type),
            node_type: node_type.to_string(),
            config: json!({}),
        };

        assert_eq!(node.node_type, node_type);
    }
}

#[test]
fn test_workflow_node_configuration() {
    let config = json!({
        "timeout": 30,
        "retries": 3,
        "parallel": true,
        "tags": ["important", "critical"]
    });

    let node = WorkflowNode {
        id: "configured".to_string(),
        node_type: "task".to_string(),
        config: config.clone(),
    };

    assert_eq!(node.config, config);
}

#[test]
fn test_complex_workflow_structure() {
    // Test a more realistic workflow structure
    let definition = WorkflowDefinition {
        nodes: vec![
            WorkflowNode {
                id: "init".to_string(),
                node_type: "task".to_string(),
                config: json!({"name": "Initialize"}),
            },
            WorkflowNode {
                id: "validate".to_string(),
                node_type: "conditional".to_string(),
                config: json!({"check": "input"}),
            },
            WorkflowNode {
                id: "process".to_string(),
                node_type: "task".to_string(),
                config: json!({"parallel": true}),
            },
            WorkflowNode {
                id: "aggregate".to_string(),
                node_type: "task".to_string(),
                config: json!({"merge": true}),
            },
            WorkflowNode {
                id: "finalize".to_string(),
                node_type: "task".to_string(),
                config: json!({"cleanup": true}),
            },
        ],
        edges: vec![
            WorkflowEdge {
                source: "init".to_string(),
                target: "validate".to_string(),
                condition: None,
            },
            WorkflowEdge {
                source: "validate".to_string(),
                target: "process".to_string(),
                condition: Some("valid".to_string()),
            },
            WorkflowEdge {
                source: "process".to_string(),
                target: "aggregate".to_string(),
                condition: None,
            },
            WorkflowEdge {
                source: "aggregate".to_string(),
                target: "finalize".to_string(),
                condition: None,
            },
        ],
    };

    let result = validate_definition(&definition);
    assert!(result.is_ok());

    let json = definition_to_json(&definition);
    assert!(json.get("nodes").is_some());
    assert!(json.get("edges").is_some());
}

#[test]
fn test_workflow_file_extension_detection() {
    // Test that both .yaml and .yml are recognized
    let yaml_ext = "workflow.yaml";
    assert!(yaml_ext.ends_with("yaml") || yaml_ext.ends_with("yml"));

    let yml_ext = "workflow.yml";
    assert!(yml_ext.ends_with("yaml") || yml_ext.ends_with("yml"));

    let json_ext = "workflow.json";
    assert!(!json_ext.ends_with("yaml") && !json_ext.ends_with("yml"));
}

#[test]
fn test_workflow_node_count() {
    // Test various workflow sizes
    let sizes = vec![1, 2, 5, 10, 100];

    for size in sizes {
        let nodes: Vec<_> = (0..size)
            .map(|i| WorkflowNode {
                id: format!("node-{}", i),
                node_type: "task".to_string(),
                config: json!({}),
            })
            .collect();

        let definition = WorkflowDefinition {
            nodes,
            edges: vec![],
        };

        assert_eq!(definition.nodes.len(), size);
    }
}

#[test]
fn test_workflow_edge_count() {
    // Test workflow with varying edge counts
    let nodes = vec![
        WorkflowNode {
            id: "n1".to_string(),
            node_type: "task".to_string(),
            config: json!({}),
        },
        WorkflowNode {
            id: "n2".to_string(),
            node_type: "task".to_string(),
            config: json!({}),
        },
        WorkflowNode {
            id: "n3".to_string(),
            node_type: "task".to_string(),
            config: json!({}),
        },
    ];

    let edges_linear = vec![
        WorkflowEdge {
            source: "n1".to_string(),
            target: "n2".to_string(),
            condition: None,
        },
        WorkflowEdge {
            source: "n2".to_string(),
            target: "n3".to_string(),
            condition: None,
        },
    ];

    let definition = WorkflowDefinition {
        nodes,
        edges: edges_linear,
    };

    assert_eq!(definition.edges.len(), 2);
}
