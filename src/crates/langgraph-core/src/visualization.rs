//! Graph Visualization - Multi-format graph rendering
//!
//! This module provides tools for visualizing graph structures in three formats:
//! - **DOT/Graphviz** - Professional diagrams rendered with graphviz tools
//! - **Mermaid** - Interactive diagrams for markdown and web documentation
//! - **ASCII art** - Quick console visualization for debugging
//!
//! # Overview
//!
//! Graph visualization helps you **understand and communicate** your workflow:
//! - Debug graph structure and edge routing
//! - Document workflows in README files
//! - Generate diagrams for presentations
//! - Validate conditional logic visually
//!
//! **Use visualization when:**
//! - Developing complex multi-step workflows
//! - Debugging unexpected execution paths
//! - Creating documentation and tutorials
//! - Presenting system architecture to stakeholders
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │  CompiledGraph                                              │
//! │  • Nodes (process, route, llm_call, etc.)                  │
//! │  • Edges (direct, conditional)                             │
//! │  • Metadata (entry, subgraphs)                             │
//! └─────────────┬──────────────────────────────────────────────┘
//!               │
//!               ↓ visualize(graph, options)
//! ┌────────────────────────────────────────────────────────────┐
//! │  VisualizationOptions                                       │
//! │  • format: Dot | Mermaid | Ascii                           │
//! │  • include_details: show reads/writes                      │
//! │  • title: optional graph label                             │
//! │  • show_subgraphs: highlight nested graphs                 │
//! └─────────────┬──────────────────────────────────────────────┘
//!               │
//!               ↓ Format-specific rendering
//! ┌────────────────────────────────────────────────────────────┐
//! │  Format Renderers                                           │
//! │                                                            │
//! │  DOT:      digraph G { ... }                               │
//! │            • Graphviz compatible                           │
//! │            • Professional rendering                        │
//! │                                                            │
//! │  Mermaid:  graph TD\n  START --> process                   │
//! │            • GitHub/GitLab compatible                      │
//! │            • Interactive in browsers                       │
//! │                                                            │
//! │  ASCII:    START -> [process] -> END                       │
//! │            • Console/terminal output                       │
//! │            • Quick debugging                               │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Basic Visualization (DOT Format)
//!
//! ```rust,ignore
//! use langgraph_core::builder::StateGraph;
//! use langgraph_core::visualization::{visualize, VisualizationOptions};
//!
//! let mut graph = StateGraph::new();
//! graph.add_node("process", |state| Box::pin(async move { Ok(state) }));
//! graph.add_edge("__start__", "process");
//! graph.add_edge("process", "__end__");
//!
//! let compiled = graph.compile()?;
//!
//! // Generate DOT format
//! let dot = visualize(&compiled.graph, &VisualizationOptions::dot());
//! println!("{}", dot);
//!
//! // Save to file for rendering: dot -Tpng graph.dot -o graph.png
//! std::fs::write("graph.dot", dot)?;
//! ```
//!
//! ## Mermaid for Documentation
//!
//! ```rust,ignore
//! use langgraph_core::visualization::VisualizationOptions;
//!
//! let mermaid = visualize(
//!     &compiled.graph,
//!     &VisualizationOptions::mermaid().with_title("Chat Agent Workflow")
//! );
//!
//! // Embed in README.md:
//! // ```mermaid
//! // {mermaid}
//! // ```
//! ```
//!
//! ## ASCII for Console
//!
//! ```rust,ignore
//! let ascii = visualize(&compiled.graph, &VisualizationOptions::ascii());
//! eprintln!("Graph structure:\n{}", ascii);  // Debug output
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Document Workflow in README
//!
//! ```rust,ignore
//! use langgraph_core::visualization::{visualize, VisualizationOptions};
//!
//! // Build your graph...
//! let compiled = graph.compile()?;
//!
//! // Generate Mermaid for GitHub README
//! let mermaid = visualize(
//!     &compiled.graph,
//!     &VisualizationOptions::mermaid()
//!         .with_title("Multi-Agent Research Workflow")
//! );
//!
//! // Output markdown:
//! println!("# Architecture\n");
//! println!("```mermaid");
//! println!("{}", mermaid);
//! println!("```");
//! ```
//!
//! **Result in markdown:**
//! ```mermaid
//! graph TD
//!     START((START))
//!     process[Process Data]
//!     END((END))
//!     START --> process
//!     process --> END
//! ```
//!
//! ## Pattern 2: Debug Conditional Routing
//!
//! ```rust,ignore
//! // Build graph with conditional edges
//! graph.add_conditional_edge("router", routing_fn, branches);
//!
//! let compiled = graph.compile()?;
//!
//! // Visualize with details to see routing logic
//! let mermaid = visualize(
//!     &compiled.graph,
//!     &VisualizationOptions::mermaid()
//!         .with_details()  // Shows reads/writes channels
//! );
//!
//! // Mermaid will show:
//! // - Conditional nodes as diamonds (yellow)
//! // - Branches as dashed lines with labels
//! // - Node details (which channels they read/write)
//! ```
//!
//! ## Pattern 3: Generate Diagrams for CI/CD
//!
//! ```rust,ignore
//! use std::fs;
//!
//! // In your build/test script
//! fn generate_architecture_diagrams() -> Result<(), Box<dyn std::error::Error>> {
//!     let graphs = vec![
//!         ("chat_agent", build_chat_agent_graph()),
//!         ("research_workflow", build_research_graph()),
//!         ("data_pipeline", build_pipeline_graph()),
//!     ];
//!
//!     for (name, graph) in graphs {
//!         let compiled = graph.compile()?;
//!
//!         // Generate multiple formats
//!         let dot = visualize(&compiled.graph, &VisualizationOptions::dot());
//!         let mermaid = visualize(&compiled.graph, &VisualizationOptions::mermaid());
//!
//!         fs::write(format!("docs/{}.dot", name), dot)?;
//!         fs::write(format!("docs/{}.mmd", name), mermaid)?;
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Pattern 4: Visual Testing
//!
//! ```rust,ignore
//! #[test]
//! fn test_graph_structure_visual() {
//!     let graph = build_my_graph();
//!     let compiled = graph.compile().unwrap();
//!
//!     // Generate ASCII for test output
//!     let ascii = visualize(&compiled.graph, &VisualizationOptions::ascii());
//!
//!     // If test fails, ASCII diagram helps debug
//!     assert!(
//!         compiled.graph.nodes.contains_key("critical_node"),
//!         "Missing node. Graph structure:\n{}",
//!         ascii
//!     );
//! }
//! ```
//!
//! # Format Comparison
//!
//! | Feature | DOT/Graphviz | Mermaid | ASCII |
//! |---------|--------------|---------|-------|
//! | **Rendering** | External tool required | Browser/GitHub native | Terminal output |
//! | **Quality** | Publication-quality | Professional | Basic |
//! | **Interactivity** | Static image | Interactive in browser | None |
//! | **Colors** | Full RGB support | CSS colors | None |
//! | **File size** | Small text | Small text | Tiny |
//! | **Setup** | Install Graphviz | No setup needed | No setup needed |
//! | **Use case** | Papers, presentations | Documentation | Quick debugging |
//!
//! ## Format Examples
//!
//! For the same graph:
//!
//! **DOT (Graphviz):**
//! ```dot
//! digraph G {
//!     rankdir=TB;
//!     node [shape=box, style=rounded];
//!     "__start__" [shape=circle, style=filled, fillcolor=green];
//!     "process" [label="process"];
//!     "__end__" [shape=circle, style=filled, fillcolor=red];
//!     "__start__" -> "process";
//!     "process" -> "__end__";
//! }
//! ```
//! Render: `dot -Tpng graph.dot -o graph.png`
//!
//! **Mermaid:**
//! ```mermaid
//! graph TD
//!     START((START))
//!     process["process"]
//!     END((END))
//!     START --> process
//!     process --> END
//!     style START fill:#90EE90
//!     style END fill:#FFB6C1
//! ```
//!
//! **ASCII:**
//! ```text
//! Graph Structure:
//! ================
//!
//! START (process) ->
//!
//! [process]
//!   -> __end__
//!
//! END
//! ```
//!
//! # Visualization Options
//!
//! ## Including Node Details
//!
//! Show which channels each node reads/writes:
//!
//! ```rust,ignore
//! let detailed = visualize(
//!     &compiled.graph,
//!     &VisualizationOptions::mermaid()
//!         .with_details()  // Shows reads/writes
//! );
//! ```
//!
//! Output will include:
//! ```mermaid
//! process["process\nreads: [input]\nwrites: [output]"]
//! ```
//!
//! ## Highlighting Subgraphs
//!
//! Subgraph nodes are automatically styled differently:
//! - **Mermaid**: Hexagon shape with blue fill
//! - **DOT**: Rounded box with light blue fill
//!
//! ```rust,ignore
//! let with_subgraphs = visualize(
//!     &compiled.graph,
//!     &VisualizationOptions::mermaid()
//!         .with_subgraphs()  // Highlight nested graphs
//! );
//! ```
//!
//! ## Node Styling (Mermaid)
//!
//! Mermaid automatically applies semantic styling:
//!
//! | Node Type | Shape | Color | Example |
//! |-----------|-------|-------|---------|
//! | START | Circle | Green | `((START))` |
//! | END | Circle | Red/Pink | `((END))` |
//! | Regular | Rectangle | Gray | `[process]` |
//! | Conditional | Diamond | Yellow | `{router}` |
//! | Subgraph | Hexagon | Blue | `[{subgraph}]` |
//!
//! # Python LangGraph Comparison
//!
//! Python LangGraph uses `draw_mermaid()` and `draw_png()`:
//!
//! ```python
//! from langgraph.graph import StateGraph
//!
//! graph = StateGraph()
//! # ... build graph
//! compiled = graph.compile()
//!
//! # Generate Mermaid
//! mermaid_str = compiled.draw_mermaid()
//!
//! # Generate PNG (requires graphviz)
//! png_bytes = compiled.draw_png()
//! ```
//!
//! **Rust Equivalent:**
//! ```rust,ignore
//! use langgraph_core::visualization::{visualize, VisualizationOptions};
//!
//! let compiled = graph.compile()?;
//!
//! // Mermaid
//! let mermaid = visualize(&compiled.graph, &VisualizationOptions::mermaid());
//!
//! // DOT (generate PNG separately with graphviz command)
//! let dot = visualize(&compiled.graph, &VisualizationOptions::dot());
//! std::fs::write("graph.dot", dot)?;
//! // Then run: dot -Tpng graph.dot -o graph.png
//! ```
//!
//! **Key Differences:**
//! - Python has built-in PNG rendering, Rust outputs DOT text
//! - Both support Mermaid format
//! - Rust adds ASCII format for quick terminal output
//! - Rust requires explicit visualization function call
//!
//! # See Also
//!
//! - [`crate::builder::StateGraph`] - Build graphs to visualize
//! - [`crate::compiled::CompiledGraph`] - Contains graph structure
//! - [`crate::graph::Graph`] - Internal graph representation
//! - [Graphviz](https://graphviz.org/) - DOT format renderer
//! - [Mermaid](https://mermaid.js.org/) - Interactive diagram tool
//! - [GitHub Mermaid support](https://github.blog/2022-02-14-include-diagrams-markdown-files-mermaid/) - Native rendering

use crate::graph::{Edge, Graph, END, START};

/// Graph visualization format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizationFormat {
    /// DOT format for Graphviz
    Dot,
    /// Mermaid diagram format  
    Mermaid,
    /// Simple ASCII art
    Ascii,
}

/// Visualization options
#[derive(Debug, Clone)]
pub struct VisualizationOptions {
    /// Output format
    pub format: VisualizationFormat,
    /// Include node details (reads/writes channels)
    pub include_details: bool,
    /// Graph title/label
    pub title: Option<String>,
    /// Whether to show subgraphs
    pub show_subgraphs: bool,
}

impl Default for VisualizationOptions {
    fn default() -> Self {
        Self {
            format: VisualizationFormat::Dot,
            include_details: false,
            title: None,
            show_subgraphs: true,
        }
    }
}

impl VisualizationOptions {
    /// Create with DOT format
    pub fn dot() -> Self {
        Self {
            format: VisualizationFormat::Dot,
            ..Default::default()
        }
    }

    /// Create with Mermaid format
    pub fn mermaid() -> Self {
        Self {
            format: VisualizationFormat::Mermaid,
            ..Default::default()
        }
    }

    /// Create with ASCII format
    pub fn ascii() -> Self {
        Self {
            format: VisualizationFormat::Ascii,
            ..Default::default()
        }
    }

    /// Set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Include node details
    pub fn with_details(mut self) -> Self {
        self.include_details = true;
        self
    }

    /// Show subgraphs
    pub fn with_subgraphs(mut self) -> Self {
        self.show_subgraphs = true;
        self
    }
}

/// Visualize a graph as a string
pub fn visualize(graph: &Graph, options: &VisualizationOptions) -> String {
    match options.format {
        VisualizationFormat::Dot => visualize_dot(graph, options),
        VisualizationFormat::Mermaid => visualize_mermaid(graph, options),
        VisualizationFormat::Ascii => visualize_ascii(graph, options),
    }
}

/// Generate DOT format visualization
fn visualize_dot(graph: &Graph, options: &VisualizationOptions) -> String {
    let mut output = String::new();
    
    // Graph header
    output.push_str("digraph G {\n");
    output.push_str("    rankdir=TB;\n");
    output.push_str("    node [shape=box, style=rounded];\n");
    
    if let Some(title) = &options.title {
        output.push_str(&format!("    labelloc=\"t\";\n"));
        output.push_str(&format!("    label=\"{}\";\n", escape_dot(title)));
    }
    
    // Special nodes
    output.push_str(&format!("    \"{}\" [shape=circle, style=filled, fillcolor=green];\n", START));
    output.push_str(&format!("    \"{}\" [shape=circle, style=filled, fillcolor=red];\n", END));
    
    // Regular nodes
    for (node_id, node_spec) in &graph.nodes {
        let label = if options.include_details {
            format!("{}\\nreads: {:?}\\nwrites: {:?}", 
                node_spec.name,
                node_spec.reads,
                node_spec.writes)
        } else {
            node_spec.name.clone()
        };
        
        let color = if node_spec.subgraph.is_some() && options.show_subgraphs {
            ", fillcolor=lightblue, style=\"rounded,filled\""
        } else {
            ""
        };
        
        output.push_str(&format!("    \"{}\" [label=\"{}\"{}];\n", 
            escape_dot(node_id),
            escape_dot(&label),
            color));
    }
    
    // Edges
    for (from, edges) in &graph.edges {
        for edge in edges {
            match edge {
                Edge::Direct(to) => {
                    output.push_str(&format!("    \"{}\" -> \"{}\";\n", 
                        escape_dot(from),
                        escape_dot(to)));
                }
                Edge::Conditional { branches, .. } => {
                    for (label, to) in branches {
                        output.push_str(&format!("    \"{}\" -> \"{}\" [label=\"{}\", style=dashed];\n",
                            escape_dot(from),
                            escape_dot(to),
                            escape_dot(label)));
                    }
                }
            }
        }
    }
    
    output.push_str("}\n");
    output
}

/// Generate Mermaid format visualization
fn visualize_mermaid(graph: &Graph, options: &VisualizationOptions) -> String {
    let mut output = String::new();

    output.push_str("%%{init: {'theme':'base', 'themeVariables': {'primaryColor':'#f4f4f4','primaryTextColor':'#333','primaryBorderColor':'#7C7C7C','lineColor':'#7C7C7C','secondaryColor':'#e8e8e8','tertiaryColor':'#fff'}}}%%\n");
    output.push_str("graph TD\n");

    if let Some(title) = &options.title {
        output.push_str(&format!("    title[\"{}\"]\n", escape_mermaid(title)));
    }

    // Node definitions with enhanced styling
    // START node - green circle
    output.push_str(&format!("    {}((START))\n", sanitize_id(START)));
    output.push_str(&format!("    style {} fill:#90EE90,stroke:#228B22,stroke-width:3px\n", sanitize_id(START)));

    // END node - red circle
    output.push_str(&format!("    {}((END))\n", sanitize_id(END)));
    output.push_str(&format!("    style {} fill:#FFB6C1,stroke:#DC143C,stroke-width:3px\n", sanitize_id(END)));

    // Identify conditional nodes (nodes with conditional edges)
    let mut conditional_nodes = std::collections::HashSet::new();
    for (from, edges) in &graph.edges {
        for edge in edges {
            if matches!(edge, Edge::Conditional { .. }) {
                conditional_nodes.insert(from.clone());
            }
        }
    }

    // Regular nodes with type-based styling
    for (node_id, node_spec) in &graph.nodes {
        let label = if options.include_details {
            format!("{}\\nreads: {:?}\\nwrites: {:?}",
                node_spec.name,
                node_spec.reads,
                node_spec.writes)
        } else {
            node_spec.name.clone()
        };

        // Determine node shape and styling based on type
        let (brackets_open, brackets_close, style_class) = if node_spec.subgraph.is_some() && options.show_subgraphs {
            // Subgraph node - hexagon with blue fill
            ("[{", "}]", "subgraph")
        } else if conditional_nodes.contains(node_id) {
            // Conditional router - diamond with yellow fill
            ("{", "}", "conditional")
        } else {
            // Regular node - rectangle with default fill
            ("[", "]", "default")
        };

        output.push_str(&format!("    {}{}\"{}\"{}\n",
            sanitize_id(node_id),
            brackets_open,
            escape_mermaid(&label),
            brackets_close));

        // Apply styling
        match style_class {
            "subgraph" => {
                output.push_str(&format!("    style {} fill:#ADD8E6,stroke:#4682B4,stroke-width:2px\n",
                    sanitize_id(node_id)));
            }
            "conditional" => {
                output.push_str(&format!("    style {} fill:#FFE4B5,stroke:#FF8C00,stroke-width:2px\n",
                    sanitize_id(node_id)));
            }
            _ => {
                output.push_str(&format!("    style {} fill:#F0F0F0,stroke:#666,stroke-width:2px\n",
                    sanitize_id(node_id)));
            }
        }
    }

    // Edges with enhanced styling
    for (from, edges) in &graph.edges {
        for edge in edges {
            match edge {
                Edge::Direct(to) => {
                    output.push_str(&format!("    {} --> {}\n",
                        sanitize_id(from),
                        sanitize_id(to)));
                }
                Edge::Conditional { branches, .. } => {
                    // Conditional edges with dashed lines and labels
                    for (label, to) in branches {
                        output.push_str(&format!("    {} -.\"{}\"..-> {}\n",
                            sanitize_id(from),
                            escape_mermaid(label),
                            sanitize_id(to)));
                    }
                }
            }
        }
    }

    // Add legend if there are different node types
    let has_subgraphs = graph.nodes.values().any(|n| n.subgraph.is_some());
    let has_conditionals = !conditional_nodes.is_empty();

    if (has_subgraphs || has_conditionals) && options.show_subgraphs {
        output.push_str("\n    %% Legend\n");
        if has_subgraphs {
            output.push_str("    subgraph Legend\n");
            output.push_str("        direction LR\n");
            output.push_str("        legend_subgraph[{Subgraph Node}]\n");
            output.push_str("        style legend_subgraph fill:#ADD8E6,stroke:#4682B4\n");
            if has_conditionals {
                output.push_str("        legend_conditional{Conditional Router}\n");
                output.push_str("        style legend_conditional fill:#FFE4B5,stroke:#FF8C00\n");
            }
            output.push_str("    end\n");
        }
    }

    output
}

/// Generate simple ASCII art visualization
fn visualize_ascii(graph: &Graph, _options: &VisualizationOptions) -> String {
    let mut output = String::new();
    
    output.push_str("Graph Structure:\n");
    output.push_str("================\n\n");
    
    // Entry point
    output.push_str(&format!("START ({}) ->\n", graph.entry));
    
    // Nodes and their connections
    for (node_id, node_spec) in &graph.nodes {
        output.push_str(&format!("\n[{}]", node_spec.name));
        if node_spec.subgraph.is_some() {
            output.push_str(" (subgraph)");
        }
        output.push_str("\n");
        
        if let Some(edges) = graph.edges.get(node_id) {
            for edge in edges {
                match edge {
                    Edge::Direct(to) => {
                        output.push_str(&format!("  -> {}\n", to));
                    }
                    Edge::Conditional { branches, .. } => {
                        output.push_str("  -> (conditional)\n");
                        for (label, to) in branches {
                            output.push_str(&format!("     [{}] -> {}\n", label, to));
                        }
                    }
                }
            }
        }
    }
    
    output.push_str("\nEND\n");
    output
}

/// Escape special characters for DOT format
fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Escape special characters for Mermaid format
fn escape_mermaid(s: &str) -> String {
    s.replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Sanitize node IDs for Mermaid (must be alphanumeric + underscore)
fn sanitize_id(s: &str) -> String {
    s.replace("__", "")
        .replace('-', "_")
        .replace('.', "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::StateGraph;

    #[test]
    fn test_visualize_simple_graph_dot() {
        let mut builder = StateGraph::new();
        builder.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_edge("__start__", "process");
        builder.add_edge("process", "__end__");
        
        let compiled = builder.compile().unwrap();
        let dot = visualize(&compiled.graph, &VisualizationOptions::dot());
        
        assert!(dot.contains("digraph G"));
        assert!(dot.contains("__start__"));
        assert!(dot.contains("process"));
        assert!(dot.contains("__end__"));
    }

    #[test]
    fn test_visualize_simple_graph_mermaid() {
        let mut builder = StateGraph::new();
        builder.add_node("step1", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_node("step2", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_edge("__start__", "step1");
        builder.add_edge("step1", "step2");
        builder.add_edge("step2", "__end__");
        
        let compiled = builder.compile().unwrap();
        let mermaid = visualize(&compiled.graph, &VisualizationOptions::mermaid());
        
        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("START"));
        assert!(mermaid.contains("step1"));
        assert!(mermaid.contains("step2"));
        assert!(mermaid.contains("END"));
    }

    #[test]
    fn test_visualize_simple_graph_ascii() {
        let mut builder = StateGraph::new();
        builder.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_edge("__start__", "process");
        builder.add_edge("process", "__end__");

        let compiled = builder.compile().unwrap();
        let ascii = visualize(&compiled.graph, &VisualizationOptions::ascii());

        assert!(ascii.contains("Graph Structure"));
        assert!(ascii.contains("START"));
        assert!(ascii.contains("process"));
        assert!(ascii.contains("END"));
    }

    #[test]
    fn test_mermaid_with_enhanced_styling() {
        let mut builder = StateGraph::new();
        builder.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_edge("__start__", "process");
        builder.add_edge("process", "__end__");

        let compiled = builder.compile().unwrap();
        let mermaid = visualize(&compiled.graph, &VisualizationOptions::mermaid());

        // Should have theme configuration
        assert!(mermaid.contains("%%{init:"));

        // Should have styled START node (green)
        assert!(mermaid.contains("style"));
        assert!(mermaid.contains("#90EE90")); // Light green fill for START

        // Should have styled END node (red)
        assert!(mermaid.contains("#FFB6C1")); // Light pink fill for END

        // Should have styled regular node (gray)
        assert!(mermaid.contains("#F0F0F0")); // Gray fill for regular nodes
    }

    #[test]
    fn test_mermaid_with_conditional_edges() {
        use std::collections::HashMap;

        let mut builder = StateGraph::new();
        builder.add_node("router", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_node("path_a", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_node("path_b", |state| {
            Box::pin(async move { Ok(state) })
        });

        let mut branches = HashMap::new();
        branches.insert("a".to_string(), "path_a".to_string());
        branches.insert("b".to_string(), "path_b".to_string());

        builder.add_edge("__start__", "router");
        builder.add_conditional_edge(
            "router",
            |state| {
                use crate::send::ConditionalEdgeResult;
                if let Some(choice) = state.get("choice").and_then(|v| v.as_str()) {
                    if choice == "a" {
                        return ConditionalEdgeResult::Node("path_a".to_string());
                    }
                }
                ConditionalEdgeResult::Node("path_b".to_string())
            },
            branches,
        );
        builder.add_finish("path_a");
        builder.add_finish("path_b");

        let compiled = builder.compile().unwrap();
        let mermaid = visualize(&compiled.graph, &VisualizationOptions::mermaid());

        // Should have conditional router styled as diamond with yellow fill
        assert!(mermaid.contains("router{"));
        assert!(mermaid.contains("#FFE4B5")); // Light yellow fill for conditional

        // Should have dashed lines for conditional edges
        assert!(mermaid.contains("-.\""));
        assert!(mermaid.contains("\"..->"));

        // Should have branch labels
        assert!(mermaid.contains("\"a\"") || mermaid.contains("\"b\""));

        // Should have legend for conditional nodes
        assert!(mermaid.contains("Legend") || mermaid.contains("legend_conditional"));
    }

    #[test]
    fn test_mermaid_node_shapes() {
        let mut builder = StateGraph::new();
        builder.add_node("regular_node", |state| {
            Box::pin(async move { Ok(state) })
        });

        builder.add_edge("__start__", "regular_node");
        builder.add_edge("regular_node", "__end__");

        let compiled = builder.compile().unwrap();
        let mermaid = visualize(&compiled.graph, &VisualizationOptions::mermaid());

        // START should be a circle
        assert!(mermaid.contains("((START))"));

        // END should be a circle
        assert!(mermaid.contains("((END))"));

        // Regular node should be a rectangle
        assert!(mermaid.contains("[\"regular_node\"]"));
    }

    #[test]
    fn test_mermaid_with_details() {
        let mut builder = StateGraph::new();
        builder.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_edge("__start__", "process");
        builder.add_edge("process", "__end__");

        let compiled = builder.compile().unwrap();
        let mermaid = visualize(
            &compiled.graph,
            &VisualizationOptions::mermaid().with_details()
        );

        // Should include reads/writes information
        assert!(mermaid.contains("reads:") || mermaid.contains("writes:"));
    }

    #[test]
    fn test_mermaid_with_title() {
        let mut builder = StateGraph::new();
        builder.add_node("process", |state| {
            Box::pin(async move { Ok(state) })
        });
        builder.add_edge("__start__", "process");
        builder.add_edge("process", "__end__");

        let compiled = builder.compile().unwrap();
        let mermaid = visualize(
            &compiled.graph,
            &VisualizationOptions::mermaid().with_title("My Test Graph")
        );

        // Should have title
        assert!(mermaid.contains("My Test Graph"));
    }
}
