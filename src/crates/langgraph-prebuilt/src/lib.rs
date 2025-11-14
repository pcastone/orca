//! # langgraph-prebuilt - High-Level Agent Patterns
//!
//! **Ready-to-use agent architectures and utilities** for building sophisticated LLM applications
//! with minimal boilerplate. This crate provides battle-tested patterns that implement common
//! agent workflows:
//!
//! - **[ReAct Agent](agents::react)** - Reasoning + Acting loop with tool calling
//! - **[Plan-Execute Agent](agents::plan_execute)** - Planning and execution for complex tasks
//! - **[Reflection Agent](agents::reflection)** - Self-critique and iterative improvement
//! - **[Tool System](tools)** - Type-safe tool abstractions with validation
//! - **[Message Types](messages)** - Standardized message format for LLM interactions
//!
//! # Overview
//!
//! Building LLM agents from scratch requires implementing common patterns repeatedly.
//! This crate eliminates that boilerplate by providing **production-ready implementations**
//! of proven agent architectures.
//!
//! **Use this crate when you want to:**
//! - Build agents quickly without reinventing patterns
//! - Leverage proven architectures (ReAct, Plan-Execute, Reflection)
//! - Use standardized tool and message abstractions
//! - Focus on your specific domain logic, not agent plumbing
//!
//! **Use langgraph-core directly when:**
//! - You need a custom agent architecture not covered here
//! - You want fine-grained control over execution
//! - Your workflow doesn't fit standard patterns
//!
//! # Quick Start
//!
//! ## ReAct Agent with Tools
//!
//! The most common pattern - an agent that can reason and use tools:
//!
//! ```rust,ignore
//! use langgraph_prebuilt::{create_react_agent, Tool, Message};
//! use langgraph_core::builder::StateGraph;
//!
//! // Define a search tool
//! #[derive(Clone)]
//! struct SearchTool;
//!
//! impl Tool for SearchTool {
//!     fn name(&self) -> &str { "search" }
//!     fn description(&self) -> &str { "Search the web" }
//!
//!     async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
//!         // Your search implementation
//!         Ok(ToolOutput::text("Search results..."))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let tools = vec![Box::new(SearchTool) as Box<dyn Tool>];
//!
//!     // Create agent with tools
//!     let agent = create_react_agent(llm_fn, tools)?;
//!
//!     // Run the agent
//!     let input = Message::human("Search for Rust async programming");
//!     let result = agent.invoke(input).await?;
//!
//!     println!("Agent response: {}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## Plan-Execute Agent
//!
//! For complex multi-step tasks that benefit from upfront planning:
//!
//! ```rust,ignore
//! use langgraph_prebuilt::{create_plan_execute_agent, PlanExecuteConfig};
//!
//! let config = PlanExecuteConfig {
//!     max_steps: 10,
//!     replanning_enabled: true,
//! };
//!
//! let agent = create_plan_execute_agent(planner_llm, executor_llm, tools, config)?;
//! let result = agent.invoke("Research and summarize Rust concurrency").await?;
//! ```
//!
//! ## Reflection Agent
//!
//! For tasks requiring self-critique and iterative improvement:
//!
//! ```rust,ignore
//! use langgraph_prebuilt::{create_reflection_agent, ReflectionConfig};
//!
//! let config = ReflectionConfig {
//!     max_iterations: 3,
//!     quality_threshold: 0.8,
//! };
//!
//! let agent = create_reflection_agent(generator_llm, critic_llm, config)?;
//! let result = agent.invoke("Write a technical blog post about Rust").await?;
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  langgraph-prebuilt - High-Level API                        │
//! │                                                             │
//! │  ┌─────────────────┐  ┌──────────────┐  ┌───────────────┐ │
//! │  │ Agent Patterns  │  │ Tool System  │  │ Message Types │ │
//! │  │ • ReAct         │  │ • Tool trait │  │ • Human       │ │
//! │  │ • Plan-Execute  │  │ • ToolNode   │  │ • AI          │ │
//! │  │ • Reflection    │  │ • Validation │  │ • ToolCall    │ │
//! │  └─────────────────┘  └──────────────┘  └───────────────┘ │
//! └─────────────┬───────────────────────────────────────────────┘
//!               │ Uses
//!               ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │  langgraph-core - Low-Level Graph Engine                    │
//! │  • StateGraph builder                                       │
//! │  • CompiledGraph execution                                  │
//! │  • Checkpoint system                                        │
//! │  • HITL interrupts                                          │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Organization
//!
//! - **[`agents`]** - Pre-built agent patterns (ReAct, Plan-Execute, Reflection)
//! - **[`tools`]** - Tool trait, registry, and validation
//! - **[`tool_node`]** - Graph node for executing tool calls
//! - **[`messages`]** - Message types for LLM communication
//! - **[`error`]** - Error types for the prebuilt crate
//!
//! # Agent Pattern Comparison
//!
//! | Pattern | Best For | Complexity | Tokens Used |
//! |---------|----------|------------|-------------|
//! | **ReAct** | General tool use, Q&A | Low | Low |
//! | **Plan-Execute** | Multi-step research, complex workflows | Medium | High |
//! | **Reflection** | Writing, creative tasks, quality improvement | High | Very High |
//!
//! **Choosing a Pattern:**
//! - Use **ReAct** for most tasks - it's simple, fast, and works well
//! - Use **Plan-Execute** when tasks require explicit planning (research, data pipelines)
//! - Use **Reflection** when output quality matters more than speed (writing, design)
//!
//! # Python LangGraph Comparison
//!
//! | Feature | Python LangGraph | rLangGraph Prebuilt |
//! |---------|------------------|---------------------|
//! | ReAct agent | `create_react_agent()` | `create_react_agent()` |
//! | Tool calling | `@tool` decorator | `Tool` trait |
//! | Message types | `BaseMessage` hierarchy | `Message` enum |
//! | Type safety | Runtime (Pydantic) | Compile-time (Rust) |
//! | Async execution | `asyncio` | `tokio` |
//!
//! **Migration Example:**
//!
//! Python:
//! ```python
//! from langgraph.prebuilt import create_react_agent
//! from langchain_core.tools import tool
//!
//! @tool
//! def search(query: str) -> str:
//!     return "results..."
//!
//! agent = create_react_agent(llm, [search])
//! result = agent.invoke({"messages": [("human", "query")]})
//! ```
//!
//! Rust:
//! ```rust,ignore
//! use langgraph_prebuilt::{create_react_agent, Tool, ToolInput, ToolOutput};
//!
//! struct SearchTool;
//! impl Tool for SearchTool {
//!     fn name(&self) -> &str { "search" }
//!     async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
//!         Ok(ToolOutput::text("results..."))
//!     }
//! }
//!
//! let agent = create_react_agent(llm, vec![Box::new(SearchTool)])?;
//! let result = agent.invoke(Message::human("query")).await?;
//! ```
//!
//! # See Also
//!
//! - [langgraph-core](../langgraph_core) - Core graph engine
//! - [langgraph-checkpoint](../langgraph_checkpoint) - State persistence
//! - [Python LangGraph Prebuilt](https://langchain-ai.github.io/langgraph/reference/prebuilt/) - Reference implementation

pub mod error;
pub mod messages;
pub mod tools;
pub mod tool_node;
pub mod agents;

// Re-export main types
pub use error::{PrebuiltError, Result};
pub use messages::{Message, MessageType, ToolCall};
pub use tools::{Tool, ToolInput, ToolOutput, ToolRegistry};
pub use tool_node::ToolNode;
pub use agents::create_react_agent;
