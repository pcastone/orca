//! Core Pregel data types.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use crate::error::Result;

/// A segment of a task path, used for tracking task execution hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PathSegment {
    /// String identifier (e.g., node name)
    String(String),
    /// Integer index (e.g., for Send() tasks)
    Int(usize),
    /// Nested tuple for hierarchical tasks
    Tuple(Vec<PathSegment>),
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSegment::String(s) => write!(f, "{}", s),
            PathSegment::Int(i) => write!(f, "{}", i),
            PathSegment::Tuple(t) => {
                write!(f, "(")?;
                for (i, seg) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", seg)?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Task state - either config or snapshot.
#[derive(Debug, Clone)]
pub enum TaskState {
    /// Runnable configuration
    Config(serde_json::Value),
    /// State snapshot
    Snapshot(serde_json::Value),
}

/// An interrupt that occurred during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interrupt {
    /// The value associated with the interrupt
    pub value: serde_json::Value,
    /// Unique interrupt ID for resumption
    pub id: String,
}

impl Interrupt {
    /// Create a new interrupt with a value.
    pub fn new(value: serde_json::Value) -> Self {
        Self {
            value,
            id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create an interrupt with a specific ID.
    pub fn with_id(value: serde_json::Value, id: String) -> Self {
        Self { value, id }
    }
}

/// A Pregel task - represents a unit of work to be executed.
#[derive(Clone)]
pub struct PregelTask {
    /// Unique task identifier
    pub id: String,
    /// Node name to execute
    pub name: String,
    /// Task path for tracking hierarchy
    pub path: Vec<PathSegment>,
    /// Error if task failed
    pub error: Option<String>,  // Simplified from Box<dyn Error> for Clone
    /// Interrupts raised during execution
    pub interrupts: Vec<Interrupt>,
    /// Task state
    pub state: Option<TaskState>,
    /// Task result
    pub result: Option<serde_json::Value>,
}

impl fmt::Debug for PregelTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PregelTask")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("path", &self.path)
            .field("error", &self.error)
            .field("interrupts", &self.interrupts)
            .field("result", &self.result)
            .finish()
    }
}

/// Retry policy for task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Initial interval between retries in seconds
    pub initial_interval: f64,
    /// Backoff multiplier for each retry
    pub backoff_factor: f64,
    /// Maximum interval between retries in seconds
    pub max_interval: f64,
    /// Maximum number of attempts (including first attempt)
    pub max_attempts: usize,
    /// Whether to add random jitter to intervals
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_interval: 0.5,
            backoff_factor: 2.0,
            max_interval: 128.0,
            max_attempts: 3,
            jitter: true,
        }
    }
}

/// Cache key for a task result.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Namespace for the cache entry
    pub ns: Vec<String>,
    /// Key for the cache entry
    pub key: String,
    /// Time to live in seconds (None = never expires)
    pub ttl: Option<u64>,
}

/// Cache policy for task results.
#[derive(Clone)]
pub struct CachePolicy {
    /// Function to generate cache key from input
    pub key_func: Arc<dyn Fn(&serde_json::Value) -> String + Send + Sync>,
    /// Time to live in seconds
    pub ttl: Option<u64>,
}

impl fmt::Debug for CachePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachePolicy")
            .field("key_func", &"<function>")
            .field("ttl", &self.ttl)
            .finish()
    }
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            key_func: Arc::new(|input| {
                // Simple hash-based key
                use std::hash::{Hash, Hasher};
                use std::collections::hash_map::DefaultHasher;
                let mut hasher = DefaultHasher::new();
                input.to_string().hash(&mut hasher);
                format!("{:x}", hasher.finish())
            }),
            ttl: None,
        }
    }
}

/// Node executor trait - defines how to execute a node.
pub trait NodeExecutor: Send + Sync {
    /// Execute the node with the given input.
    fn execute(
        &self,
        input: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send + '_>>;
}

/// Writer trait - for additional write operations.
pub trait Writer: Send + Sync {
    /// Write a value.
    fn write(
        &self,
        value: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

/// A task ready for execution.
pub struct PregelExecutableTask {
    /// Node name
    pub name: String,
    /// Input to the node
    pub input: serde_json::Value,
    /// The executor to run
    pub proc: Arc<dyn NodeExecutor>,
    /// Pending writes from this task
    pub writes: VecDeque<(String, serde_json::Value)>,
    /// Execution configuration
    pub config: serde_json::Value,
    /// Channel names that triggered this task
    pub triggers: Vec<String>,
    /// Channel names to write output to (e.g., vec!["state"] for StateGraph)
    pub write_channels: Vec<String>,
    /// Retry policy for this task
    pub retry_policy: Vec<RetryPolicy>,
    /// Cache key if caching is enabled
    pub cache_key: Option<CacheKey>,
    /// Task ID
    pub id: String,
    /// Task path
    pub path: Vec<PathSegment>,
    /// Additional writers
    pub writers: Vec<Arc<dyn Writer>>,
}

impl fmt::Debug for PregelExecutableTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PregelExecutableTask")
            .field("name", &self.name)
            .field("input", &self.input)
            .field("writes", &self.writes)
            .field("triggers", &self.triggers)
            .field("write_channels", &self.write_channels)
            .field("id", &self.id)
            .field("path", &self.path)
            .finish()
    }
}

/// Protocol for objects containing writes.
pub trait WritesProtocol {
    /// Get the task path
    fn path(&self) -> &[PathSegment];
    /// Get the task name
    fn name(&self) -> &str;
    /// Get the writes
    fn writes(&self) -> &[(String, serde_json::Value)];
    /// Get the triggers
    fn triggers(&self) -> &[String];
}

/// Simple implementation of WritesProtocol for non-task writes (e.g., graph input).
#[derive(Debug, Clone)]
pub struct PregelTaskWrites {
    pub path: Vec<PathSegment>,
    pub name: String,
    pub writes: Vec<(String, serde_json::Value)>,
    pub triggers: Vec<String>,
}

impl WritesProtocol for PregelTaskWrites {
    fn path(&self) -> &[PathSegment] {
        &self.path
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn writes(&self) -> &[(String, serde_json::Value)] {
        &self.writes
    }

    fn triggers(&self) -> &[String] {
        &self.triggers
    }
}

impl WritesProtocol for PregelExecutableTask {
    fn path(&self) -> &[PathSegment] {
        &self.path
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn writes(&self) -> &[(String, serde_json::Value)] {
        // Convert VecDeque to slice via temporary Vec
        // In real implementation, we'd store writes as Vec
        &[]  // Placeholder - needs refactoring
    }

    fn triggers(&self) -> &[String] {
        &self.triggers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_segment_display() {
        let seg = PathSegment::String("node1".into());
        assert_eq!(seg.to_string(), "node1");

        let seg = PathSegment::Int(42);
        assert_eq!(seg.to_string(), "42");

        let seg = PathSegment::Tuple(vec![
            PathSegment::String("a".into()),
            PathSegment::Int(1),
        ]);
        assert_eq!(seg.to_string(), "(a, 1)");
    }

    #[test]
    fn test_interrupt_creation() {
        let int = Interrupt::new(serde_json::json!({"reason": "user_input"}));
        assert!(int.id.len() > 0);
        assert_eq!(int.value["reason"], "user_input");
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.initial_interval, 0.5);
        assert_eq!(policy.backoff_factor, 2.0);
        assert_eq!(policy.max_attempts, 3);
    }

    #[test]
    fn test_cache_key() {
        let key = CacheKey {
            ns: vec!["graph1".into(), "node1".into()],
            key: "abc123".into(),
            ttl: Some(3600),
        };
        assert_eq!(key.ns.len(), 2);
        assert_eq!(key.ttl, Some(3600));
    }
}
