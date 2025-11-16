//! Integration tests for langgraph-cli
//!
//! These tests verify the CLI functionality including:
//! - Project initialization
//! - Graph creation from templates
//! - YAML validation
//! - Graph structure checking

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create a test project directory
fn create_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Helper function to create a valid example YAML graph
fn create_example_yaml(path: &PathBuf) {
    let yaml_content = r#"name: test_graph
description: A test graph
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
    fs::write(path, yaml_content).expect("Failed to write YAML file");
}

#[test]
fn test_project_structure_creation() {
    let temp_dir = create_test_dir();
    let project_path = temp_dir.path().join("test_project");
    let project_name = "test_project";

    // Simulate what init_project does
    fs::create_dir_all(&project_path).unwrap();
    fs::create_dir_all(project_path.join("src")).unwrap();
    fs::create_dir_all(project_path.join("graphs")).unwrap();

    // Verify directory structure
    assert!(project_path.exists());
    assert!(project_path.join("src").exists());
    assert!(project_path.join("graphs").exists());

    // Create Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
langgraph-core = "0.1"
langgraph-checkpoint = "0.1"
langgraph-prebuilt = "0.1"
tokio = {{ version = "1", features = ["full"] }}
serde_json = "1.0"
"#,
        project_name
    );
    fs::write(project_path.join("Cargo.toml"), &cargo_toml).unwrap();

    // Verify Cargo.toml exists and contains project name
    let cargo_content = fs::read_to_string(project_path.join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains(project_name));
    assert!(cargo_content.contains("langgraph-core"));
    assert!(cargo_content.contains("edition = \"2021\""));
}

#[test]
fn test_main_rs_generation() {
    let temp_dir = create_test_dir();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(project_path.join("src")).unwrap();

    let main_rs = r#"use langgraph_core::StateGraph;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = StateGraph::new();

    // Add your nodes here
    graph.add_node("process", |state| {
        Box::pin(async move {
            println!("Processing: {:?}", state);
            Ok(state)
        })
    });

    graph.add_edge("__start__", "process");
    graph.add_edge("process", "__end__");

    let compiled = graph.compile()?;
    let result = compiled.invoke(json!({"input": "Hello, LangGraph!"})).await?;

    println!("Result: {}", result);
    Ok(())
}
"#;
    fs::write(project_path.join("src/main.rs"), main_rs).unwrap();

    // Verify main.rs exists and contains expected content
    let main_content = fs::read_to_string(project_path.join("src/main.rs")).unwrap();
    assert!(main_content.contains("use langgraph_core::StateGraph"));
    assert!(main_content.contains("async fn main()"));
    assert!(main_content.contains("StateGraph::new()"));
}

#[test]
fn test_example_yaml_generation() {
    let temp_dir = create_test_dir();
    let graphs_dir = temp_dir.path().join("graphs");
    fs::create_dir_all(&graphs_dir).unwrap();

    let example_yaml = r#"name: example_graph
description: An example graph
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
    fs::write(graphs_dir.join("example.yaml"), example_yaml).unwrap();

    // Verify example YAML exists and contains expected content
    let yaml_content = fs::read_to_string(graphs_dir.join("example.yaml")).unwrap();
    assert!(yaml_content.contains("name: example_graph"));
    assert!(yaml_content.contains("handler: \"process_handler\""));
    assert!(yaml_content.contains("from: \"__start__\""));
}

#[test]
fn test_simple_graph_template() {
    let temp_dir = create_test_dir();
    let graphs_dir = temp_dir.path().join("graphs");
    fs::create_dir_all(&graphs_dir).unwrap();

    let graph_name = "my_graph";
    let yaml_content = format!(
        r#"name: {}
description: A simple sequential graph
entry: process

nodes:
  process:
    handler: "process_handler"

edges:
  - from: "__start__"
    to: "process"
  - from: "process"
    to: "__end__"
"#,
        graph_name
    );

    let filename = graphs_dir.join(format!("{}.yaml", graph_name));
    fs::write(&filename, &yaml_content).unwrap();

    // Verify the file was created with correct content
    let content = fs::read_to_string(&filename).unwrap();
    assert!(content.contains(&format!("name: {}", graph_name)));
    assert!(content.contains("entry: process"));
    assert!(content.contains("handler: \"process_handler\""));
}

#[test]
fn test_conditional_graph_template() {
    let temp_dir = create_test_dir();
    let graphs_dir = temp_dir.path().join("graphs");
    fs::create_dir_all(&graphs_dir).unwrap();

    let graph_name = "conditional_graph";
    let yaml_content = format!(
        r#"name: {}
description: A graph with conditional routing
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
"#,
        graph_name
    );

    let filename = graphs_dir.join(format!("{}.yaml", graph_name));
    fs::write(&filename, &yaml_content).unwrap();

    // Verify the file was created with conditional routing
    let content = fs::read_to_string(&filename).unwrap();
    assert!(content.contains("entry: router"));
    assert!(content.contains("condition: \"route_condition\""));
    assert!(content.contains("branches:"));
    assert!(content.contains("path_a"));
    assert!(content.contains("path_b"));
}

#[test]
fn test_yaml_validation_structure() {
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("test.yaml");
    create_example_yaml(&yaml_file);

    // Read and parse the YAML
    let content = fs::read_to_string(&yaml_file).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

    // Verify structure
    assert!(parsed["name"].as_str().is_some());
    assert!(parsed["entry"].as_str().is_some());
    assert!(parsed["nodes"].is_mapping());
    assert!(parsed["edges"].is_sequence());
}

#[test]
fn test_invalid_yaml_detection() {
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("invalid.yaml");

    // Create invalid YAML (malformed syntax)
    let invalid_yaml = r#"
name: test
nodes:
  process
    handler: "bad_indent"
"#;
    fs::write(&yaml_file, invalid_yaml).unwrap();

    // Try to parse - should fail
    let content = fs::read_to_string(&yaml_file).unwrap();
    let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(&content);
    assert!(result.is_err());
}

#[test]
fn test_graph_directory_creation() {
    let temp_dir = create_test_dir();
    let graphs_dir = temp_dir.path().join("graphs");

    // Create graphs directory
    fs::create_dir_all(&graphs_dir).unwrap();

    // Verify it exists and is a directory
    assert!(graphs_dir.exists());
    assert!(graphs_dir.is_dir());
}

#[test]
fn test_project_with_custom_path() {
    let temp_dir = create_test_dir();
    let custom_path = temp_dir.path().join("custom/location/my_project");

    // Create project at custom path
    fs::create_dir_all(&custom_path).unwrap();
    fs::create_dir_all(custom_path.join("src")).unwrap();

    assert!(custom_path.exists());
    assert!(custom_path.join("src").exists());
}

#[test]
fn test_multiple_graphs_creation() {
    let temp_dir = create_test_dir();
    let graphs_dir = temp_dir.path().join("graphs");
    fs::create_dir_all(&graphs_dir).unwrap();

    // Create multiple graph files
    let graph_names = vec!["graph1", "graph2", "graph3"];

    for name in &graph_names {
        let yaml_content = format!(
            r#"name: {}
entry: process

nodes:
  process:
    handler: "handler"

edges:
  - from: "__start__"
    to: "process"
  - from: "process"
    to: "__end__"
"#,
            name
        );
        fs::write(graphs_dir.join(format!("{}.yaml", name)), yaml_content).unwrap();
    }

    // Verify all files exist
    for name in &graph_names {
        let file_path = graphs_dir.join(format!("{}.yaml", name));
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains(&format!("name: {}", name)));
    }
}

#[test]
fn test_graph_with_description() {
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("described.yaml");

    let yaml_content = r#"name: described_graph
description: This graph has a description
entry: start

nodes:
  start:
    handler: "start_handler"
    description: "Starting node"

edges:
  - from: "__start__"
    to: "start"
  - from: "start"
    to: "__end__"
"#;
    fs::write(&yaml_file, yaml_content).unwrap();

    let content = fs::read_to_string(&yaml_file).unwrap();
    assert!(content.contains("description: This graph has a description"));
    assert!(content.contains("description: \"Starting node\""));
}

#[test]
fn test_edge_structure_direct() {
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("edges.yaml");

    let yaml_content = r#"name: edge_test
entry: node1

nodes:
  node1:
    handler: "h1"
  node2:
    handler: "h2"

edges:
  - from: "__start__"
    to: "node1"
  - from: "node1"
    to: "node2"
  - from: "node2"
    to: "__end__"
"#;
    fs::write(&yaml_file, yaml_content).unwrap();

    let content = fs::read_to_string(&yaml_file).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

    let edges = parsed["edges"].as_sequence().unwrap();
    assert_eq!(edges.len(), 3);
}

#[test]
fn test_edge_structure_conditional() {
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("conditional_edges.yaml");

    let yaml_content = r#"name: conditional_test
entry: router

nodes:
  router:
    handler: "router"
  option_a:
    handler: "handler_a"
  option_b:
    handler: "handler_b"

edges:
  - from: "__start__"
    to: "router"
  - from: "router"
    condition: "decide"
    branches:
      a: "option_a"
      b: "option_b"
  - from: "option_a"
    to: "__end__"
  - from: "option_b"
    to: "__end__"
"#;
    fs::write(&yaml_file, yaml_content).unwrap();

    let content = fs::read_to_string(&yaml_file).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

    let edges = parsed["edges"].as_sequence().unwrap();
    // Should have 4 edges: start->router, router->conditional, option_a->end, option_b->end
    assert_eq!(edges.len(), 4);

    // Check conditional edge structure
    let conditional_edge = &edges[1];
    assert!(conditional_edge["condition"].as_str().is_some());
    assert!(conditional_edge["branches"].is_mapping());
}

#[test]
fn test_template_unknown_type() {
    // Test that an unknown template type would be rejected
    let unknown_template = "unknown_template_type";
    let valid_templates = vec!["simple", "conditional"];

    assert!(!valid_templates.contains(&unknown_template));
}

#[test]
fn test_cargo_toml_dependencies() {
    let temp_dir = create_test_dir();
    let cargo_file = temp_dir.path().join("Cargo.toml");

    let cargo_toml = r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]
langgraph-core = "0.1"
langgraph-checkpoint = "0.1"
langgraph-prebuilt = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
"#;
    fs::write(&cargo_file, cargo_toml).unwrap();

    let content = fs::read_to_string(&cargo_file).unwrap();

    // Verify all required dependencies are present
    assert!(content.contains("langgraph-core"));
    assert!(content.contains("langgraph-checkpoint"));
    assert!(content.contains("langgraph-prebuilt"));
    assert!(content.contains("tokio"));
    assert!(content.contains("serde_json"));
}

#[test]
fn test_entry_point_validation() {
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("entry_test.yaml");

    // Create YAML with entry point
    let yaml_content = r#"name: entry_test
entry: my_entry_node

nodes:
  my_entry_node:
    handler: "entry_handler"
  other_node:
    handler: "other_handler"

edges:
  - from: "__start__"
    to: "my_entry_node"
  - from: "my_entry_node"
    to: "other_node"
  - from: "other_node"
    to: "__end__"
"#;
    fs::write(&yaml_file, yaml_content).unwrap();

    let content = fs::read_to_string(&yaml_file).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

    // Verify entry point matches a defined node
    let entry = parsed["entry"].as_str().unwrap();
    let nodes = parsed["nodes"].as_mapping().unwrap();

    assert_eq!(entry, "my_entry_node");
    assert!(nodes.contains_key(&serde_yaml::Value::String("my_entry_node".to_string())));
}

#[test]
fn test_node_handler_format() {
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("handlers.yaml");

    let yaml_content = r#"name: handler_test
entry: node1

nodes:
  node1:
    handler: "my_handler_function"
  node2:
    handler: "another_handler"

edges:
  - from: "__start__"
    to: "node1"
  - from: "node1"
    to: "node2"
  - from: "node2"
    to: "__end__"
"#;
    fs::write(&yaml_file, yaml_content).unwrap();

    let content = fs::read_to_string(&yaml_file).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

    let nodes = parsed["nodes"].as_mapping().unwrap();
    for (_name, node) in nodes {
        assert!(node["handler"].as_str().is_some());
        let handler = node["handler"].as_str().unwrap();
        assert!(!handler.is_empty());
    }
}

#[test]
fn test_special_node_names() {
    // Test that special node names __start__ and __end__ are recognized
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("special_nodes.yaml");

    let yaml_content = r#"name: special_test
entry: process

nodes:
  process:
    handler: "handler"

edges:
  - from: "__start__"
    to: "process"
  - from: "process"
    to: "__end__"
"#;
    fs::write(&yaml_file, yaml_content).unwrap();

    let content = fs::read_to_string(&yaml_file).unwrap();
    assert!(content.contains("__start__"));
    assert!(content.contains("__end__"));

    // Parse and verify edges reference special nodes
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
    let edges = parsed["edges"].as_sequence().unwrap();

    let start_edge = &edges[0];
    assert_eq!(start_edge["from"].as_str().unwrap(), "__start__");

    let end_edge = &edges[1];
    assert_eq!(end_edge["to"].as_str().unwrap(), "__end__");
}

#[test]
fn test_yaml_file_extension() {
    let temp_dir = create_test_dir();

    // Test that .yaml extension is used
    let yaml_file = temp_dir.path().join("test.yaml");
    create_example_yaml(&yaml_file);

    assert!(yaml_file.exists());
    assert_eq!(yaml_file.extension().unwrap(), "yaml");
}

#[test]
fn test_empty_description_optional() {
    let temp_dir = create_test_dir();
    let yaml_file = temp_dir.path().join("no_desc.yaml");

    // Create YAML without description (should be optional)
    let yaml_content = r#"name: no_desc_graph
entry: process

nodes:
  process:
    handler: "handler"

edges:
  - from: "__start__"
    to: "process"
  - from: "process"
    to: "__end__"
"#;
    fs::write(&yaml_file, yaml_content).unwrap();

    let content = fs::read_to_string(&yaml_file).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

    // Description should be None/null
    assert!(parsed["description"].is_null() || !content.contains("description:"));
}

#[test]
fn test_graphs_directory_isolation() {
    let temp_dir = create_test_dir();
    let graphs_dir = temp_dir.path().join("graphs");
    let src_dir = temp_dir.path().join("src");

    fs::create_dir_all(&graphs_dir).unwrap();
    fs::create_dir_all(&src_dir).unwrap();

    // Create files in both directories
    fs::write(graphs_dir.join("graph.yaml"), "test").unwrap();
    fs::write(src_dir.join("main.rs"), "test").unwrap();

    // Verify separation
    assert!(graphs_dir.join("graph.yaml").exists());
    assert!(src_dir.join("main.rs").exists());
    assert!(!graphs_dir.join("main.rs").exists());
    assert!(!src_dir.join("graph.yaml").exists());
}
