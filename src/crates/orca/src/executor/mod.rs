//! Task Executor - Executes tasks using LangGraph agent patterns
//!
//! This module integrates DirectToolBridge with langgraph-prebuilt agents
//! to execute tasks with LLM-powered decision making.
//!
//! # Components
//!
//! - **Adapter** - Bridges DirectToolBridge to langgraph-prebuilt Tool trait
//! - **LLM Integration** - Wraps llm crate providers as LlmFunction
//! - **Task Executor** - Main execution engine for tasks
//! - **State Management** - Checkpointing and state tracking

mod adapter;
mod llm_provider;
mod task_executor;
pub mod retry;
pub mod budget_tracked_llm;
pub mod multi_llm_provider;

pub use adapter::ToolAdapter;
pub use llm_provider::{LlmProvider, create_llm_function};
pub use task_executor::{TaskExecutor, ExecutionResult};
pub use retry::{RetryConfig, with_retry};
pub use budget_tracked_llm::BudgetTrackedLlm;
pub use multi_llm_provider::{MultiLlmProvider, BudgetTrackedMultiLlmProvider};
