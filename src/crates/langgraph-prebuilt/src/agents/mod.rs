//! Agent Patterns - Production-Ready LLM Agent Architectures
//!
//! This module provides **three proven agent patterns** for different use cases:
//!
//! 1. **[ReAct](react)** - Reasoning + Acting loop with tool calling
//!    - Best for: General Q&A, tool use, information retrieval
//!    - Complexity: Low
//!    - Token usage: Low
//!
//! 2. **[Plan-Execute](plan_execute)** - Explicit planning before execution
//!    - Best for: Multi-step research, complex workflows, data pipelines
//!    - Complexity: Medium
//!    - Token usage: High (generates explicit plan upfront)
//!
//! 3. **[Reflection](reflection)** - Self-critique and iterative improvement
//!    - Best for: Writing, creative tasks, quality-critical outputs
//!    - Complexity: High
//!    - Token usage: Very High (multiple LLM calls per iteration)
//!
//! # Choosing the Right Pattern
//!
//! ```text
//! Start Here
//!     │
//!     ↓
//! Need tool calling? ───→ [Yes] ───→ ReAct Agent (90% of use cases)
//!     │
//!     [No]
//!     │
//!     ↓
//! Multi-step task     ───→ [Yes] ───→ Plan-Execute Agent
//! requiring planning?
//!     │
//!     [No]
//!     │
//!     ↓
//! Need self-critique  ───→ [Yes] ───→ Reflection Agent
//! and refinement?
//!     │
//!     [No]
//!     │
//!     ↓
//! Use langgraph-core directly for custom patterns
//! ```
//!
//! # Pattern Comparison
//!
//! | Feature | ReAct | Plan-Execute | Reflection |
//! |---------|-------|--------------|------------|
//! | **Loop Structure** | Think → Act → Observe | Plan → Execute steps | Generate → Critique → Refine |
//! | **Tool Use** | Yes, on-demand | Yes, during execution | No (but can be added) |
//! | **Planning** | Implicit (step-by-step) | Explicit (upfront plan) | Implicit (iterative) |
//! | **LLM Calls** | 1-5 per task | 2-20 per task | 3-10 per task |
//! | **Latency** | Low (seconds) | Medium (tens of seconds) | High (minutes) |
//! | **Best For** | Q&A, search, API calls | Research, data pipelines | Writing, design, creative |
//!
//! # Quick Start Examples
//!
//! ## ReAct Agent
//!
//! ```rust,ignore
//! use langgraph_prebuilt::agents::create_react_agent;
//!
//! let agent = create_react_agent(llm, tools)?;
//! let result = agent.invoke("Search for Rust async best practices").await?;
//! ```
//!
//! **When it runs:**
//! 1. LLM reasons: "I need to search for this"
//! 2. Calls search tool
//! 3. LLM observes results
//! 4. Formulates final answer
//!
//! ## Plan-Execute Agent
//!
//! ```rust,ignore
//! use langgraph_prebuilt::agents::{create_plan_execute_agent, PlanExecuteConfig};
//!
//! let config = PlanExecuteConfig { max_steps: 10, replanning_enabled: true };
//! let agent = create_plan_execute_agent(planner_llm, executor_llm, tools, config)?;
//! let result = agent.invoke("Research Rust web frameworks and create comparison").await?;
//! ```
//!
//! **When it runs:**
//! 1. Planner LLM creates explicit plan: "1. Search for frameworks, 2. Compare features, 3. Summarize"
//! 2. Executor LLM executes step 1 with tools
//! 3. Executor LLM executes step 2 with tools
//! 4. Executor LLM executes step 3
//! 5. Returns final result
//!
//! ## Reflection Agent
//!
//! ```rust,ignore
//! use langgraph_prebuilt::agents::{create_reflection_agent, ReflectionConfig};
//!
//! let config = ReflectionConfig { max_iterations: 3, quality_threshold: 0.8 };
//! let agent = create_reflection_agent(generator_llm, critic_llm, config)?;
//! let result = agent.invoke("Write a blog post about Rust ownership").await?;
//! ```
//!
//! **When it runs:**
//! 1. Generator LLM writes initial draft
//! 2. Critic LLM provides critique (score: 0.6)
//! 3. Generator LLM refines based on critique
//! 4. Critic LLM evaluates again (score: 0.85)
//! 5. Quality threshold met, returns refined output
//!
//! # Implementation Details
//!
//! All three patterns are built on top of [`langgraph_core::builder::StateGraph`]:
//!
//! - **ReAct**: 2-3 node graph (agent → tools → agent loop)
//! - **Plan-Execute**: 4-5 node graph (planner → executor loop → replanner → finish)
//! - **Reflection**: 3-4 node graph (generate → critique → refine loop → finish)
//!
//! Each pattern handles:
//! - State management (messages, plan, critique)
//! - Loop termination (max iterations, quality thresholds)
//! - Error handling and retries
//! - Tool execution coordination
//!
//! # See Also
//!
//! - [`create_react_agent`](react::create_react_agent) - ReAct pattern implementation
//! - [`create_plan_execute_agent`](plan_execute::create_plan_execute_agent) - Plan-Execute implementation
//! - [`create_reflection_agent`](reflection::create_reflection_agent) - Reflection implementation
//! - [Python LangGraph Agents](https://langchain-ai.github.io/langgraph/reference/prebuilt/) - Reference patterns

pub mod react;
pub mod plan_execute;
pub mod reflection;

pub use react::create_react_agent;
pub use plan_execute::{create_plan_execute_agent, PlanExecuteConfig, PlanExecuteState, PlanStep};
pub use reflection::{create_reflection_agent, ReflectionConfig, ReflectionState, ReflectionCritique, QualityMetrics};
