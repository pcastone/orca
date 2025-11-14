//! State schema and reducer system for graph workflows
//!
//! This module provides a flexible, type-safe state management system with schema validation
//! and customizable reducers for merging concurrent updates. It's inspired by Python LangGraph's
//! `Annotated` types and reducer functions, adapted for Rust's type system.
//!
//! # Overview
//!
//! State management in rLangGraph handles:
//! - **Schema Definition**: Declare fields and their merge behavior
//! - **Reducers**: Define how concurrent writes are combined
//! - **Validation**: Ensure state conforms to schema
//! - **Type Safety**: JSON-based with schema enforcement
//!
//! # Core Concepts
//!
//! ## Reducers
//!
//! When multiple nodes write to the same state field simultaneously, reducers determine
//! how to merge the values. rLangGraph provides built-in reducers and supports custom ones.
//!
//! ### Built-in Reducers
//!
//! | Reducer | Behavior | Use Case |
//! |---------|----------|----------|
//! | [`OverwriteReducer`] | Last write wins | Simple values that should be replaced |
//! | [`AppendReducer`] | Concatenate arrays | Message history, event logs |
//! | [`MergeReducer`] | Deep merge objects | Combining partial updates |
//! | [`SumReducer`] | Add numeric values | Counters, aggregations |
//!
//! ## [`StateSchema`]
//!
//! Defines the structure of your state and how fields merge:
//!
//! ```rust
//! use langgraph_core::state::{StateSchema, AppendReducer, OverwriteReducer, SumReducer};
//! use serde_json::json;
//!
//! let mut schema = StateSchema::new();
//!
//! // Messages append to list
//! schema.add_field("messages", Box::new(AppendReducer));
//!
//! // Current step overwrites
//! schema.add_field("current_step", Box::new(OverwriteReducer));
//!
//! // Scores sum together
//! schema.add_field("total_score", Box::new(SumReducer));
//! ```
//!
//! # Examples
//!
//! ## Basic State Management
//!
//! ```rust
//! use langgraph_core::state::{StateSchema, AppendReducer, OverwriteReducer};
//! use serde_json::json;
//!
//! let mut schema = StateSchema::new();
//! schema.add_field("messages", Box::new(AppendReducer));
//! schema.add_field("status", Box::new(OverwriteReducer));
//!
//! let mut state = json!({
//!     "messages": ["Hello"],
//!     "status": "thinking"
//! });
//!
//! let update = json!({
//!     "messages": ["World"],
//!     "status": "complete"
//! });
//!
//! schema.apply(&mut state, &update).unwrap();
//!
//! // Messages appended: ["Hello", "World"]
//! assert_eq!(state["messages"].as_array().unwrap().len(), 2);
//!
//! // Status overwritten: "complete"
//! assert_eq!(state["status"], "complete");
//! ```
//!
//! ## Concurrent Node Writes
//!
//! When parallel nodes write to the same field, the reducer merges their updates:
//!
//! ```rust
//! use langgraph_core::state::{StateSchema, AppendReducer};
//! use serde_json::json;
//!
//! let mut schema = StateSchema::new();
//! schema.add_field("events", Box::new(AppendReducer));
//!
//! let mut state = json!({"events": []});
//!
//! // Node A writes
//! let update_a = json!({"events": ["node_a_started"]});
//! schema.apply(&mut state, &update_a).unwrap();
//!
//! // Node B writes (concurrent in real graph)
//! let update_b = json!({"events": ["node_b_started"]});
//! schema.apply(&mut state, &update_b).unwrap();
//!
//! // Both events preserved
//! assert_eq!(state["events"].as_array().unwrap().len(), 2);
//! ```
//!
//! ## Custom Reducers
//!
//! Implement the [`Reducer`] trait for custom merge logic:
//!
//! ```rust
//! use langgraph_core::state::{Reducer, StateError};
//! use serde_json::{json, Value};
//!
//! /// Keeps the maximum numeric value
//! struct MaxReducer;
//!
//! impl Reducer for MaxReducer {
//!     fn reduce(&self, current: &Value, update: &Value) -> Result<Value, StateError> {
//!         let a = current.as_f64().unwrap_or(f64::MIN);
//!         let b = update.as_f64().unwrap_or(f64::MIN);
//!         Ok(json!(a.max(b)))
//!     }
//!
//!     fn name(&self) -> &str {
//!         "max"
//!     }
//! }
//!
//! // Use custom reducer
//! use langgraph_core::state::StateSchema;
//! let mut schema = StateSchema::new();
//! schema.add_field("high_score", Box::new(MaxReducer));
//! ```
//!
//! ## Object Merging
//!
//! Use [`MergeReducer`] to deep-merge object fields:
//!
//! ```rust
//! use langgraph_core::state::{StateSchema, MergeReducer};
//! use serde_json::json;
//!
//! let mut schema = StateSchema::new();
//! schema.add_field("config", Box::new(MergeReducer));
//!
//! let mut state = json!({
//!     "config": {
//!         "api_key": "secret",
//!         "timeout": 30
//!     }
//! });
//!
//! let update = json!({
//!     "config": {
//!         "timeout": 60,
//!         "retries": 3
//!     }
//! });
//!
//! schema.apply(&mut state, &update).unwrap();
//!
//! // Merged: { "api_key": "secret", "timeout": 60, "retries": 3 }
//! assert_eq!(state["config"]["api_key"], "secret");
//! assert_eq!(state["config"]["timeout"], 60);
//! assert_eq!(state["config"]["retries"], 3);
//! ```
//!
//! ## Aggregations
//!
//! Use [`SumReducer`] for accumulating numeric values:
//!
//! ```rust
//! use langgraph_core::state::{StateSchema, SumReducer};
//! use serde_json::json;
//!
//! let mut schema = StateSchema::new();
//! schema.add_field("token_count", Box::new(SumReducer));
//!
//! let mut state = json!({"token_count": 100});
//!
//! // Multiple nodes add tokens
//! schema.apply(&mut state, &json!({"token_count": 50})).unwrap();
//! schema.apply(&mut state, &json!({"token_count": 75})).unwrap();
//!
//! assert_eq!(state["token_count"], 225);
//! ```
//!
//! # Integration with StateGraph
//!
//! State schemas work seamlessly with StateGraph:
//!
//! ```rust,ignore
//! use langgraph_core::{StateGraph, state::{StateSchema, AppendReducer}};
//! use serde_json::json;
//!
//! let mut schema = StateSchema::new();
//! schema.add_field("messages", Box::new(AppendReducer));
//!
//! let mut graph = StateGraph::with_schema(schema);
//!
//! graph.add_node("process", |state| {
//!     Box::pin(async move {
//!         // Update is automatically merged using schema reducers
//!         Ok(json!({"messages": ["Processed"]}))
//!     })
//! });
//! ```
//!
//! # Default Reducer
//!
//! Set a default reducer for fields not explicitly configured:
//!
//! ```rust
//! use langgraph_core::state::{StateSchema, MergeReducer, AppendReducer};
//!
//! let schema = StateSchema::new()
//!     .with_default_reducer(Box::new(MergeReducer));
//!
//! // Now all fields use MergeReducer unless explicitly set
//! ```
//!
//! # Performance Considerations
//!
//! - **Reducer Overhead**: Simple reducers (Overwrite, Sum) are O(1), complex ones (Merge) are O(n)
//! - **Append Performance**: O(k) where k is the number of items being appended
//! - **Validation**: O(n) where n is the number of schema fields
//! - **Memory**: State cloning occurs during updates, consider large state implications
//!
//! # Comparison with Python LangGraph
//!
//! This module provides Rust equivalents to Python's state management:
//!
//! | Python | Rust |
//! |--------|------|
//! | `Annotated[list, add]` | `AppendReducer` |
//! | Default (overwrite) | `OverwriteReducer` |
//! | Custom reducer function | Implement `Reducer` trait |
//! | TypedDict | `StateSchema` with validation |
//!
//! # See Also
//!
//! - [`StateGraph`](crate::StateGraph) - Building stateful graphs
//! - [`messages`](crate::messages) - Message-specific state management
//! - [`Channel`](crate::pregel::Channel) - Low-level channel primitives
//! - Python LangGraph State - <https://langchain-ai.github.io/langgraph/concepts/low_level/#state>

use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during state operations
///
/// State operations can fail for several reasons including validation failures,
/// reducer type mismatches, and malformed state structures.
///
/// # Examples
///
/// ## Invalid State Structure
///
/// ```rust
/// use langgraph_core::state::{StateSchema, StateError};
/// use serde_json::json;
///
/// let schema = StateSchema::new();
/// let invalid_state = json!("not an object");
///
/// match schema.validate(&invalid_state) {
///     Err(StateError::ValidationFailed(msg)) => {
///         println!("Validation failed: {}", msg);
///     }
///     _ => unreachable!(),
/// }
/// ```
///
/// ## Reducer Type Mismatch
///
/// ```rust
/// use langgraph_core::state::{AppendReducer, Reducer, StateError};
/// use serde_json::json;
///
/// let reducer = AppendReducer;
/// let result = reducer.reduce(&json!(42), &json!("not an array"));
///
/// assert!(matches!(result, Err(StateError::ReducerError(_))));
/// ```
#[derive(Debug, Error)]
pub enum StateError {
    /// State structure is invalid (e.g., not an object when expected)
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Schema validation failed
    #[error("Schema validation failed: {0}")]
    ValidationFailed(String),

    /// Reducer encountered incompatible types or failed to merge
    #[error("Reducer error: {0}")]
    ReducerError(String),

    /// Referenced field does not exist in schema
    #[error("Field not found: {0}")]
    FieldNotFound(String),
}

pub type Result<T> = std::result::Result<T, StateError>;

/// Trait for reducing/merging state values
///
/// Reducers define how multiple writes to the same state field are combined.
/// This is equivalent to Python's reducer functions in Annotated types.
pub trait Reducer: Send + Sync {
    /// Apply an update to the current value
    ///
    /// # Arguments
    ///
    /// * `current` - The current value (may be null)
    /// * `update` - The new value to merge
    ///
    /// # Returns
    ///
    /// The merged value
    fn reduce(&self, current: &Value, update: &Value) -> Result<Value>;

    /// Get a human-readable name for this reducer
    fn name(&self) -> &str;
}

/// Overwrite reducer - replaces the current value with the update
///
/// The simplest reducer that discards the current value and replaces it with
/// the new value. This is the default behavior when no reducer is specified.
///
/// **Equivalent to**: Python LangGraph's default (no `Annotated` reducer)
///
/// # Use Cases
///
/// - **Simple state fields**: Values that should always reflect the latest write
/// - **Status fields**: Current step, mode, or state of the system
/// - **Configuration**: Settings that should be replaced atomically
/// - **Scalar values**: Numbers, strings, booleans that have no merge semantics
///
/// # Examples
///
/// ## Basic Overwrite
///
/// ```rust
/// use langgraph_core::state::{OverwriteReducer, Reducer};
/// use serde_json::json;
///
/// let reducer = OverwriteReducer;
/// let current = json!({"step": "analyze"});
/// let update = json!({"step": "execute"});
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!({"step": "execute"}));
/// ```
///
/// ## In StateSchema
///
/// ```rust
/// use langgraph_core::state::{StateSchema, OverwriteReducer};
/// use serde_json::json;
///
/// let mut schema = StateSchema::new();
/// schema.add_field("status", Box::new(OverwriteReducer));
///
/// let mut state = json!({"status": "idle"});
/// schema.apply(&mut state, &json!({"status": "running"})).unwrap();
///
/// assert_eq!(state["status"], "running");
/// ```
///
/// # Performance
///
/// - **Time**: O(1) - simply clones the new value
/// - **Memory**: O(size of new value) - no accumulation
///
/// # See Also
///
/// - [`AppendReducer`] - For accumulating values
/// - [`MergeReducer`] - For combining objects
/// - [`SumReducer`] - For adding numbers
#[derive(Debug, Clone)]
pub struct OverwriteReducer;

impl Reducer for OverwriteReducer {
    fn reduce(&self, _current: &Value, update: &Value) -> Result<Value> {
        Ok(update.clone())
    }

    fn name(&self) -> &str {
        "overwrite"
    }
}

/// Append reducer - appends update to current array
///
/// Accumulates values by concatenating arrays. This is the most common reducer
/// for building up message history, event logs, or any list that grows over time.
///
/// **Equivalent to**: Python LangGraph's `Annotated[list, operator.add]`
///
/// # Behavior
///
/// - **Array + Array**: Concatenates both arrays
/// - **Array + Scalar**: Appends scalar as single element
/// - **Null + Array**: Initializes with the array
/// - **Null + Scalar**: Creates array with single element
///
/// # Use Cases
///
/// - **Message history**: Chat messages, tool results
/// - **Event logs**: Tracking all events in execution
/// - **Audit trails**: Recording all actions taken
/// - **Aggregating results**: Collecting outputs from parallel nodes
///
/// # Examples
///
/// ## Appending Message History
///
/// ```rust
/// use langgraph_core::state::{AppendReducer, Reducer};
/// use serde_json::json;
///
/// let reducer = AppendReducer;
/// let current = json!(["User: Hello"]);
/// let update = json!(["AI: Hi there!"]);
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!(["User: Hello", "AI: Hi there!"]));
/// ```
///
/// ## Initializing Empty State
///
/// ```rust
/// use langgraph_core::state::{AppendReducer, Reducer};
/// use serde_json::{json, Value};
///
/// let reducer = AppendReducer;
/// let current = Value::Null;
/// let update = json!(["First message"]);
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!(["First message"]));
/// ```
///
/// ## Appending Single Values
///
/// ```rust
/// use langgraph_core::state::{AppendReducer, Reducer};
/// use serde_json::json;
///
/// let reducer = AppendReducer;
/// let current = json!([1, 2, 3]);
/// let update = json!(4);  // Single value, not array
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!([1, 2, 3, 4]));
/// ```
///
/// ## In StateGraph
///
/// ```rust,ignore
/// use langgraph_core::{StateGraph, state::AppendReducer};
/// use serde_json::json;
///
/// let mut graph = StateGraph::new();
/// let mut schema = StateSchema::new();
/// schema.add_field("messages", Box::new(AppendReducer));
///
/// // Each node appends to messages
/// graph.add_node("node_a", |mut state| {
///     Box::pin(async move {
///         Ok(json!({"messages": ["Node A ran"]}))
///     })
/// });
/// ```
///
/// # Performance
///
/// - **Time**: O(n + m) where n and m are the sizes of current and update arrays
/// - **Memory**: O(n + m) for the combined array
/// - **Note**: Large arrays can accumulate memory over time
///
/// # See Also
///
/// - [`add_messages`](crate::messages::add_messages) - Smart message list merging
/// - [`OverwriteReducer`] - For replacing values
/// - [`MergeReducer`] - For combining objects
#[derive(Debug, Clone)]
pub struct AppendReducer;

impl Reducer for AppendReducer {
    fn reduce(&self, current: &Value, update: &Value) -> Result<Value> {
        match (current, update) {
            (Value::Array(curr_arr), Value::Array(upd_arr)) => {
                let mut result = curr_arr.clone();
                result.extend_from_slice(upd_arr);
                Ok(Value::Array(result))
            }
            (Value::Null, Value::Array(upd_arr)) => Ok(Value::Array(upd_arr.clone())),
            (Value::Array(curr_arr), single_value) => {
                let mut result = curr_arr.clone();
                result.push(single_value.clone());
                Ok(Value::Array(result))
            }
            (Value::Null, single_value) => Ok(Value::Array(vec![single_value.clone()])),
            _ => Err(StateError::ReducerError(
                "AppendReducer requires array values".to_string(),
            )),
        }
    }

    fn name(&self) -> &str {
        "append"
    }
}

/// Merge reducer - deep merges objects
///
/// Combines objects by merging their keys. When both current and update contain
/// the same key, the update's value overwrites the current value.
///
/// **Equivalent to**: Python LangGraph's dict merge (`{**current, **update}`)
///
/// # Behavior
///
/// - **Object + Object**: Merges keys, update values win on conflicts
/// - **Null + Object**: Initializes with the object
/// - **Other types**: Returns error
///
/// # Use Cases
///
/// - **Configuration objects**: Partial config updates
/// - **Feature flags**: Merging flag settings
/// - **Metadata**: Accumulating metadata fields
/// - **Partial state updates**: When nodes provide subset of fields
///
/// # Examples
///
/// ## Basic Object Merge
///
/// ```rust
/// use langgraph_core::state::{MergeReducer, Reducer};
/// use serde_json::json;
///
/// let reducer = MergeReducer;
/// let current = json!({"name": "Alice", "age": 30});
/// let update = json!({"age": 31, "city": "NYC"});
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!({
///     "name": "Alice",  // Preserved
///     "age": 31,        // Updated
///     "city": "NYC"     // Added
/// }));
/// ```
///
/// ## Configuration Merging
///
/// ```rust
/// use langgraph_core::state::{StateSchema, MergeReducer};
/// use serde_json::json;
///
/// let mut schema = StateSchema::new();
/// schema.add_field("config", Box::new(MergeReducer));
///
/// let mut state = json!({
///     "config": {
///         "api_key": "secret",
///         "timeout": 30,
///         "retries": 3
///     }
/// });
///
/// // Update only timeout
/// schema.apply(&mut state, &json!({
///     "config": {"timeout": 60}
/// })).unwrap();
///
/// assert_eq!(state["config"]["api_key"], "secret");  // Preserved
/// assert_eq!(state["config"]["timeout"], 60);         // Updated
/// assert_eq!(state["config"]["retries"], 3);          // Preserved
/// ```
///
/// ## Initializing from Null
///
/// ```rust
/// use langgraph_core::state::{MergeReducer, Reducer};
/// use serde_json::{json, Value};
///
/// let reducer = MergeReducer;
/// let current = Value::Null;
/// let update = json!({"key": "value"});
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!({"key": "value"}));
/// ```
///
/// # Performance
///
/// - **Time**: O(n) where n is the number of keys in the update
/// - **Memory**: O(total keys) for the merged object
/// - **Note**: Shallow merge only - nested objects are replaced, not recursively merged
///
/// # Limitations
///
/// This is a **shallow merge**. Nested objects are not recursively merged:
///
/// ```rust
/// use langgraph_core::state::{MergeReducer, Reducer};
/// use serde_json::json;
///
/// let reducer = MergeReducer;
/// let current = json!({"nested": {"a": 1, "b": 2}});
/// let update = json!({"nested": {"b": 3, "c": 4}});
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// // Nested object is replaced, not merged:
/// assert_eq!(result["nested"], json!({"b": 3, "c": 4}));
/// ```
///
/// # See Also
///
/// - [`OverwriteReducer`] - For replacing entire values
/// - [`AppendReducer`] - For concatenating arrays
#[derive(Debug, Clone)]
pub struct MergeReducer;

impl Reducer for MergeReducer {
    fn reduce(&self, current: &Value, update: &Value) -> Result<Value> {
        match (current, update) {
            (Value::Object(curr_obj), Value::Object(upd_obj)) => {
                let mut result = curr_obj.clone();
                for (key, value) in upd_obj {
                    result.insert(key.clone(), value.clone());
                }
                Ok(Value::Object(result))
            }
            (Value::Null, Value::Object(upd_obj)) => Ok(Value::Object(upd_obj.clone())),
            _ => Err(StateError::ReducerError(
                "MergeReducer requires object values".to_string(),
            )),
        }
    }

    fn name(&self) -> &str {
        "merge"
    }
}

/// Sum reducer - adds numeric values
///
/// Accumulates numeric values by addition. Supports both integers and floats,
/// maintaining type precision where possible.
///
/// **Equivalent to**: Python LangGraph's `Annotated[int, operator.add]` or `Annotated[float, operator.add]`
///
/// # Behavior
///
/// - **Int + Int**: Adds integers, returns integer
/// - **Float + Float**: Adds floats, returns float
/// - **Mixed Int/Float**: Adds as floats, returns float
/// - **Null + Number**: Initializes with the number
/// - **Non-numeric**: Returns error
///
/// # Use Cases
///
/// - **Counters**: Total actions, iterations, retries
/// - **Metrics**: Token counts, latency sums, cost tracking
/// - **Aggregations**: Summing results from parallel nodes
/// - **Accumulators**: Building up totals over time
///
/// # Examples
///
/// ## Integer Counter
///
/// ```rust
/// use langgraph_core::state::{SumReducer, Reducer};
/// use serde_json::json;
///
/// let reducer = SumReducer;
/// let current = json!(10);
/// let update = json!(5);
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!(15));
/// ```
///
/// ## Token Counting
///
/// ```rust
/// use langgraph_core::state::{StateSchema, SumReducer};
/// use serde_json::json;
///
/// let mut schema = StateSchema::new();
/// schema.add_field("total_tokens", Box::new(SumReducer));
///
/// let mut state = json!({"total_tokens": 0});
///
/// // Multiple nodes add tokens
/// schema.apply(&mut state, &json!({"total_tokens": 150})).unwrap();
/// schema.apply(&mut state, &json!({"total_tokens": 200})).unwrap();
/// schema.apply(&mut state, &json!({"total_tokens": 75})).unwrap();
///
/// assert_eq!(state["total_tokens"], 425);
/// ```
///
/// ## Float Aggregation
///
/// ```rust
/// use langgraph_core::state::{SumReducer, Reducer};
/// use serde_json::json;
///
/// let reducer = SumReducer;
/// let current = json!(2.5);
/// let update = json!(3.75);
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!(6.25));
/// ```
///
/// ## Initializing from Null
///
/// ```rust
/// use langgraph_core::state::{SumReducer, Reducer};
/// use serde_json::{json, Value};
///
/// let reducer = SumReducer;
/// let current = Value::Null;
/// let update = json!(100);
///
/// let result = reducer.reduce(&current, &update).unwrap();
/// assert_eq!(result, json!(100));
/// ```
///
/// ## Parallel Node Accumulation
///
/// ```rust,ignore
/// use langgraph_core::{StateGraph, state::{StateSchema, SumReducer}};
/// use serde_json::json;
///
/// let mut schema = StateSchema::new();
/// schema.add_field("cost", Box::new(SumReducer));
///
/// // Each parallel node adds its cost
/// graph.add_node("node_a", |state| {
///     Box::pin(async move { Ok(json!({"cost": 0.02})) })
/// });
///
/// graph.add_node("node_b", |state| {
///     Box::pin(async move { Ok(json!({"cost": 0.03})) })
/// });
///
/// // After both nodes run, state["cost"] = 0.05
/// ```
///
/// # Performance
///
/// - **Time**: O(1) - single addition operation
/// - **Memory**: O(1) - single numeric value
/// - **Precision**: Float operations may accumulate floating-point errors
///
/// # See Also
///
/// - [`AppendReducer`] - For collecting values in a list
/// - [`OverwriteReducer`] - For replacing values
#[derive(Debug, Clone)]
pub struct SumReducer;

impl Reducer for SumReducer {
    fn reduce(&self, current: &Value, update: &Value) -> Result<Value> {
        match (current, update) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(a_i64), Some(b_i64)) = (a.as_i64(), b.as_i64()) {
                    Ok(Value::Number((a_i64 + b_i64).into()))
                } else if let (Some(a_f64), Some(b_f64)) = (a.as_f64(), b.as_f64()) {
                    Ok(serde_json::Number::from_f64(a_f64 + b_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null))
                } else {
                    Err(StateError::ReducerError(
                        "Cannot add non-numeric values".to_string(),
                    ))
                }
            }
            (Value::Null, Value::Number(_)) => Ok(update.clone()),
            _ => Err(StateError::ReducerError(
                "SumReducer requires numeric values".to_string(),
            )),
        }
    }

    fn name(&self) -> &str {
        "sum"
    }
}

/// State schema defining fields and their reducers
///
/// Equivalent to Python's StateGraph with Annotated type hints
#[derive(Default)]
pub struct StateSchema {
    /// Map of field name to reducer
    fields: HashMap<String, Box<dyn Reducer>>,

    /// Default reducer for fields not explicitly defined
    default_reducer: Option<Box<dyn Reducer>>,
}

impl StateSchema {
    /// Create a new empty state schema
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field with a specific reducer
    ///
    /// # Arguments
    ///
    /// * `field_name` - Name of the state field
    /// * `reducer` - Reducer to use for this field
    pub fn add_field(&mut self, field_name: impl Into<String>, reducer: Box<dyn Reducer>) {
        self.fields.insert(field_name.into(), reducer);
    }

    /// Set the default reducer for fields not explicitly defined
    pub fn with_default_reducer(mut self, reducer: Box<dyn Reducer>) -> Self {
        self.default_reducer = Some(reducer);
        self
    }

    /// Get the reducer for a field (or default)
    fn get_reducer(&self, field_name: &str) -> Option<&dyn Reducer> {
        self.fields
            .get(field_name)
            .map(|r| r.as_ref())
            .or_else(|| self.default_reducer.as_ref().map(|r| r.as_ref()))
    }

    /// Apply an update to state according to schema reducers
    ///
    /// # Arguments
    ///
    /// * `state` - Current state (will be modified in place)
    /// * `update` - Update to apply
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn apply(&self, state: &mut Value, update: &Value) -> Result<()> {
        // Both state and update must be objects
        let state_obj = state
            .as_object_mut()
            .ok_or_else(|| StateError::InvalidState("State must be an object".to_string()))?;

        let update_obj = update
            .as_object()
            .ok_or_else(|| StateError::InvalidState("Update must be an object".to_string()))?;

        // Apply each field from update
        for (field_name, update_value) in update_obj {
            let current_value = state_obj
                .get(field_name)
                .cloned()
                .unwrap_or(Value::Null);

            // Get reducer for this field
            let reduced_value = if let Some(reducer) = self.get_reducer(field_name) {
                reducer.reduce(&current_value, update_value)?
            } else {
                // No reducer defined - use overwrite by default
                update_value.clone()
            };

            state_obj.insert(field_name.clone(), reduced_value);
        }

        Ok(())
    }

    /// Validate that state conforms to schema
    ///
    /// # Arguments
    ///
    /// * `state` - State to validate
    ///
    /// # Returns
    ///
    /// Ok(()) if valid, Err otherwise
    pub fn validate(&self, state: &Value) -> Result<()> {
        if !state.is_object() {
            return Err(StateError::ValidationFailed(
                "State must be an object".to_string(),
            ));
        }

        // For now, just check it's an object
        // More sophisticated validation can be added later
        Ok(())
    }

    /// Get the list of fields in this schema
    pub fn fields(&self) -> Vec<String> {
        self.fields.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_overwrite_reducer() {
        let reducer = OverwriteReducer;
        let current = json!({"old": "value"});
        let update = json!({"new": "value"});

        let result = reducer.reduce(&current, &update).unwrap();
        assert_eq!(result, json!({"new": "value"}));
    }

    #[test]
    fn test_append_reducer_arrays() {
        let reducer = AppendReducer;
        let current = json!([1, 2, 3]);
        let update = json!([4, 5]);

        let result = reducer.reduce(&current, &update).unwrap();
        assert_eq!(result, json!([1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_append_reducer_null_current() {
        let reducer = AppendReducer;
        let current = Value::Null;
        let update = json!([1, 2]);

        let result = reducer.reduce(&current, &update).unwrap();
        assert_eq!(result, json!([1, 2]));
    }

    #[test]
    fn test_append_reducer_single_value() {
        let reducer = AppendReducer;
        let current = json!([1, 2]);
        let update = json!(3);

        let result = reducer.reduce(&current, &update).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_merge_reducer() {
        let reducer = MergeReducer;
        let current = json!({"a": 1, "b": 2});
        let update = json!({"b": 3, "c": 4});

        let result = reducer.reduce(&current, &update).unwrap();
        assert_eq!(result, json!({"a": 1, "b": 3, "c": 4}));
    }

    #[test]
    fn test_sum_reducer_integers() {
        let reducer = SumReducer;
        let current = json!(5);
        let update = json!(3);

        let result = reducer.reduce(&current, &update).unwrap();
        assert_eq!(result, json!(8));
    }

    #[test]
    fn test_sum_reducer_floats() {
        let reducer = SumReducer;
        let current = json!(2.5);
        let update = json!(3.5);

        let result = reducer.reduce(&current, &update).unwrap();
        assert_eq!(result, json!(6.0));
    }

    #[test]
    fn test_state_schema_apply() {
        let mut schema = StateSchema::new();
        schema.add_field("messages", Box::new(AppendReducer));
        schema.add_field("count", Box::new(SumReducer));

        let mut state = json!({
            "messages": ["hello"],
            "count": 1
        });

        let update = json!({
            "messages": ["world"],
            "count": 2
        });

        schema.apply(&mut state, &update).unwrap();

        assert_eq!(state["messages"], json!(["hello", "world"]));
        assert_eq!(state["count"], json!(3));
    }

    #[test]
    fn test_state_schema_default_reducer() {
        let schema = StateSchema::new().with_default_reducer(Box::new(OverwriteReducer));

        let mut state = json!({"field": "old"});
        let update = json!({"field": "new", "other": "value"});

        schema.apply(&mut state, &update).unwrap();

        assert_eq!(state["field"], json!("new"));
        assert_eq!(state["other"], json!("value"));
    }

    #[test]
    fn test_state_schema_validation() {
        let schema = StateSchema::new();

        // Valid state
        let valid_state = json!({"field": "value"});
        assert!(schema.validate(&valid_state).is_ok());

        // Invalid state (not an object)
        let invalid_state = json!("not an object");
        assert!(schema.validate(&invalid_state).is_err());
    }

    #[test]
    fn test_state_schema_fields() {
        let mut schema = StateSchema::new();
        schema.add_field("field1", Box::new(OverwriteReducer));
        schema.add_field("field2", Box::new(AppendReducer));

        let fields = schema.fields();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains(&"field1".to_string()));
        assert!(fields.contains(&"field2".to_string()));
    }

    #[test]
    fn test_reducer_names() {
        assert_eq!(OverwriteReducer.name(), "overwrite");
        assert_eq!(AppendReducer.name(), "append");
        assert_eq!(MergeReducer.name(), "merge");
        assert_eq!(SumReducer.name(), "sum");
    }
}
