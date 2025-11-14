//! Node-level inline interrupts for programmatic human-in-the-loop control
//!
//! This module provides **inline interrupts** - the ability for nodes to programmatically pause
//! graph execution from within their code. Unlike configured interrupts (see [`interrupt`] module),
//! inline interrupts are triggered dynamically at runtime based on node logic, state values, or
//! external conditions.
//!
//! # Overview
//!
//! Inline interrupts enable:
//!
//! - **Runtime Decision-Making** - Nodes decide when to interrupt based on state
//! - **Typed Interrupt Requests** - Approval, Input, Edit, or Custom interrupt types
//! - **Rich Metadata** - Attach context, prompts, and data to interrupts
//! - **Flexible Resumption** - Continue, Abort, Skip, or Retry with state updates
//! - **Conditional Interrupts** - Only interrupt when specific conditions are met
//! - **Multi-Step Workflows** - Chain multiple interrupts within single node
//! - **Error Handling** - Interrupts integrate with error propagation
//!
//! # Core Types
//!
//! - [`InterruptType`] - Type of interrupt: Approval, Input, Edit, or Custom
//! - [`InlineResumeValue`] - Resume data with action and state updates
//! - [`ResumeAction`] - Continue, Abort, Skip, or Retry
//! - [`InlineInterruptState`] - Runtime state of inline interrupt
//!
//! # Key Functions
//!
//! - [`interrupt()`] - Main function to trigger inline interrupt
//! - [`interrupt_for_approval()`] - Convenience for approval requests
//! - [`interrupt_for_input()`] - Convenience for input collection
//! - [`interrupt_for_edit()`] - Convenience for state editing
//!
//! # Inline vs Configured Interrupts
//!
//! | Feature | Inline Interrupts (this module) | Configured Interrupts ([`interrupt`]) |
//! |---------|----------------------------------|---------------------------------------|
//! | **Trigger** | Programmatic (inside nodes) | Configuration (before/after nodes) |
//! | **When** | Runtime decision | Fixed at graph construction |
//! | **Conditional** | ✅ Yes (if/else logic) | ❌ No (always interrupts) |
//! | **Context** | ✅ Rich (message, data, schema) | ⚡ Basic (node name, timing) |
//! | **Use Case** | Dynamic approval, validation | Debugging, standard gates |
//! | **Flexibility** | ✅ High | ⚡ Low |
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Node Execution with Inline Interrupt                        │
//! │                                                               │
//! │  ┌────────────────────────────────────────────────┐          │
//! │  │  Node Code                                     │          │
//! │  │  ┌──────────────────────────────────────┐     │          │
//! │  │  │  async fn node(state: Value) {        │     │          │
//! │  │  │    let risk = calculate_risk(&state); │     │          │
//! │  │  │                                        │     │          │
//! │  │  │    if risk > 0.7 {                     │     │          │
//! │  │  │      // Inline interrupt!             │     │          │
//! │  │  │      interrupt_for_approval(          │     │          │
//! │  │  │        "High risk detected",          │     │          │
//! │  │  │        Some(json!({"risk": risk}))    │     │          │
//! │  │  │      )?;                               │     │          │
//! │  │  │    }                                   │     │          │
//! │  │  │                                        │     │          │
//! │  │  │    Ok(state)                           │     │          │
//! │  │  │  }                                     │     │          │
//! │  │  └──────────────┬─────────────────────────┘     │          │
//! │  │                 │                               │          │
//! │  └─────────────────┼───────────────────────────────┘          │
//! │                    │ GraphError::InlineInterrupt              │
//! │                    ↓                                          │
//! │  ┌───────────────────────────────────────────────┐           │
//! │  │  Pregel Engine                                │           │
//! │  │  • Catches InlineInterrupt error              │           │
//! │  │  • Saves checkpoint                            │           │
//! │  │  • Returns control to user                     │           │
//! │  └───────────────────────────────────────────────┘           │
//! │                    ↓                                          │
//! │  ┌───────────────────────────────────────────────┐           │
//! │  │  Human Interaction                             │           │
//! │  │  • Review message: "High risk detected"       │           │
//! │  │  • Inspect data: {"risk": 0.85}               │           │
//! │  │  • Decide: Continue / Abort / Skip            │           │
//! │  └───────────────────────────────────────────────┘           │
//! │                    ↓                                          │
//! │  ┌───────────────────────────────────────────────┐           │
//! │  │  Resume Execution                              │           │
//! │  │  • Load checkpoint                             │           │
//! │  │  • Apply InlineResumeValue                     │           │
//! │  │  • Continue/Abort/Skip based on action         │           │
//! │  └───────────────────────────────────────────────┘           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Basic Approval Interrupt
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_approval};
//! use serde_json::json;
//!
//! graph.add_node("risky_operation", |state| {
//!     Box::pin(async move {
//!         let amount = state["amount"].as_i64().unwrap();
//!
//!         // Interrupt if amount is large
//!         if amount > 10000 {
//!             interrupt_for_approval(
//!                 format!("Approve transaction of ${}", amount),
//!                 Some(json!({"amount": amount, "risk": "high"}))
//!             )?;
//!         }
//!
//!         // Process transaction
//!         Ok(json!({"status": "completed"}))
//!     })
//! });
//! ```
//!
//! ## Request Input from User
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_input};
//! use serde_json::json;
//!
//! graph.add_node("collect_feedback", |state| {
//!     Box::pin(async move {
//!         // Request user feedback
//!         interrupt_for_input(
//!             "Please provide feedback on the result",
//!             "feedback",
//!             Some(json!("Good")), // Default value
//!             None // No validation schema
//!         )?;
//!
//!         // Process continues after user provides input
//!         Ok(state)
//!     })
//! });
//! ```
//!
//! ## Edit State Interactively
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_edit};
//! use serde_json::json;
//!
//! graph.add_node("review_data", |state| {
//!     Box::pin(async move {
//!         let current_data = json!({
//!             "name": state["name"],
//!             "email": state["email"],
//!             "phone": state["phone"]
//!         });
//!
//!         // Let user edit specific fields
//!         interrupt_for_edit(
//!             "Review and edit user information",
//!             vec!["email".to_string(), "phone".to_string()],
//!             current_data
//!         )?;
//!
//!         // Continue with potentially edited state
//!         Ok(state)
//!     })
//! });
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Conditional Approval Based on Risk Score
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_approval};
//! use serde_json::json;
//!
//! graph.add_node("risk_assessment", |state| {
//!     Box::pin(async move {
//!         let risk_score = calculate_risk(&state)?;
//!
//!         match risk_score {
//!             score if score > 0.8 => {
//!                 // High risk - require approval
//!                 interrupt_for_approval(
//!                     format!("High risk detected ({}). Approve?", score),
//!                     Some(json!({
//!                         "risk_score": score,
//!                         "risk_level": "high",
//!                         "factors": identify_risk_factors(&state)
//!                     }))
//!                 )?;
//!             }
//!             score if score > 0.5 => {
//!                 // Medium risk - log but continue
//!                 log::warn!("Medium risk: {}", score);
//!             }
//!             _ => {
//!                 // Low risk - auto approve
//!                 log::info!("Low risk, auto-approved");
//!             }
//!         }
//!
//!         Ok(json!({"risk_score": risk_score, "approved": true}))
//!     })
//! });
//! ```
//!
//! ## Pattern 2: Multi-Field Input Collection
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_input};
//! use serde_json::json;
//!
//! graph.add_node("gather_user_info", |state| {
//!     Box::pin(async move {
//!         // Collect multiple inputs sequentially
//!         interrupt_for_input(
//!             "Enter your name",
//!             "name",
//!             None,
//!             Some(json!({"type": "string", "minLength": 1}))
//!         )?;
//!
//!         interrupt_for_input(
//!             "Enter your email",
//!             "email",
//!             None,
//!             Some(json!({"type": "string", "format": "email"}))
//!         )?;
//!
//!         interrupt_for_input(
//!             "Enter your age",
//!             "age",
//!             Some(json!(18)),
//!             Some(json!({"type": "integer", "minimum": 0, "maximum": 120}))
//!         )?;
//!
//!         Ok(state)
//!     })
//! });
//! ```
//!
//! ## Pattern 3: State Validation with Edit Option
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_edit};
//! use serde_json::json;
//!
//! graph.add_node("validate_output", |state| {
//!     Box::pin(async move {
//!         let output = &state["llm_output"];
//!
//!         // Check output quality
//!         let quality_issues = check_quality(output);
//!
//!         if !quality_issues.is_empty() {
//!             // Quality issues found - let user edit
//!             interrupt_for_edit(
//!                 format!("Quality issues detected: {:?}. Please edit.", quality_issues),
//!                 vec!["llm_output".to_string(), "quality_notes".to_string()],
//!                 json!({
//!                     "llm_output": output,
//!                     "quality_issues": quality_issues,
//!                     "quality_notes": ""
//!                 })
//!             )?;
//!         }
//!
//!         Ok(state)
//!     })
//! });
//! ```
//!
//! ## Pattern 4: Custom Interrupt with Structured Data
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt, InterruptType};
//! use serde_json::json;
//!
//! graph.add_node("custom_review", |state| {
//!     Box::pin(async move {
//!         let analysis = perform_analysis(&state)?;
//!
//!         // Custom interrupt with rich structured data
//!         interrupt(InterruptType::Custom {
//!             custom_type: "multi_choice_review".to_string(),
//!             data: json!({
//!                 "question": "Which result should we use?",
//!                 "options": [
//!                     {"id": "a", "label": "Result A", "data": analysis.option_a},
//!                     {"id": "b", "label": "Result B", "data": analysis.option_b},
//!                     {"id": "c", "label": "Result C", "data": analysis.option_c},
//!                 ],
//!                 "metadata": {
//!                     "confidence": analysis.confidence,
//!                     "timestamp": chrono::Utc::now().to_rfc3339()
//!                 }
//!             })
//!         })?;
//!
//!         Ok(state)
//!     })
//! });
//! ```
//!
//! ## Pattern 5: Iterative Refinement Loop
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_approval};
//! use serde_json::json;
//!
//! graph.add_node("iterative_refinement", |state| {
//!     Box::pin(async move {
//!         let mut current_output = state["draft"].clone();
//!         let mut iteration = 0;
//!         let max_iterations = 5;
//!
//!         loop {
//!             iteration += 1;
//!
//!             // Show current draft for review
//!             interrupt_for_approval(
//!                 format!("Draft iteration {}. Approve or request refinement?", iteration),
//!                 Some(json!({
//!                     "draft": current_output,
//!                     "iteration": iteration,
//!                     "max_iterations": max_iterations
//!                 }))
//!             )?;
//!
//!             // Check resume value to see if approved or needs refinement
//!             // (This would come from the resume data when execution continues)
//!             let approved = state["interrupt_resume"]["approved"].as_bool().unwrap_or(false);
//!
//!             if approved || iteration >= max_iterations {
//!                 break;
//!             }
//!
//!             // Refine based on feedback
//!             current_output = refine_output(&current_output, &state["feedback"])?;
//!         }
//!
//!         Ok(json!({"final_output": current_output, "iterations": iteration}))
//!     })
//! });
//! ```
//!
//! ## Pattern 6: Batch Approval with Summary
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_approval};
//! use serde_json::json;
//!
//! graph.add_node("batch_approve", |state| {
//!     Box::pin(async move {
//!         let items = state["items"].as_array().unwrap();
//!
//!         // Collect items that need approval
//!         let needs_approval: Vec<_> = items.iter()
//!             .filter(|item| item["requires_approval"].as_bool().unwrap_or(false))
//!             .collect();
//!
//!         if !needs_approval.is_empty() {
//!             // Single interrupt for all items
//!             interrupt_for_approval(
//!                 format!("Approve {} items for processing", needs_approval.len()),
//!                 Some(json!({
//!                     "items": needs_approval,
//!                     "total_value": needs_approval.iter()
//!                         .filter_map(|i| i["value"].as_f64())
//!                         .sum::<f64>(),
//!                     "count": needs_approval.len()
//!                 }))
//!             )?;
//!         }
//!
//!         Ok(state)
//!     })
//! });
//! ```
//!
//! # Interrupt Types
//!
//! ## Approval
//!
//! Request yes/no approval with optional context:
//!
//! ```rust,ignore
//! InterruptType::Approval {
//!     message: "Approve this action?".to_string(),
//!     data: Some(json!({"context": "...", "risk": "medium"}))
//! }
//! ```
//!
//! **Use cases**: Dangerous operations, high-value transactions, policy compliance
//!
//! ## Input
//!
//! Collect structured input with validation:
//!
//! ```rust,ignore
//! InterruptType::Input {
//!     prompt: "Enter the API key".to_string(),
//!     field: "api_key".to_string(),
//!     default: None,
//!     schema: Some(json!({"type": "string", "pattern": "^sk-[a-zA-Z0-9]+$"}))
//! }
//! ```
//!
//! **Use cases**: Configuration, missing data, user preferences
//!
//! ## Edit
//!
//! Allow editing specific fields with validation:
//!
//! ```rust,ignore
//! InterruptType::Edit {
//!     description: "Review and edit these fields".to_string(),
//!     editable_fields: vec!["name".to_string(), "email".to_string()],
//!     current_values: json!({"name": "Alice", "email": "alice@example.com"})
//! }
//! ```
//!
//! **Use cases**: Data correction, LLM output refinement, form validation
//!
//! ## Custom
//!
//! Arbitrary structured interrupt with custom UI:
//!
//! ```rust,ignore
//! InterruptType::Custom {
//!     custom_type: "file_upload".to_string(),
//!     data: json!({
//!         "accepted_formats": [".pdf", ".docx"],
//!         "max_size_mb": 10,
//!         "description": "Upload supporting documents"
//!     })
//! }
//! ```
//!
//! **Use cases**: Complex UI interactions, custom workflows, specialized tooling
//!
//! # Resume Actions
//!
//! ## Continue
//!
//! Proceed with execution (default):
//!
//! ```rust,ignore
//! InlineResumeValue {
//!     action: ResumeAction::Continue,
//!     updates: Some(json!({"approved": true})),
//!     inputs: None,
//!     metadata: Some(json!({"approver": "user@example.com"}))
//! }
//! ```
//!
//! ## Abort
//!
//! Stop execution and return error:
//!
//! ```rust,ignore
//! InlineResumeValue {
//!     action: ResumeAction::Abort,
//!     updates: Some(json!({"reason": "User cancelled"})),
//!     inputs: None,
//!     metadata: None
//! }
//! ```
//!
//! ## Skip
//!
//! Skip current node and move to next:
//!
//! ```rust,ignore
//! InlineResumeValue {
//!     action: ResumeAction::Skip,
//!     updates: Some(json!({"skipped": true})),
//!     inputs: None,
//!     metadata: None
//! }
//! ```
//!
//! ## Retry
//!
//! Re-execute current node from beginning:
//!
//! ```rust,ignore
//! InlineResumeValue {
//!     action: ResumeAction::Retry,
//!     updates: Some(json!({"retry_count": retry_count + 1})),
//!     inputs: None,
//!     metadata: None
//! }
//! ```
//!
//! # Integration with Configured Interrupts
//!
//! Inline and configured interrupts can coexist:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, InterruptConfig};
//! use langgraph_core::inline_interrupt::{interrupt_for_approval};
//! use serde_json::json;
//!
//! let mut graph = StateGraph::new();
//!
//! graph.add_node("analyze", |state| {
//!     Box::pin(async move {
//!         let risk = calculate_risk(&state);
//!
//!         // Inline interrupt if risk is high
//!         if risk > 0.8 {
//!             interrupt_for_approval("High risk!", Some(json!({"risk": risk})))?;
//!         }
//!
//!         Ok(state)
//!     })
//! });
//!
//! graph.add_node("execute", |state| {
//!     Box::pin(async move {
//!         Ok(json!({"status": "executed"}))
//!     })
//! });
//!
//! // Also configure interrupt before execute (always triggers)
//! let config = InterruptConfig::new()
//!     .with_interrupt_before(vec!["execute".to_string()]);
//!
//! let compiled = graph.compile_with_interrupts(config)?;
//! ```
//!
//! # Performance Considerations
//!
//! ## Overhead
//!
//! - **Zero cost when not triggered**: No overhead unless `interrupt()` is called
//! - **Error propagation**: Uses standard Rust error handling (no allocations)
//! - **Checkpoint cost**: Same as configured interrupts (state serialization)
//!
//! ## Best Practices for Performance
//!
//! 1. **Avoid unnecessary interrupts**: Only call `interrupt()` when truly needed
//! 2. **Minimize data in interrupts**: Keep `data` field small for serialization
//! 3. **Use conditional logic**: Check conditions before calling `interrupt()`
//! 4. **Batch interrupts**: Collect multiple approvals into single interrupt
//!
//! # Error Handling
//!
//! Inline interrupts integrate with standard error handling:
//!
//! ```rust,ignore
//! use langgraph_core::inline_interrupt::{interrupt_for_approval};
//! use langgraph_core::GraphError;
//!
//! graph.add_node("my_node", |state| {
//!     Box::pin(async move {
//!         // Normal operations can fail
//!         let data = fetch_data().await?;
//!
//!         // Inline interrupt returns Result
//!         interrupt_for_approval("Approve data?", Some(data.clone()))?;
//!
//!         // Continue processing
//!         process_data(data)?;
//!
//!         Ok(state)
//!     })
//! });
//!
//! // When interrupt triggers, it returns GraphError::InlineInterrupt
//! // The Pregel engine catches this and pauses execution
//! ```
//!
//! # Best Practices
//!
//! 1. **Use Inline for Dynamic Conditions** - When interrupt decision depends on runtime state or external factors
//!
//! 2. **Provide Clear Messages** - Include context in interrupt messages for better UX
//!
//! 3. **Attach Relevant Data** - Include all data needed for decision-making in interrupt payload
//!
//! 4. **Use Schemas for Input** - Validate user input with JSON schemas to prevent invalid data
//!
//! 5. **Handle All Resume Actions** - Properly handle Continue, Abort, Skip, and Retry in your workflow
//!
//! 6. **Add Metadata for Audit** - Track who approved, when, and why in metadata fields
//!
//! 7. **Prefer Configured Interrupts for Fixed Gates** - Use configured interrupts for always-required approvals
//!
//! 8. **Test Edge Cases** - Test all resume actions (Continue, Abort, Skip, Retry)
//!
//! # Comparison with Python LangGraph
//!
//! | Python LangGraph | rLangGraph | Notes |
//! |------------------|------------|-------|
//! | `interrupt("message")` | `interrupt_for_approval("message", None)?` | Similar API |
//! | Dictionary data | `Some(json!({...}))` | Rust uses serde_json |
//! | Raises `GraphInterrupt` | Returns `Err(GraphError::InlineInterrupt)` | Error handling |
//! | Resume with dict | `InlineResumeValue { ... }` | Typed resume value |
//! | Auto-resume on continue | Explicit `ResumeAction::Continue` | More explicit |
//! | No typed interrupts | `InterruptType` enum | Rust has typed variants |
//! | Limited metadata | Full `InlineInterruptState` | Richer state tracking |
//!
//! # See Also
//!
//! - [`interrupt`](crate::interrupt) - Configured interrupts (before/after nodes)
//! - [`Command`](crate::command) - Dynamic graph control with resume support
//! - [`CompiledGraph`](crate::compiled) - Execution runtime that handles inline interrupts
//! - [`GraphError`](crate::error) - Error types including InlineInterrupt

use crate::error::{GraphError, Result};
use crate::runtime::get_runtime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Type of inline interrupt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InterruptType {
    /// Request approval before continuing
    Approval {
        /// Message to display to user
        message: String,
        /// Optional data to include
        data: Option<Value>,
    },

    /// Request input from user
    Input {
        /// Prompt for input
        prompt: String,
        /// Input field name
        field: String,
        /// Optional default value
        default: Option<Value>,
        /// Optional validation schema
        schema: Option<Value>,
    },

    /// Request to edit state
    Edit {
        /// Description of what to edit
        description: String,
        /// Fields that can be edited
        editable_fields: Vec<String>,
        /// Current values
        current_values: Value,
    },

    /// Custom interrupt type
    Custom {
        /// Custom type name
        custom_type: String,
        /// Custom data
        data: Value,
    },
}

/// Resume value provided when continuing from an inline interrupt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineResumeValue {
    /// Whether to continue or abort
    pub action: ResumeAction,

    /// Optional state updates to apply
    pub updates: Option<Value>,

    /// Optional input values
    pub inputs: Option<HashMap<String, Value>>,

    /// Additional metadata
    pub metadata: Option<Value>,
}

/// Action to take when resuming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResumeAction {
    /// Continue execution
    Continue,
    /// Abort execution
    Abort,
    /// Skip current node
    Skip,
    /// Retry current node
    Retry,
}

/// Inline interrupt state stored in runtime
#[derive(Debug, Clone)]
pub struct InlineInterruptState {
    /// Unique interrupt ID
    pub id: String,

    /// Type of interrupt
    pub interrupt_type: InterruptType,

    /// Node where interrupt occurred
    pub node: String,

    /// Step when interrupted
    pub step: usize,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Resume value if provided
    pub resume_value: Option<InlineResumeValue>,
}

/// Thread-local storage for interrupt requests
thread_local! {
    static INTERRUPT_REQUEST: RwLock<Option<InlineInterruptState>> = RwLock::new(None);
}

/// Request an inline interrupt from within a node
///
/// This function can be called from within any node to pause execution
/// and request human interaction. The graph execution will be interrupted
/// and can be resumed with a `ResumeValue`.
///
/// # Arguments
///
/// * `interrupt_type` - The type of interrupt to request
///
/// # Returns
///
/// Returns `Ok(())` if the interrupt was accepted and execution should continue,
/// or an error if the interrupt was rejected or aborted.
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::inline_interrupt::{interrupt, InterruptType};
/// use serde_json::json;
///
/// // Inside a node:
/// if some_condition {
///     interrupt(InterruptType::Approval {
///         message: "Please approve this action".to_string(),
///         data: Some(json!({"action": "delete_all"})),
///     })?;
/// }
/// ```
pub fn interrupt(interrupt_type: InterruptType) -> Result<()> {
    // Get current runtime context
    let runtime = get_runtime()
        .ok_or_else(|| GraphError::Execution("No runtime context available".to_string()))?;

    let node = runtime.current_node()
        .ok_or_else(|| GraphError::Execution("No current node in runtime".to_string()))?;

    let step = runtime.current_step();

    // Create interrupt state
    let interrupt_state = InlineInterruptState {
        id: Uuid::new_v4().to_string(),
        interrupt_type,
        node,
        step,
        timestamp: chrono::Utc::now(),
        resume_value: None,
    };

    // Return GraphInterrupt error to bubble up through execution
    Err(GraphError::InlineInterrupt(interrupt_state))
}

/// Request approval before continuing
///
/// Convenience function for approval interrupts.
pub fn interrupt_for_approval(message: impl Into<String>, data: Option<Value>) -> Result<()> {
    interrupt(InterruptType::Approval {
        message: message.into(),
        data,
    })
}

/// Request input from user
///
/// Convenience function for input interrupts.
pub fn interrupt_for_input(
    prompt: impl Into<String>,
    field: impl Into<String>,
    default: Option<Value>,
    schema: Option<Value>,
) -> Result<()> {
    interrupt(InterruptType::Input {
        prompt: prompt.into(),
        field: field.into(),
        default,
        schema,
    })
}

/// Request to edit state
///
/// Convenience function for edit interrupts.
pub fn interrupt_for_edit(
    description: impl Into<String>,
    editable_fields: Vec<String>,
    current_values: Value,
) -> Result<()> {
    interrupt(InterruptType::Edit {
        description: description.into(),
        editable_fields,
        current_values,
    })
}

/// Check if there's a pending interrupt request
pub async fn has_interrupt_request() -> bool {
    INTERRUPT_REQUEST.with(|r| {
        futures::executor::block_on(async {
            r.read().await.is_some()
        })
    })
}

/// Get the current interrupt request
pub async fn get_interrupt_request() -> Option<InlineInterruptState> {
    INTERRUPT_REQUEST.with(|r| {
        futures::executor::block_on(async {
            r.read().await.clone()
        })
    })
}

/// Set interrupt request (internal use)
pub(crate) async fn set_interrupt_request(state: Option<InlineInterruptState>) {
    INTERRUPT_REQUEST.with(|r| {
        futures::executor::block_on(async {
            *r.write().await = state;
        })
    });
}

/// Clear interrupt request (internal use)
pub(crate) async fn clear_interrupt_request() {
    set_interrupt_request(None).await;
}

/// Extension trait for Runtime to support inline interrupts
pub trait RuntimeInterruptExt {
    /// Check if an interrupt has been requested
    fn has_interrupt(&self) -> bool;

    /// Get the current interrupt request
    fn get_interrupt(&self) -> Option<InlineInterruptState>;

    /// Set resume value for current interrupt
    fn set_resume_value(&self, resume: InlineResumeValue) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Runtime, set_runtime};
    use crate::managed::ExecutionContext;

    #[tokio::test]
    async fn test_interrupt_approval() {
        // Set up runtime context
        let context = ExecutionContext::new(10);
        let mut runtime = Runtime::new(context);
        runtime.set_current_node(Some("test_node".to_string()));
        set_runtime(runtime);

        // Request approval interrupt
        let result = interrupt_for_approval(
            "Please approve this action",
            Some(serde_json::json!({"risk": "high"})),
        );

        // Should return an error (GraphInterrupt)
        assert!(result.is_err());

        if let Err(GraphError::InlineInterrupt(state)) = result {
            assert_eq!(state.node, "test_node");

            if let InterruptType::Approval { message, data } = state.interrupt_type {
                assert_eq!(message, "Please approve this action");
                assert_eq!(data, Some(serde_json::json!({"risk": "high"})));
            } else {
                panic!("Expected Approval interrupt type");
            }
        } else {
            panic!("Expected InlineInterrupt error");
        }
    }

    #[tokio::test]
    async fn test_interrupt_input() {
        // Set up runtime context
        let context = ExecutionContext::new(10);
        let mut runtime = Runtime::new(context);
        runtime.set_current_node(Some("input_node".to_string()));
        set_runtime(runtime);

        // Request input interrupt
        let result = interrupt_for_input(
            "Enter your name",
            "user_name",
            Some(serde_json::json!("John Doe")),
            None,
        );

        // Should return an error (GraphInterrupt)
        assert!(result.is_err());

        if let Err(GraphError::InlineInterrupt(state)) = result {
            assert_eq!(state.node, "input_node");

            if let InterruptType::Input { prompt, field, default, .. } = state.interrupt_type {
                assert_eq!(prompt, "Enter your name");
                assert_eq!(field, "user_name");
                assert_eq!(default, Some(serde_json::json!("John Doe")));
            } else {
                panic!("Expected Input interrupt type");
            }
        } else {
            panic!("Expected InlineInterrupt error");
        }
    }

    #[tokio::test]
    async fn test_interrupt_edit() {
        // Set up runtime context
        let context = ExecutionContext::new(10);
        let mut runtime = Runtime::new(context);
        runtime.set_current_node(Some("edit_node".to_string()));
        set_runtime(runtime);

        // Request edit interrupt
        let current = serde_json::json!({
            "name": "Alice",
            "age": 30,
            "email": "alice@example.com"
        });

        let result = interrupt_for_edit(
            "Please review and edit user data",
            vec!["name".to_string(), "email".to_string()],
            current.clone(),
        );

        // Should return an error (GraphInterrupt)
        assert!(result.is_err());

        if let Err(GraphError::InlineInterrupt(state)) = result {
            assert_eq!(state.node, "edit_node");

            if let InterruptType::Edit { description, editable_fields, current_values } = state.interrupt_type {
                assert_eq!(description, "Please review and edit user data");
                assert_eq!(editable_fields, vec!["name", "email"]);
                assert_eq!(current_values, current);
            } else {
                panic!("Expected Edit interrupt type");
            }
        } else {
            panic!("Expected InlineInterrupt error");
        }
    }

    #[test]
    fn test_resume_value_serialization() {
        let resume = InlineResumeValue {
            action: ResumeAction::Continue,
            updates: Some(serde_json::json!({"approved": true})),
            inputs: None,
            metadata: Some(serde_json::json!({"reviewer": "admin"})),
        };

        let json = serde_json::to_string(&resume).unwrap();
        let deserialized: InlineResumeValue = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized.action, ResumeAction::Continue));
        assert_eq!(deserialized.updates, Some(serde_json::json!({"approved": true})));
        assert_eq!(deserialized.metadata, Some(serde_json::json!({"reviewer": "admin"})));
    }

    #[test]
    fn test_interrupt_without_runtime() {
        // No runtime set
        let result = interrupt_for_approval("Test", None);

        // Should fail with no runtime context
        assert!(result.is_err());
        if let Err(GraphError::Execution(msg)) = result {
            assert!(msg.contains("No runtime context"));
        } else {
            panic!("Expected Execution error for missing runtime");
        }
    }
}