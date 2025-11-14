//! Store and Cache - Key-value storage for graph state management
//!
//! This module provides two complementary storage abstractions:
//! - **[`Store`]** - Persistent key-value storage for sharing data across graph invocations
//! - **[`Cache`]** - Temporary storage with TTL support for caching computation results
//!
//! Both traits provide async, thread-safe storage backends that can be implemented
//! using any underlying technology (in-memory, Redis, database, etc.).
//!
//! # Overview
//!
//! **Store** is designed for persistent data that needs to be shared across different
//! graph executions or persist beyond a single workflow:
//! - User preferences and settings
//! - Conversation history and context
//! - Application configuration
//! - Cross-session data sharing
//!
//! **Cache** is designed for temporary data with automatic expiration:
//! - LLM response caching with TTL
//! - Rate limiting buckets
//! - Session tokens with expiration
//! - Computation result memoization
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Graph Execution                                                 │
//! │  • Nodes read/write to Store for persistent data                │
//! │  • Nodes use Cache for temporary/expensive computations         │
//! └────────────┬─────────────────────────┬──────────────────────────┘
//!              │                         │
//!              ↓                         ↓
//! ┌────────────────────────┐  ┌────────────────────────┐
//! │  Store Trait           │  │  Cache Trait           │
//! │  • get(key)            │  │  • get(key)            │
//! │  • put(key, value)     │  │  • put(key, value, ttl)│
//! │  • delete(key)         │  │  • delete(key)         │
//! │  • list_keys(prefix)   │  │  • clear_expired()     │
//! │  • clear(prefix)       │  │  • clear_all()         │
//! └────────┬───────────────┘  └────────┬───────────────┘
//!          │                           │
//!          ↓                           ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Implementations                                                 │
//! │  • InMemoryStore    - Development/testing (no persistence)      │
//! │  • InMemoryCache    - Development/testing (lazy expiration)     │
//! │  • RedisStore       - Production (external, persistent)         │
//! │  • RedisCache       - Production (built-in TTL support)         │
//! │  • DatabaseStore    - Production (SQL/NoSQL backends)           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Using Store for Persistent Data
//!
//! ```rust,ignore
//! use langgraph_core::store::{Store, InMemoryStore};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let store = InMemoryStore::new();
//!
//!     // Store user preferences
//!     store.put("user:123:prefs", json!({
//!         "theme": "dark",
//!         "notifications": true
//!     })).await?;
//!
//!     // Retrieve preferences later
//!     let prefs = store.get("user:123:prefs").await?;
//!     if let Some(p) = prefs {
//!         println!("Theme: {}", p["theme"]);
//!     }
//!
//!     // List all user keys
//!     let user_keys = store.list_keys(Some("user:123:")).await?;
//!     println!("User has {} stored items", user_keys.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Using Cache for Temporary Data
//!
//! ```rust,ignore
//! use langgraph_core::store::{Cache, InMemoryCache};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let cache = InMemoryCache::new();
//!
//!     // Cache LLM response for 1 hour (3600 seconds)
//!     let prompt_hash = "abc123";
//!     cache.put(
//!         &format!("llm:{}", prompt_hash),
//!         json!({"response": "Hello, world!"}),
//!         Some(3600)
//!     ).await?;
//!
//!     // Check if cached (within TTL)
//!     if let Some(cached) = cache.get(&format!("llm:{}", prompt_hash)).await? {
//!         println!("Cache hit: {}", cached["response"]);
//!     } else {
//!         println!("Cache miss - call LLM");
//!     }
//!
//!     // Clean up expired entries
//!     let removed = cache.clear_expired().await?;
//!     println!("Removed {} expired entries", removed);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Cross-Session Data Sharing
//!
//! Store persistent data that multiple graph invocations need to access:
//!
//! ```rust,ignore
//! use langgraph_core::store::Store;
//! use serde_json::json;
//!
//! async fn store_conversation_context(
//!     store: &impl Store,
//!     session_id: &str,
//!     context: &str
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     // Append to conversation history
//!     let key = format!("conversation:{}", session_id);
//!
//!     let mut history = match store.get(&key).await? {
//!         Some(h) => serde_json::from_value(h)?,
//!         None => Vec::new()
//!     };
//!
//!     history.push(context.to_string());
//!     store.put(&key, json!(history)).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Pattern 2: LLM Response Caching
//!
//! Cache expensive LLM calls with automatic expiration:
//!
//! ```rust,ignore
//! use langgraph_core::store::Cache;
//! use serde_json::{json, Value};
//!
//! async fn get_or_compute_llm_response(
//!     cache: &impl Cache,
//!     prompt: &str,
//!     ttl_seconds: u64
//! ) -> Result<Value, Box<dyn std::error::Error>> {
//!     let cache_key = format!("llm:{}", hash(prompt));
//!
//!     // Check cache first
//!     if let Some(cached) = cache.get(&cache_key).await? {
//!         return Ok(cached);
//!     }
//!
//!     // Cache miss - call LLM
//!     let response = call_llm(prompt).await?;
//!
//!     // Store with TTL (e.g., 1 hour)
//!     cache.put(&cache_key, response.clone(), Some(ttl_seconds)).await?;
//!
//!     Ok(response)
//! }
//! ```
//!
//! ## Pattern 3: Rate Limiting with Cache
//!
//! Implement rate limiting using TTL-based counters:
//!
//! ```rust,ignore
//! use langgraph_core::store::Cache;
//! use serde_json::json;
//!
//! async fn check_rate_limit(
//!     cache: &impl Cache,
//!     user_id: &str,
//!     limit: u64,
//!     window_secs: u64
//! ) -> Result<bool, Box<dyn std::error::Error>> {
//!     let key = format!("ratelimit:{}:{}", user_id, current_window());
//!
//!     let count = match cache.get(&key).await? {
//!         Some(v) => v.as_u64().unwrap_or(0),
//!         None => 0
//!     };
//!
//!     if count >= limit {
//!         return Ok(false); // Rate limited
//!     }
//!
//!     // Increment counter with TTL
//!     cache.put(&key, json!(count + 1), Some(window_secs)).await?;
//!     Ok(true)
//! }
//! ```
//!
//! ## Pattern 4: Session Management
//!
//! Store session tokens with automatic expiration:
//!
//! ```rust,ignore
//! use langgraph_core::store::Cache;
//! use serde_json::json;
//!
//! async fn create_session(
//!     cache: &impl Cache,
//!     user_id: &str,
//!     session_duration_secs: u64
//! ) -> Result<String, Box<dyn std::error::Error>> {
//!     let session_id = generate_session_token();
//!
//!     // Store session with TTL
//!     cache.put(
//!         &format!("session:{}", session_id),
//!         json!({"user_id": user_id, "created_at": now()}),
//!         Some(session_duration_secs)
//!     ).await?;
//!
//!     Ok(session_id)
//! }
//!
//! async fn validate_session(
//!     cache: &impl Cache,
//!     session_id: &str
//! ) -> Result<Option<String>, Box<dyn std::error::Error>> {
//!     let key = format!("session:{}", session_id);
//!
//!     if let Some(session) = cache.get(&key).await? {
//!         return Ok(Some(session["user_id"].as_str().unwrap().to_string()));
//!     }
//!
//!     Ok(None) // Session expired or invalid
//! }
//! ```
//!
//! ## Pattern 5: Multi-Tenant Data Isolation
//!
//! Use key prefixes to isolate tenant data:
//!
//! ```rust,ignore
//! use langgraph_core::store::Store;
//! use serde_json::json;
//!
//! async fn store_tenant_data(
//!     store: &impl Store,
//!     tenant_id: &str,
//!     key: &str,
//!     value: serde_json::Value
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     let namespaced_key = format!("tenant:{}:{}", tenant_id, key);
//!     store.put(&namespaced_key, value).await?;
//!     Ok(())
//! }
//!
//! async fn list_tenant_data(
//!     store: &impl Store,
//!     tenant_id: &str
//! ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
//!     let prefix = format!("tenant:{}:", tenant_id);
//!     let keys = store.list_keys(Some(&prefix)).await?;
//!     Ok(keys)
//! }
//!
//! async fn delete_tenant_data(
//!     store: &impl Store,
//!     tenant_id: &str
//! ) -> Result<usize, Box<dyn std::error::Error>> {
//!     let prefix = format!("tenant:{}:", tenant_id);
//!     let deleted = store.clear(Some(&prefix)).await?;
//!     Ok(deleted)
//! }
//! ```
//!
//! # When to Use Store vs Cache
//!
//! | Use Store When | Use Cache When |
//! |----------------|----------------|
//! | Data must persist across restarts | Data can be recomputed if lost |
//! | Data is infrequently updated | Data changes frequently |
//! | Data size is small to moderate | Data should expire automatically |
//! | Need to list all keys with prefix | Need TTL-based expiration |
//! | Implementing user preferences | Implementing rate limiting |
//! | Storing conversation history | Caching LLM responses |
//! | Managing application configuration | Managing session tokens |
//!
//! # Performance Considerations
//!
//! ## Store Performance
//!
//! **InMemoryStore:**
//! - **Get**: O(1) - HashMap lookup
//! - **Put**: O(1) - HashMap insert
//! - **Delete**: O(1) - HashMap remove
//! - **List**: O(n) - Scans all keys (use prefixes to reduce n)
//! - **Clear**: O(n) - Removes matching keys
//! - **Concurrency**: RwLock (multiple readers, single writer)
//!
//! **External Store Implementations (Redis, Database):**
//! - Network latency dominates (1-10ms per operation)
//! - Use pipelining for batch operations
//! - Implement connection pooling (5-20 connections)
//! - Consider read replicas for high read volume
//!
//! ## Cache Performance
//!
//! **InMemoryCache:**
//! - **Get**: O(1) + expiration check (lazy cleanup)
//! - **Put**: O(1) + TTL calculation
//! - **Expiration**: Lazy (checked on access, not actively cleaned)
//! - **Memory**: Grows until clear_expired() is called
//! - **Best Practice**: Run periodic clear_expired() in background task
//!
//! **Redis Cache:**
//! - Native TTL support (active expiration)
//! - Automatic memory management
//! - Better for production use
//!
//! ## Best Practices for Performance
//!
//! 1. **Use Prefixes Strategically**: Keep prefix hierarchies shallow
//!    ```rust,ignore
//!    // Good: tenant:123:user:456
//!    // Bad:  tenant:123:region:us:datacenter:east:user:456
//!    ```
//!
//! 2. **Batch Operations**: Group related operations
//!    ```rust,ignore
//!    // Good: Use single transaction/pipeline for related puts
//!    for (key, value) in batch {
//!        store.put(key, value).await?; // Consider batching API
//!    }
//!    ```
//!
//! 3. **Cache Warming**: Pre-populate cache at startup
//!    ```rust,ignore
//!    async fn warm_cache(cache: &impl Cache) {
//!        for common_query in get_common_queries() {
//!             cache.put(&query.key, query.result, Some(3600)).await?;
//!        }
//!    }
//!    ```
//!
//! 4. **TTL Selection**: Balance freshness vs cache hit rate
//!    ```rust,ignore
//!    // Frequently changing data: 60-300 seconds
//!    cache.put(key, value, Some(60)).await?;
//!
//!    // Stable data: 3600-86400 seconds (1-24 hours)
//!    cache.put(key, value, Some(3600)).await?;
//!    ```
//!
//! 5. **Periodic Cleanup**: For InMemoryCache, run background cleanup
//!    ```rust,ignore
//!    tokio::spawn(async move {
//!        loop {
//!            tokio::time::sleep(Duration::from_secs(300)).await;
//!            let _ = cache.clear_expired().await;
//!        }
//!    });
//!    ```
//!
//! # Python LangGraph Comparison
//!
//! | Concept | Python LangGraph | rLangGraph (Rust) |
//! |---------|------------------|-------------------|
//! | Store trait | `BaseStore` abstract class | `Store` trait with async methods |
//! | In-memory store | `InMemoryStore` class | `InMemoryStore` struct |
//! | Redis integration | `langgraph-checkpoint-redis` | Implement `Store` trait for Redis |
//! | TTL support | Via backend-specific features | `Cache` trait with explicit TTL |
//! | Async API | Sync by default, async optional | Async-first with `async_trait` |
//! | Type safety | Dynamic typing (Any) | Strong typing with `serde_json::Value` |
//! | Concurrency | GIL limitations | Lock-free or RwLock with true parallelism |
//!
//! **Python Example:**
//! ```python
//! store = InMemoryStore()
//! store.put("key", {"value": 42})
//! result = store.get("key")
//! ```
//!
//! **Rust Equivalent:**
//! ```rust,ignore
//! let store = InMemoryStore::new();
//! store.put("key", json!({"value": 42})).await?;
//! let result = store.get("key").await?;
//! ```
//!
//! # Implementing Custom Backends
//!
//! Both `Store` and `Cache` traits are designed for extensibility:
//!
//! ```rust,ignore
//! use async_trait::async_trait;
//! use langgraph_core::store::{Store, Result};
//!
//! pub struct RedisStore {
//!     client: redis::Client,
//! }
//!
//! #[async_trait]
//! impl Store for RedisStore {
//!     async fn get(&self, key: &str) -> Result<Option<Value>> {
//!         // Use redis client to get value
//!         todo!()
//!     }
//!
//!     async fn put(&self, key: &str, value: Value) -> Result<()> {
//!         // Use redis client to set value
//!         todo!()
//!     }
//!
//!     // Implement remaining methods...
//! }
//! ```
//!
//! # See Also
//!
//! - [`crate::compiled::CompiledGraph`] - Execute graphs that use Store/Cache
//! - [`crate::builder::StateGraph`] - Build graphs with Store/Cache access
//! - [`langgraph_checkpoint::CheckpointSaver`] - Persistent state checkpointing
//! - [`crate::state`] - Graph state management
//! - Python LangGraph Store documentation

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Error type for Store operations
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Key not found
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(String),

    /// Other error
    #[error("Store error: {0}")]
    Other(String),
}

/// Result type for Store operations
pub type Result<T> = std::result::Result<T, StoreError>;

/// Store trait for key-value storage
///
/// Implementations can use any backend: in-memory, Redis, database, etc.
#[async_trait]
pub trait Store: Send + Sync {
    /// Get a value by key
    ///
    /// # Arguments
    ///
    /// * `key` - The key to retrieve
    ///
    /// # Returns
    ///
    /// The value associated with the key, or None if not found
    async fn get(&self, key: &str) -> Result<Option<Value>>;

    /// Store a value by key
    ///
    /// # Arguments
    ///
    /// * `key` - The key to store under
    /// * `value` - The value to store
    async fn put(&self, key: &str, value: Value) -> Result<()>;

    /// Delete a value by key
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete
    ///
    /// # Returns
    ///
    /// true if the key existed and was deleted, false otherwise
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Check if a key exists
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    async fn exists(&self, key: &str) -> Result<bool>;

    /// List all keys with an optional prefix
    ///
    /// # Arguments
    ///
    /// * `prefix` - Optional prefix to filter keys
    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>>;

    /// Clear all keys with an optional prefix
    ///
    /// # Arguments
    ///
    /// * `prefix` - Optional prefix to filter keys to clear
    async fn clear(&self, prefix: Option<&str>) -> Result<usize>;
}

/// In-memory implementation of Store
///
/// This is a simple, thread-safe in-memory store suitable for development
/// and testing. For production use, consider implementing Store with a
/// persistent backend like Redis or a database.
#[derive(Clone)]
pub struct InMemoryStore {
    data: Arc<RwLock<HashMap<String, Value>>>,
}

impl InMemoryStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the number of keys in the store
    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Store for InMemoryStore {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        let data = self.data.read().unwrap();
        Ok(data.get(key).cloned())
    }

    async fn put(&self, key: &str, value: Value) -> Result<()> {
        let mut data = self.data.write().unwrap();
        data.insert(key.to_string(), value);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let mut data = self.data.write().unwrap();
        Ok(data.remove(key).is_some())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let data = self.data.read().unwrap();
        Ok(data.contains_key(key))
    }

    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let data = self.data.read().unwrap();
        let keys: Vec<String> = match prefix {
            Some(p) => data
                .keys()
                .filter(|k| k.starts_with(p))
                .cloned()
                .collect(),
            None => data.keys().cloned().collect(),
        };
        Ok(keys)
    }

    async fn clear(&self, prefix: Option<&str>) -> Result<usize> {
        let mut data = self.data.write().unwrap();
        match prefix {
            Some(p) => {
                let to_remove: Vec<String> = data
                    .keys()
                    .filter(|k| k.starts_with(p))
                    .cloned()
                    .collect();
                let count = to_remove.len();
                for key in to_remove {
                    data.remove(&key);
                }
                Ok(count)
            }
            None => {
                let count = data.len();
                data.clear();
                Ok(count)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_store_put_get() {
        let store = InMemoryStore::new();

        store.put("key1", json!({"value": 42})).await.unwrap();

        let result = store.get("key1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), json!({"value": 42}));
    }

    #[tokio::test]
    async fn test_store_get_nonexistent() {
        let store = InMemoryStore::new();

        let result = store.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_store_delete() {
        let store = InMemoryStore::new();

        store.put("key1", json!("value1")).await.unwrap();
        assert_eq!(store.len(), 1);

        let deleted = store.delete("key1").await.unwrap();
        assert!(deleted);
        assert_eq!(store.len(), 0);

        let deleted_again = store.delete("key1").await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_store_exists() {
        let store = InMemoryStore::new();

        store.put("key1", json!("value1")).await.unwrap();

        let exists = store.exists("key1").await.unwrap();
        assert!(exists);

        let not_exists = store.exists("key2").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_store_list_keys() {
        let store = InMemoryStore::new();

        store.put("user:1", json!({"name": "Alice"})).await.unwrap();
        store.put("user:2", json!({"name": "Bob"})).await.unwrap();
        store.put("config:theme", json!("dark")).await.unwrap();

        // List all keys
        let all_keys = store.list_keys(None).await.unwrap();
        assert_eq!(all_keys.len(), 3);

        // List with prefix
        let user_keys = store.list_keys(Some("user:")).await.unwrap();
        assert_eq!(user_keys.len(), 2);
        assert!(user_keys.contains(&"user:1".to_string()));
        assert!(user_keys.contains(&"user:2".to_string()));
    }

    #[tokio::test]
    async fn test_store_clear() {
        let store = InMemoryStore::new();

        store.put("user:1", json!({"name": "Alice"})).await.unwrap();
        store.put("user:2", json!({"name": "Bob"})).await.unwrap();
        store.put("config:theme", json!("dark")).await.unwrap();

        // Clear with prefix
        let cleared = store.clear(Some("user:")).await.unwrap();
        assert_eq!(cleared, 2);
        assert_eq!(store.len(), 1);

        // Clear all
        let cleared_all = store.clear(None).await.unwrap();
        assert_eq!(cleared_all, 1);
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn test_store_overwrite() {
        let store = InMemoryStore::new();

        store.put("key1", json!("value1")).await.unwrap();
        store.put("key1", json!("value2")).await.unwrap();

        let result = store.get("key1").await.unwrap();
        assert_eq!(result.unwrap(), json!("value2"));
    }
}

/// Cache trait for temporary key-value storage with TTL support
///
/// Caches are similar to Stores but support time-to-live (TTL) for automatic
/// expiration of cached values. This is useful for:
/// - Caching LLM responses with expiration
/// - Rate limiting
/// - Session management
/// - Temporary computation results
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a value by key
    ///
    /// Returns None if key doesn't exist or has expired
    async fn get(&self, key: &str) -> Result<Option<Value>>;

    /// Store a value with optional TTL in seconds
    ///
    /// # Arguments
    ///
    /// * `key` - The key to store under
    /// * `value` - The value to store
    /// * `ttl` - Optional time-to-live in seconds (None = no expiration)
    async fn put(&self, key: &str, value: Value, ttl: Option<u64>) -> Result<()>;

    /// Delete a value by key
    ///
    /// Returns true if the key existed and was deleted
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Check if a key exists and has not expired
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Clear all expired entries
    ///
    /// Returns the number of entries cleared
    async fn clear_expired(&self) -> Result<usize>;

    /// Clear all entries
    ///
    /// Returns the number of entries cleared
    async fn clear_all(&self) -> Result<usize>;
}

/// In-memory implementation of Cache with TTL support
///
/// This implementation uses tokio's time utilities for TTL checking.
/// Entries are lazily expired on access, not actively cleaned up.
#[derive(Clone)]
pub struct InMemoryCache {
    data: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

#[derive(Clone)]
struct CacheEntry {
    value: Value,
    expires_at: Option<std::time::Instant>,
}

impl InMemoryCache {
    /// Create a new in-memory cache
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the number of keys in the cache (including expired)
    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }

    /// Check if an entry has expired
    fn is_expired(entry: &CacheEntry) -> bool {
        if let Some(expires_at) = entry.expires_at {
            std::time::Instant::now() > expires_at
        } else {
            false
        }
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get(&self, key: &str) -> Result<Option<Value>> {
        let mut data = self.data.write().unwrap();

        if let Some(entry) = data.get(key) {
            if Self::is_expired(entry) {
                // Remove expired entry
                data.remove(key);
                Ok(None)
            } else {
                Ok(Some(entry.value.clone()))
            }
        } else {
            Ok(None)
        }
    }

    async fn put(&self, key: &str, value: Value, ttl: Option<u64>) -> Result<()> {
        let mut data = self.data.write().unwrap();

        let expires_at = ttl.map(|seconds| {
            std::time::Instant::now() + std::time::Duration::from_secs(seconds)
        });

        let entry = CacheEntry {
            value,
            expires_at,
        };

        data.insert(key.to_string(), entry);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let mut data = self.data.write().unwrap();
        Ok(data.remove(key).is_some())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let mut data = self.data.write().unwrap();

        if let Some(entry) = data.get(key) {
            if Self::is_expired(entry) {
                data.remove(key);
                Ok(false)
            } else {
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    async fn clear_expired(&self) -> Result<usize> {
        let mut data = self.data.write().unwrap();

        let to_remove: Vec<String> = data
            .iter()
            .filter(|(_k, entry)| Self::is_expired(entry))
            .map(|(k, _)| k.clone())
            .collect();

        let count = to_remove.len();
        for key in to_remove {
            data.remove(&key);
        }

        Ok(count)
    }

    async fn clear_all(&self) -> Result<usize> {
        let mut data = self.data.write().unwrap();
        let count = data.len();
        data.clear();
        Ok(count)
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_cache_put_get() {
        let cache = InMemoryCache::new();

        cache.put("key1", json!({"value": 42}), None).await.unwrap();

        let result = cache.get("key1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), json!({"value": 42}));
    }

    #[tokio::test]
    async fn test_cache_get_nonexistent() {
        let cache = InMemoryCache::new();

        let result = cache.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let cache = InMemoryCache::new();

        // Put with 1 second TTL
        cache.put("key1", json!("value1"), Some(1)).await.unwrap();

        // Should exist immediately
        let result = cache.get("key1").await.unwrap();
        assert!(result.is_some());

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should be expired
        let result = cache.get("key1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_no_ttl() {
        let cache = InMemoryCache::new();

        cache.put("key1", json!("value1"), None).await.unwrap();

        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Should still exist (no TTL)
        let result = cache.get("key1").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_cache_delete() {
        let cache = InMemoryCache::new();

        cache.put("key1", json!("value1"), None).await.unwrap();
        assert_eq!(cache.len(), 1);

        let deleted = cache.delete("key1").await.unwrap();
        assert!(deleted);
        assert_eq!(cache.len(), 0);

        let deleted_again = cache.delete("key1").await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_cache_exists() {
        let cache = InMemoryCache::new();

        cache.put("key1", json!("value1"), None).await.unwrap();

        let exists = cache.exists("key1").await.unwrap();
        assert!(exists);

        let not_exists = cache.exists("key2").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_cache_exists_expired() {
        let cache = InMemoryCache::new();

        // Put with very short TTL
        cache.put("key1", json!("value1"), Some(1)).await.unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should not exist (expired)
        let exists = cache.exists("key1").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_cache_clear_expired() {
        let cache = InMemoryCache::new();

        // Add entries with different TTLs
        cache.put("expired1", json!("value1"), Some(1)).await.unwrap();
        cache.put("expired2", json!("value2"), Some(1)).await.unwrap();
        cache.put("permanent", json!("value3"), None).await.unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Clear expired
        let cleared = cache.clear_expired().await.unwrap();
        assert_eq!(cleared, 2);
        assert_eq!(cache.len(), 1);

        // Permanent key should still exist
        let result = cache.get("permanent").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_cache_clear_all() {
        let cache = InMemoryCache::new();

        cache.put("key1", json!("value1"), None).await.unwrap();
        cache.put("key2", json!("value2"), Some(10)).await.unwrap();
        cache.put("key3", json!("value3"), None).await.unwrap();

        let cleared = cache.clear_all().await.unwrap();
        assert_eq!(cleared, 3);
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_cache_overwrite() {
        let cache = InMemoryCache::new();

        cache.put("key1", json!("value1"), None).await.unwrap();
        cache.put("key1", json!("value2"), None).await.unwrap();

        let result = cache.get("key1").await.unwrap();
        assert_eq!(result.unwrap(), json!("value2"));
    }

    #[tokio::test]
    async fn test_cache_overwrite_ttl() {
        let cache = InMemoryCache::new();

        // Put with short TTL
        cache.put("key1", json!("value1"), Some(1)).await.unwrap();

        // Immediately overwrite with no TTL
        cache.put("key1", json!("value2"), None).await.unwrap();

        // Wait past original TTL
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should still exist (no TTL on second put)
        let result = cache.get("key1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), json!("value2"));
    }
}
