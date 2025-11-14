//! # langgraph-cli
//!
//! CLI tool for rLangGraph development and management.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "langgraph")]
#[command(about = "rLangGraph CLI - Build and manage LangGraph applications", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new LangGraph project
    Init {
        /// Project name
        name: String,

        /// Project directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Create a new graph from template
    New {
        /// Graph name
        name: String,

        /// Template type
        #[arg(short, long, default_value = "simple")]
        template: String,
    },

    /// Validate a YAML graph definition
    Validate {
        /// Path to YAML file
        file: PathBuf,
    },

    /// Check graph structure
    Check {
        /// Path to YAML file
        file: PathBuf,
    },

    /// Run a graph
    Run {
        /// Path to YAML or Rust file
        file: PathBuf,

        /// Input JSON
        #[arg(short, long)]
        input: Option<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name, path } => {
            let project_path = path.unwrap_or_else(|| PathBuf::from(&name));
            println!("Initializing new LangGraph project: {}", name);
            println!("Location: {}", project_path.display());
            init_project(&name, &project_path)?;
        }
        Commands::New { name, template } => {
            println!("Creating new graph: {} (template: {})", name, template);
            create_graph(&name, &template)?;
        }
        Commands::Validate { file } => {
            println!("Validating: {}", file.display());
            validate_yaml(&file)?;
        }
        Commands::Check { file } => {
            println!("Checking graph structure: {}", file.display());
            check_graph(&file)?;
        }
        Commands::Run { file, input } => {
            println!("Running: {}", file.display());
            run_graph(&file, input.as_deref())?;
        }
    }

    Ok(())
}

fn init_project(name: &str, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Create project directory
    fs::create_dir_all(path)?;

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
        name
    );
    fs::write(path.join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    fs::create_dir_all(path.join("src"))?;

    // Create main.rs
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
    fs::write(path.join("src/main.rs"), main_rs)?;

    // Create graphs directory
    fs::create_dir_all(path.join("graphs"))?;

    // Create example graph YAML
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
    fs::write(path.join("graphs/example.yaml"), example_yaml)?;

    println!("✓ Created project structure");
    println!("✓ Created Cargo.toml");
    println!("✓ Created src/main.rs");
    println!("✓ Created graphs/example.yaml");
    println!("\nNext steps:");
    println!("  cd {}", name);
    println!("  cargo build");
    println!("  cargo run");

    Ok(())
}

fn create_graph(name: &str, template: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    let yaml_content = match template {
        "simple" => format!(
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
            name
        ),
        "conditional" => format!(
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
            name
        ),
        _ => {
            return Err(format!("Unknown template: {}", template).into());
        }
    };

    let filename = format!("graphs/{}.yaml", name);
    fs::create_dir_all("graphs")?;
    fs::write(&filename, yaml_content)?;

    println!("✓ Created graph: {}", filename);
    Ok(())
}

fn validate_yaml(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use langgraph_core::yaml::YamlGraphDef;

    let graph_def = YamlGraphDef::from_file(file)?;
    graph_def.validate()?;

    println!("✓ YAML is valid");
    println!("  Graph name: {}", graph_def.name);
    println!("  Nodes: {}", graph_def.nodes.len());
    println!("  Edges: {}", graph_def.edges.len());
    println!("  Entry point: {}", graph_def.entry);

    Ok(())
}

fn check_graph(file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use langgraph_core::yaml::YamlGraphDef;

    let graph_def = YamlGraphDef::from_file(file)?;
    graph_def.validate()?;

    println!("✓ Graph structure is valid");
    println!("\nGraph Analysis:");
    println!("  Name: {}", graph_def.name);
    if let Some(desc) = &graph_def.description {
        println!("  Description: {}", desc);
    }
    println!("  Entry point: {}", graph_def.entry);
    println!("\nNodes ({}):", graph_def.nodes.len());
    for (name, node) in &graph_def.nodes {
        println!("  - {}: {}", name, node.handler);
        if let Some(desc) = &node.description {
            println!("    {}", desc);
        }
    }

    println!("\nEdges ({}):", graph_def.edges.len());
    for edge in &graph_def.edges {
        match edge {
            langgraph_core::yaml::YamlEdgeDef::Direct { from, to } => {
                println!("  - {} -> {}", from, to);
            }
            langgraph_core::yaml::YamlEdgeDef::Conditional {
                from,
                condition,
                branches,
            } => {
                println!("  - {} -> [conditional: {}]", from, condition);
                for (key, target) in branches {
                    println!("      {} -> {}", key, target);
                }
            }
        }
    }

    Ok(())
}

fn run_graph(_file: &PathBuf, _input: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Graph execution not yet implemented");
    println!("This feature requires runtime handler resolution");
    Ok(())
}
