//! # langgraph-core - Stateful Multi-Actor Graphs with LLMs
//!
//! **A Rust port of Python's LangGraph** - Build stateful, multi-actor applications with large language models
//! using a low-level orchestration framework inspired by Google's Pregel.
//!
//! ## Overview
//!
//! `langgraph-core` is the foundation for building complex agent workflows in Rust. It provides:
//!
//! - **Stateful graph execution** - Persistent state across multiple execution steps
//! - **Async-first design** - Non-blocking I/O with tokio/async-std support
//! - **Checkpoint/resume** - Save and restore execution state at any point
//! - **Human-in-the-loop** - Pause execution for human approval or input
//! - **Conditional routing** - Dynamic graph paths based on state or LLM output
//! - **Streaming execution** - Real-time event streams for progress monitoring
//! - **Type-safe state** - Generic state types with compile-time validation
//!
//! ## Core Concepts
//!
//! ### 1. StateGraph - Primary API
//!
//! [`StateGraph`] is the main entry point for building graphs. It manages:
//! - **Nodes**: Async functions that process and update state
//! - **Edges**: Connections between nodes (regular or conditional)
//! - **State**: Typed data structure shared across all nodes
//! - **Reducers**: Functions that combine multiple writes to the same state field
//!
//! ### 2. Pregel Execution Model
//!
//! Execution follows Google's Pregel paper:
//! - **Supersteps**: Nodes execute in coordinated rounds
//! - **Message passing**: Nodes communicate via shared state channels
//! - **Barriers**: Synchronization points between supersteps
//! - **Checkpointing**: Automatic state snapshots after each superstep
//!
//! ### 3. Checkpointing & Time Travel
//!
//! Every execution step creates a checkpoint:
//! - **Deterministic replay**: Resume from any checkpoint
//! - **Versioning**: Track state evolution over time
//! - **Branching**: Create alternate timelines from checkpoints
//! - **Debugging**: Inspect state at any point in execution
//!
//! ### 4. Human-in-the-Loop
//!
//! Pause execution for human interaction:
//! - **Interrupts**: Breakpoints triggered by conditions
//! - **Approval workflows**: Wait for explicit user confirmation
//! - **State editing**: Modify state during pause
//! - **Dynamic routing**: User chooses next path
//!
//! ## Quick Start
//!
//! ### Basic Graph
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, GraphError};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Clone, Serialize, Deserialize)]
//! struct AgentState {
//!     messages: Vec<String>,
//!     count: i32,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), GraphError> {
//!     let mut graph = StateGraph::new();
//!
//!     // Add nodes
//!     graph.add_node("process", |mut state: AgentState| {
//!         Box::pin(async move {
//!             state.messages.push("Processed!".to_string());
//!             state.count += 1;
//!             Ok(state)
//!         })
//!     });
//!
//!     // Add edges
//!     graph.add_edge("__start__", "process");
//!     graph.add_edge("process", "__end__");
//!
//!     // Compile and execute
//!     let compiled = graph.compile()?;
//!     let initial_state = AgentState {
//!         messages: vec!["Hello".to_string()],
//!         count: 0,
//!     };
//!
//!     let result = compiled.invoke(initial_state).await?;
//!     println!("Messages: {:?}, Count: {}", result.messages, result.count);
//!     Ok(())
//! }
//! ```
//!
//! ### Conditional Routing
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, GraphError};
//!
//! #[derive(Clone)]
//! struct State {
//!     value: i32,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), GraphError> {
//!     let mut graph = StateGraph::new();
//!
//!     graph.add_node("check", |state: State| {
//!         Box::pin(async move { Ok(state) })
//!     });
//!
//!     graph.add_node("positive", |mut state: State| {
//!         Box::pin(async move {
//!             state.value *= 2;
//!             Ok(state)
//!         })
//!     });
//!
//!     graph.add_node("negative", |mut state: State| {
//!         Box::pin(async move {
//!             state.value = state.value.abs();
//!             Ok(state)
//!         })
//!     });
//!
//!     // Conditional edge based on state
//!     graph.add_conditional_edge(
//!         "check",
//!         |state: &State| {
//!             Box::pin(async move {
//!                 if state.value > 0 {
//!                     Ok("positive".to_string())
//!                 } else {
//!                     Ok("negative".to_string())
//!                 }
//!             })
//!         },
//!         vec!["positive", "negative"],
//!     );
//!
//!     graph.add_edge("__start__", "check");
//!     graph.add_edge("positive", "__end__");
//!     graph.add_edge("negative", "__end__");
//!
//!     let compiled = graph.compile()?;
//!     let result = compiled.invoke(State { value: -5 }).await?;
//!     println!("Result: {}", result.value); // 5
//!     Ok(())
//! }
//! ```
//!
//! ### With Checkpointing
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, CheckpointConfig};
//! use langgraph_checkpoint::MemoryCheckpointer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut graph = StateGraph::new();
//!
//!     // Build graph...
//!     let checkpointer = MemoryCheckpointer::new();
//!     let compiled = graph.compile_with_checkpointer(checkpointer)?;
//!
//!     // First execution - creates checkpoints
//!     let config = CheckpointConfig::new("session-1");
//!     let result1 = compiled.invoke_with_config(initial_state, &config).await?;
//!
//!     // Resume from checkpoint - continues from last state
//!     let result2 = compiled.invoke_with_config(new_state, &config).await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Streaming Execution
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, StreamMode};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let compiled = graph.compile()?;
//!
//!     // Stream events as they occur
//!     let mut stream = compiled.stream(initial_state, StreamMode::Values).await?;
//!
//!     while let Some(event) = stream.next().await {
//!         match event {
//!             Ok(chunk) => println!("Node: {}, State: {:?}", chunk.node, chunk.state),
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//!                    ┌─────────────────────────────────────┐
//!                    │         StateGraph API              │
//!                    │  • add_node() • add_edge()          │
//!                    │  • add_conditional_edge()           │
//!                    │  • compile()                        │
//!                    └──────────────┬──────────────────────┘
//!                                   │
//!                                   ▼
//!                    ┌─────────────────────────────────────┐
//!                    │      CompiledGraph (Runtime)        │
//!                    │  • invoke() - One-shot execution    │
//!                    │  • stream() - Streaming events      │
//!                    │  • get_state() - State inspection   │
//!                    └──────────────┬──────────────────────┘
//!                                   │
//!                    ┌──────────────┴──────────────┐
//!                    ▼                             ▼
//!         ┌──────────────────────┐     ┌──────────────────────┐
//!         │   Pregel Executor    │     │  Checkpoint System   │
//!         │  • Superstep loop    │────▶│  • Save state        │
//!         │  • Node scheduling   │     │  • Load state        │
//!         │  • Message passing   │     │  • Version tracking  │
//!         └──────────────────────┘     └──────────────────────┘
//!                    │
//!         ┌──────────┴────────────┐
//!         ▼                       ▼
//!    ┌─────────┐          ┌──────────────┐
//!    │  Nodes  │          │   Channels   │
//!    │  (User  │◀────────▶│   (State)    │
//!    │  Logic) │          │   LastValue  │
//!    └─────────┘          │   Topic      │
//!                         │   BinaryOp   │
//!                         └──────────────┘
//! ```
//!
//! ## Module Organization
//!
//! ### Core APIs (Start Here)
//! - [`builder`] - [`StateGraph`] and graph construction API
//! - [`compiled`] - [`CompiledGraph`] runtime and execution
//! - [`graph`] - Low-level graph representation
//! - [`message_graph`] - [`MessageGraph`] for chat-based workflows
//!
//! ### State Management
//! - [`state`] - State schemas, reducers (overwrite, append, merge, sum)
//! - [`messages`] - Message types and utilities for chat history
//! - [`store`] - Persistent key-value storage ([`Store`], [`Cache`])
//! - [`managed`] - Runtime-managed values (contexts, configs)
//!
//! ### Execution Control
//! - [`stream`] - Streaming execution and event types
//! - [`send`] - Dynamic task creation via [`Send`] command
//! - [`command`] - Graph control commands (goto, resume)
//! - [`interrupt`] - Breakpoints and human-in-the-loop
//! - [`inline_interrupt`] - Inline interrupt helpers
//!
//! ### Graph Features
//! - [`subgraph`] - Nested graphs and hierarchical workflows
//! - [`parent_child`] - Parent-child graph communication
//! - [`retry`] - Retry policies with exponential backoff
//! - [`visualization`] - Graph rendering (DOT, Mermaid, ASCII)
//!
//! ### Tools & Integrations
//! - [`tool`] - Tool abstractions for agent actions
//! - [`llm_stream`] - LLM streaming token adapters
//! - [`runtime`] - Global runtime context and utilities
//! - [`cache`] - Performance caching (LRU, LFU, FIFO, TTL)
//!
//! ### Advanced
//! - [`pregel`] - Pregel execution algorithm internals
//! - [`functional`] - Functional workflow API
//! - [`yaml`] - YAML-based graph definitions
//! - [`prebuilt`] - Pre-built graph patterns
//!
//! ## Common Patterns
//!
//! ### 1. Agent Loop with Tool Calling
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, Tool};
//!
//! let mut graph = StateGraph::new();
//!
//! // Agent decides what to do
//! graph.add_node("agent", |state| async move {
//!     let tool_call = llm.call(state.messages).await?;
//!     state.next_action = tool_call;
//!     Ok(state)
//! });
//!
//! // Execute tool based on agent decision
//! graph.add_node("tools", |state| async move {
//!     let result = execute_tool(&state.next_action).await?;
//!     state.messages.push(result);
//!     Ok(state)
//! });
//!
//! // Loop until agent says "FINISH"
//! graph.add_conditional_edge("agent", |state| async move {
//!     if state.next_action == "FINISH" {
//!         Ok("__end__")
//!     } else {
//!         Ok("tools")
//!     }
//! });
//! ```
//!
//! ### 2. Multi-Agent Collaboration
//!
//! ```rust,ignore
//! let mut graph = StateGraph::new();
//!
//! graph.add_node("researcher", research_node);
//! graph.add_node("writer", writing_node);
//! graph.add_node("reviewer", review_node);
//!
//! // Sequential workflow
//! graph.add_edge("__start__", "researcher");
//! graph.add_edge("researcher", "writer");
//! graph.add_edge("writer", "reviewer");
//!
//! // Conditional loop back based on review
//! graph.add_conditional_edge("reviewer", |state| async move {
//!     if state.approved {
//!         Ok("__end__")
//!     } else {
//!         Ok("writer") // Revise
//!     }
//! });
//! ```
//!
//! ### 3. Parallel Execution with Fan-out/Fan-in
//!
//! ```rust,ignore
//! use langgraph_core::Send;
//!
//! graph.add_node("dispatch", |state| async move {
//!     // Fan-out: Create parallel tasks
//!     let tasks = state.items.iter().map(|item| {
//!         Send::new("process_item", item.clone())
//!     }).collect();
//!     Ok(tasks)
//! });
//!
//! graph.add_node("process_item", |item| async move {
//!     // Process each item independently
//!     let result = expensive_computation(item).await?;
//!     Ok(result)
//! });
//!
//! graph.add_node("aggregate", |results| async move {
//!     // Fan-in: Combine results
//!     Ok(combine(results))
//! });
//! ```
//!
//! ### 4. Human Approval Workflow
//!
//! ```rust,ignore
//! use langgraph_core::{interrupt_for_approval, InterruptConfig};
//!
//! graph.add_node("generate_action", |state| async move {
//!     let action = plan_action(state).await?;
//!
//!     // Pause for human approval
//!     let approved = interrupt_for_approval("Approve this action?").await?;
//!
//!     if approved {
//!         execute_action(action).await?;
//!     }
//!     Ok(state)
//! });
//!
//! let compiled = graph.compile_with_interrupts(
//!     InterruptConfig::before_node("generate_action")
//! )?;
//! ```
//!
//! ## Python LangGraph Comparison
//!
//! | Feature | Python LangGraph | Rust langgraph-core |
//! |---------|------------------|---------------------|
//! | **StateGraph API** | `StateGraph()` | [`StateGraph::new()`] |
//! | **MessageGraph API** | `MessageGraph()` | [`MessageGraph::new()`] |
//! | **State type** | Dict-based | Generic `T: Serialize` |
//! | **Async execution** | `async def` | `async fn` with tokio |
//! | **Checkpointing** | `MemorySaver`, SQLite | trait-based, [`langgraph_checkpoint`] |
//! | **Interrupts** | `interrupt()` | [`interrupt()`](inline_interrupt::interrupt) |
//! | **Reducers** | `Annotated[list, operator.add]` | [`AppendReducer`](state::AppendReducer) |
//! | **Streaming** | `graph.stream()` | [`CompiledGraph::stream()`] |
//! | **Subgraphs** | Nested graphs | [`CompiledSubgraph`](subgraph::CompiledSubgraph) |
//! | **Dynamic dispatch** | `Send()` | [`Send::new()`](send::Send) |
//!
//! ### Key Differences
//!
//! 1. **Type Safety**: Rust version uses generics for compile-time state validation
//! 2. **Error Handling**: `Result` types instead of exceptions
//! 3. **Async Runtime**: Explicit tokio/async-std instead of asyncio
//! 4. **Ownership**: Rust's ownership system requires explicit cloning
//! 5. **Serialization**: Uses `serde` instead of pickle/json
//!
//! ## Performance Characteristics
//!
//! - **Memory**: Zero-copy state access where possible, checkpoints require serialization
//! - **Concurrency**: Async tasks scheduled on tokio thread pool
//! - **Checkpointing**: O(state_size) serialization cost per superstep
//! - **Graph compilation**: O(nodes + edges) validation, happens once
//! - **Streaming**: Constant memory overhead, events sent as produced
//!
//! ## Best Practices
//!
//! ### 1. State Design
//!
//! ```rust,ignore
//! // ✅ Good: Flat, serializable state
//! #[derive(Clone, Serialize, Deserialize)]
//! struct State {
//!     messages: Vec<String>,
//!     count: i32,
//! }
//!
//! // ❌ Bad: Non-serializable types
//! struct State {
//!     connection: Arc<Mutex<DbConnection>>, // Can't checkpoint!
//! }
//! ```
//!
//! ### 2. Error Handling
//!
//! ```rust,ignore
//! // ✅ Good: Propagate errors
//! graph.add_node("process", |state| async move {
//!     let result = api_call().await?;
//!     Ok(state)
//! });
//!
//! // ❌ Bad: Swallow errors
//! graph.add_node("process", |state| async move {
//!     let _ = api_call().await; // Lost error!
//!     Ok(state)
//! });
//! ```
//!
//! ### 3. Node Granularity
//!
//! ```rust,ignore
//! // ✅ Good: Checkpoint-worthy boundaries
//! graph.add_node("fetch_data", fetch_node);
//! graph.add_node("process_data", process_node);
//! graph.add_node("save_results", save_node);
//!
//! // ❌ Bad: Too fine-grained (checkpoint overhead)
//! graph.add_node("validate_input", ...);
//! graph.add_node("parse_json", ...);
//! graph.add_node("extract_field_1", ...);
//! ```
//!
//! ### 4. Use Type Aliases
//!
//! ```rust,ignore
//! type AgentNode = Box<dyn Fn(State) -> BoxFuture<'static, Result<State>>>;
//!
//! fn create_node(name: &str) -> AgentNode {
//!     Box::new(move |state| Box::pin(async move { Ok(state) }))
//! }
//! ```
//!
//! ## Getting Started
//!
//! 1. **Read the basics**: Start with [`StateGraph`] and [`CompiledGraph`]
//! 2. **Understand checkpointing**: See [`langgraph_checkpoint`] crate
//! 3. **Explore patterns**: Check [`langgraph_prebuilt`] for ready-to-use agents
//! 4. **Deep dive**: Read [`pregel`] module for execution details
//!
//! ## See Also
//!
//! - [`langgraph_checkpoint`] - Checkpoint trait and implementations
//! - [`langgraph_prebuilt`] - High-level agent patterns (ReAct, Plan-Execute, Reflection)
//! - [`langgraph_cli`] - CLI tool for project management
//! - [Python LangGraph Docs](https://langchain-ai.github.io/langgraph/)
//! - [Pregel Paper](https://research.google/pubs/pub37252/)

pub mod builder;
pub mod cache;
pub mod compiled;
pub mod compiled_enhanced;
pub mod error;
pub mod graph;
pub mod yaml;
pub mod pregel;
pub mod stream;
pub mod managed;
pub mod send;
pub mod command;
pub mod node_result;
pub mod retry;
pub mod interrupt;
pub mod inline_interrupt;
pub mod state;
pub mod state_filter;
pub mod parent_child;
pub mod subgraph;
pub mod message_graph;
pub mod store;
pub mod runtime;
pub mod tool;
pub mod prebuilt;
pub mod visualization;
pub mod functional;
pub mod llm_stream;
pub mod messages;
pub mod llm;

// Re-export main types
pub use builder::StateGraph;
pub use message_graph::MessageGraph;
pub use compiled::{CompiledGraph, EventStream, ExecutionEvent, StateSnapshot, StateSnapshotStream, StreamChunkStream};
pub use langgraph_checkpoint::CheckpointConfig;
pub use error::{GraphError, Result};
pub use graph::{
    ChannelSpec, ChannelType, Edge, Graph, NodeExecutor, NodeId, NodeSpec, ReducerFn, END, START, TASKS,
};
pub use stream::{StreamConfig, StreamEvent, StreamMode, StreamChunk, Namespace};
pub use managed::{ExecutionContext, ManagedValueType};
pub use send::{ConditionalEdgeResult, Send};
pub use command::{Command, CommandGraph, GotoTarget, ResumeValue, PARENT};
pub use node_result::NodeResult;
pub use cache::{
    Cache as GraphCache, CacheConfig, CacheEntry, CacheMetrics, EvictionPolicy,
    NodeCache, ToolCache, CheckpointCache,
    create_node_cache, create_tool_cache, create_checkpoint_cache
};
pub use retry::{RetryPolicy, RetryState};
pub use interrupt::{InterruptConfig, InterruptError, InterruptState, InterruptTracker, InterruptWhen};
pub use inline_interrupt::{
    interrupt, interrupt_for_approval, interrupt_for_input, interrupt_for_edit,
    InterruptType, InlineResumeValue, ResumeAction, InlineInterruptState
};
pub use state::{StateSchema, Reducer, OverwriteReducer, AppendReducer, MergeReducer, SumReducer, StateError};
pub use state_filter::StateHistoryFilter;
pub use parent_child::{
    ParentContext, ParentMessage, SubgraphConfig, GraphHierarchy,
    send_to_parent, get_parent_context, set_parent_context, CommandParentExt
};
pub use subgraph::{
    CompiledSubgraph, create_subgraph_node, StateGraphSubgraphExt
};
pub use store::{Store, InMemoryStore, Cache, InMemoryCache, StoreError};
pub use runtime::{Runtime, StreamWriter, get_runtime, get_store, get_stream_writer};
pub use tool::{Tool, ToolRuntime, ToolRegistry, ToolCall, ToolCallResult, ToolOutput, ToolError, ToolResult};
pub use visualization::{visualize, VisualizationFormat, VisualizationOptions};
pub use functional::{Task, Workflow, WorkflowBuilder, task};
pub use llm_stream::{MessageChunk, TokenBuffer, TokenStream, MessageChunkStream, TokenStreamAdapter};
pub use messages::{
    Message, MessageRole, MessageContent, ContentPart, RemoveMessage, MessageLike,
    add_messages, add_message_likes, convert_to_messages, filter_by_role, get_last_message,
    get_messages_by_id, merge_consecutive_messages, truncate_messages,
    push_message, push_messages, trim_messages, TrimOptions, TrimStrategy
};
