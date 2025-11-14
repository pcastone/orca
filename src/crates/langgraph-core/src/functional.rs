//! Functional API for building graphs with a builder pattern
//!
//! This module provides a more ergonomic, functional approach to building graphs
//! as an alternative to the imperative StateGraph API. It's inspired by Python
//! LangGraph's @task and @entrypoint decorators but adapted to Rust's patterns.
//!
//! # Example
//!
//! ```rust,no_run
//! use langgraph_core::functional::{Workflow, task};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Define tasks
//!     let add_one = task("add_one", |mut state| Box::pin(async move {
//!         if let Some(obj) = state.as_object_mut() {
//!             let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
//!             obj.insert("value".to_string(), json!(val + 1));
//!         }
//!         Ok(state)
//!     }));
//!
//!     let multiply_two = task("multiply_two", |mut state| Box::pin(async move {
//!         if let Some(obj) = state.as_object_mut() {
//!             let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
//!             obj.insert("value".to_string(), json!(val * 2));
//!         }
//!         Ok(state)
//!     }));
//!
//!     // Create workflow
//!     let workflow = Workflow::builder()
//!         .add_task(add_one)
//!         .then(multiply_two)
//!         .build()?;
//!
//!     // Execute
//!     let result = workflow.invoke(json!({"value": 5})).await?;
//!     // (5 + 1) * 2 = 12
//!     
//!     Ok(())
//! }
//! ```

use crate::{StateGraph, CompiledGraph, Result as GraphResult};
use crate::error::GraphError;
use crate::retry::RetryPolicy;
use serde_json::Value;
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;

/// Type alias for task executor functions
pub type TaskFn = Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = GraphResult<Value>> + Send>> + Send + Sync>;

/// A task represents a single unit of work in a workflow
///
/// Tasks are the building blocks of functional workflows. Each task:
/// - Has a unique name for identification
/// - Executes an async function
/// - Can have retry policies
/// - Can be cached (future feature)
///
/// Tasks are composable and can be chained together to form workflows.
#[derive(Clone)]
pub struct Task {
    /// Task name (must be unique within a workflow)
    pub name: String,
    
    /// The async function that executes this task
    pub executor: TaskFn,
    
    /// Optional retry policy for this task
    pub retry_policy: Option<Vec<RetryPolicy>>,
    
    /// Whether to cache results (future feature)
    pub cache: bool,
}

impl Task {
    /// Create a new task with a name and executor function
    ///
    /// # Arguments
    ///
    /// * `name` - Unique name for this task
    /// * `executor` - Async function that processes state
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use langgraph_core::functional::Task;
    /// use serde_json::json;
    ///
    /// let task = Task::new("my_task", |state| {
    ///     Box::pin(async move {
    ///         // Process state
    ///         Ok(state)
    ///     })
    /// });
    /// ```
    pub fn new<F>(name: impl Into<String>, executor: F) -> Self
    where
        F: Fn(Value) -> Pin<Box<dyn Future<Output = GraphResult<Value>> + Send>> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            executor: Arc::new(executor),
            retry_policy: None,
            cache: false,
        }
    }

    /// Set retry policy for this task
    pub fn with_retry(mut self, policies: Vec<RetryPolicy>) -> Self {
        self.retry_policy = Some(policies);
        self
    }

    /// Enable caching for this task (placeholder for future implementation)
    pub fn with_cache(mut self) -> Self {
        self.cache = true;
        self
    }
}

/// Convenience function to create a task
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::functional::task;
///
/// let my_task = task("process", |state| Box::pin(async move {
///     Ok(state)
/// }));
/// ```
pub fn task<F>(name: impl Into<String>, executor: F) -> Task
where
    F: Fn(Value) -> Pin<Box<dyn Future<Output = GraphResult<Value>> + Send>> + Send + Sync + 'static,
{
    Task::new(name, executor)
}

/// Builder for creating functional workflows
///
/// This builder provides a fluent API for composing tasks into workflows.
/// Tasks are executed sequentially in the order they are added.
///
/// # Example
///
/// ```rust,no_run
/// use langgraph_core::functional::{WorkflowBuilder, task};
/// use serde_json::json;
///
/// let workflow = WorkflowBuilder::new()
///     .add_task(task("step1", |state| Box::pin(async move { Ok(state) })))
///     .add_task(task("step2", |state| Box::pin(async move { Ok(state) })))
///     .build()
///     .unwrap();
/// ```
pub struct WorkflowBuilder {
    tasks: Vec<Task>,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
        }
    }

    /// Add a task to the workflow
    pub fn add_task(mut self, task: Task) -> Self {
        self.tasks.push(task);
        self
    }

    /// Add a task (alias for add_task for more fluent API)
    pub fn then(self, task: Task) -> Self {
        self.add_task(task)
    }

    /// Build the workflow into a compiled graph
    ///
    /// This converts the functional workflow into a StateGraph and compiles it.
    pub fn build(self) -> GraphResult<Workflow> {
        if self.tasks.is_empty() {
            return Err(GraphError::Configuration("Workflow must have at least one task".to_string()));
        }

        let mut graph = StateGraph::new();
        
        // Add all tasks as nodes
        for task in &self.tasks {
            let executor = task.executor.clone();
            graph.add_node(&task.name, move |state| {
                let exec = executor.clone();
                Box::pin(async move {
                    exec(state).await
                })
            });
        }

        // Chain tasks sequentially
        graph.add_edge("__start__", &self.tasks[0].name);
        for i in 0..self.tasks.len() - 1 {
            graph.add_edge(&self.tasks[i].name, &self.tasks[i + 1].name);
        }
        graph.add_edge(&self.tasks[self.tasks.len() - 1].name, "__end__");

        let compiled = graph.compile()?;
        
        Ok(Workflow {
            tasks: self.tasks,
            compiled,
        })
    }
}

impl Default for WorkflowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A compiled functional workflow ready for execution
///
/// Workflows are the result of building a WorkflowBuilder. They contain
/// the sequence of tasks and a compiled graph ready for execution.
pub struct Workflow {
    /// The tasks in this workflow
    tasks: Vec<Task>,
    
    /// The compiled graph
    compiled: CompiledGraph,
}

impl Workflow {
    /// Create a new workflow builder
    pub fn builder() -> WorkflowBuilder {
        WorkflowBuilder::new()
    }

    /// Execute the workflow with the given input
    ///
    /// # Arguments
    ///
    /// * `input` - Initial state value
    ///
    /// # Returns
    ///
    /// Final state after all tasks have executed
    pub async fn invoke(&self, input: Value) -> GraphResult<Value> {
        self.compiled.invoke(input).await
    }

    /// Get the task names in this workflow
    pub fn task_names(&self) -> Vec<&str> {
        self.tasks.iter().map(|t| t.name.as_str()).collect()
    }

    /// Visualize the workflow
    pub fn visualize(&self, options: &crate::visualization::VisualizationOptions) -> String {
        self.compiled.visualize(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_simple_workflow() {
        let add_one = task("add_one", |mut state| Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("value".to_string(), json!(val + 1));
            }
            Ok(state)
        }));

        let multiply_two = task("multiply_two", |mut state| Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let val = obj.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("value".to_string(), json!(val * 2));
            }
            Ok(state)
        }));

        let workflow = Workflow::builder()
            .add_task(add_one)
            .then(multiply_two)
            .build()
            .unwrap();

        let result = workflow.invoke(json!({"value": 5})).await.unwrap();
        
        // (5 + 1) * 2 = 12
        assert_eq!(result["value"], 12);
    }

    #[tokio::test]
    async fn test_workflow_task_names() {
        let task1 = task("task1", |state| Box::pin(async move { Ok(state) }));
        let task2 = task("task2", |state| Box::pin(async move { Ok(state) }));

        let workflow = Workflow::builder()
            .add_task(task1)
            .add_task(task2)
            .build()
            .unwrap();

        let names = workflow.task_names();
        assert_eq!(names, vec!["task1", "task2"]);
    }

    #[tokio::test]
    async fn test_empty_workflow_fails() {
        let result = WorkflowBuilder::new().build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_workflow_with_three_tasks() {
        let t1 = task("double", |mut state| Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let val = obj.get("n").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("n".to_string(), json!(val * 2));
            }
            Ok(state)
        }));

        let t2 = task("add_ten", |mut state| Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let val = obj.get("n").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("n".to_string(), json!(val + 10));
            }
            Ok(state)
        }));

        let t3 = task("subtract_five", |mut state| Box::pin(async move {
            if let Some(obj) = state.as_object_mut() {
                let val = obj.get("n").and_then(|v| v.as_i64()).unwrap_or(0);
                obj.insert("n".to_string(), json!(val - 5));
            }
            Ok(state)
        }));

        let workflow = Workflow::builder()
            .then(t1)
            .then(t2)
            .then(t3)
            .build()
            .unwrap();

        // (3 * 2) + 10 - 5 = 11
        let result = workflow.invoke(json!({"n": 3})).await.unwrap();
        assert_eq!(result["n"], 11);
    }
}
