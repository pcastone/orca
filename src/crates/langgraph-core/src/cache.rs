//! Performance Cache - Generic caching with eviction policies and metrics
//!
//! This module provides a high-performance, generic caching layer for optimizing
//! graph execution. Unlike the simple [`Cache`](crate::store::Cache) trait in the
//! store module, this cache is designed for internal performance optimization with:
//! - Multiple eviction policies (LRU, LFU, FIFO, TTL)
//! - Detailed metrics tracking (hits, misses, evictions)
//! - Generic type support for any key-value pair
//! - Specialized caches for nodes, tools, and checkpoints
//!
//! # Overview
//!
//! The cache system is designed to **avoid redundant computation** by memoizing
//! results of expensive operations:
//! - **Node execution results** - Cache deterministic node outputs
//! - **Tool call results** - Avoid repeated LLM tool invocations
//! - **Checkpoint loading** - Speed up state restoration from storage
//!
//! **Use this cache when:**
//! - Operations are expensive (network calls, LLM invocations, database queries)
//! - Operations are deterministic (same input → same output)
//! - Memory trade-off is acceptable for performance gain
//!
//! **Don't use when:**
//! - Operations are cheap (O(1) memory access)
//! - Operations have side effects or are non-deterministic
//! - Memory constraints are tight
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │  Graph Execution Layer                                        │
//! │  • Node executions                                            │
//! │  • Tool invocations                                           │
//! │  • Checkpoint loading                                         │
//! └─────────────┬────────────────────────────────────────────────┘
//!               │
//!               ↓ Check cache before computation
//! ┌──────────────────────────────────────────────────────────────┐
//! │  Cache<K, V> - Generic cache with configurable policies      │
//! │                                                               │
//! │  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────┐│
//! │  │ CacheEntry<V>   │  │ EvictionPolicy   │  │ Metrics     ││
//! │  │ • value: V      │  │ • LRU (recency)  │  │ • hits      ││
//! │  │ • timestamps    │  │ • LFU (frequency)│  │ • misses    ││
//! │  │ • access_count  │  │ • FIFO (age)     │  │ • evictions ││
//! │  │ • expires_at    │  │ • TTL (time)     │  │ • hit_ratio ││
//! │  └─────────────────┘  └──────────────────┘  └─────────────┘│
//! └───────────────────────────┬──────────────────────────────────┘
//!                             │
//!                             ↓ Store in HashMap
//! ┌──────────────────────────────────────────────────────────────┐
//! │  Storage: Arc<RwLock<HashMap<K, CacheEntry<V>>>>             │
//! │  • Thread-safe with RwLock                                   │
//! │  • Multiple readers, single writer                           │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ## Basic Generic Cache Usage
//!
//! ```rust,ignore
//! use langgraph_core::cache::{Cache, CacheConfig, EvictionPolicy};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = CacheConfig {
//!         max_size: 1000,
//!         default_ttl: Some(Duration::from_secs(3600)), // 1 hour
//!         eviction_policy: EvictionPolicy::LRU,
//!         track_metrics: true,
//!     };
//!
//!     let cache: Cache<String, String> = Cache::new(config);
//!
//!     // Put a value
//!     cache.put("key1".to_string(), "expensive_result".to_string()).await;
//!
//!     // Get from cache
//!     if let Some(value) = cache.get(&"key1".to_string()).await {
//!         println!("Cache hit: {}", value);
//!     }
//!
//!     // Check metrics
//!     let metrics = cache.metrics().await;
//!     println!("Hit ratio: {:.2}%", metrics.hit_ratio() * 100.0);
//! }
//! ```
//!
//! ## Node Execution Caching
//!
//! ```rust,ignore
//! use langgraph_core::cache::{create_node_cache, NodeCache};
//! use std::time::Duration;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() {
//!     let node_cache = create_node_cache(500, Duration::from_secs(300));
//!
//!     let node_id = "expensive_llm_node";
//!     let cache_key = format!("{}:input_hash_abc123", node_id);
//!
//!     // Check cache before execution
//!     if let Some(cached_output) = node_cache.get(&cache_key).await {
//!         println!("Using cached node output");
//!         return cached_output;
//!     }
//!
//!     // Cache miss - execute node
//!     let output = execute_expensive_node().await;
//!     node_cache.put(cache_key, output.clone()).await;
//!
//!     output
//! }
//! ```
//!
//! ## Get-or-Compute Pattern
//!
//! ```rust,ignore
//! use langgraph_core::cache::Cache;
//!
//! async fn get_llm_response(
//!     cache: &Cache<String, String>,
//!     prompt: String
//! ) -> String {
//!     cache.get_or_compute(prompt.clone(), || async {
//!         // This closure only runs on cache miss
//!         expensive_llm_call(&prompt).await
//!     }).await
//! }
//! ```
//!
//! # Common Patterns
//!
//! ## Pattern 1: Tool Call Caching
//!
//! Cache tool invocations to avoid repeated API calls:
//!
//! ```rust,ignore
//! use langgraph_core::cache::{create_tool_cache, ToolCache};
//! use std::time::Duration;
//! use serde_json::{json, Value};
//!
//! async fn cached_tool_execution(
//!     cache: &ToolCache,
//!     tool_name: &str,
//!     input: Value
//! ) -> Result<Value, Box<dyn std::error::Error>> {
//!     let cache_key = (tool_name.to_string(), input.clone());
//!
//!     // Check cache
//!     if let Some(output) = cache.get(&cache_key).await {
//!         return Ok(output);
//!     }
//!
//!     // Execute tool
//!     let output = execute_tool(tool_name, &input).await?;
//!
//!     // Cache result (5 minutes TTL)
//!     cache.put_with_ttl(cache_key, output.clone(), Some(Duration::from_secs(300))).await;
//!
//!     Ok(output)
//! }
//! ```
//!
//! ## Pattern 2: Checkpoint Loading Optimization
//!
//! Cache deserialized checkpoints to avoid repeated parsing:
//!
//! ```rust,ignore
//! use langgraph_core::cache::{create_checkpoint_cache, CheckpointCache};
//!
//! async fn load_checkpoint_with_cache(
//!     cache: &CheckpointCache,
//!     checkpoint_id: &str
//! ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
//!     // Check cache first
//!     if let Some(checkpoint_data) = cache.get(&checkpoint_id.to_string()).await {
//!         return Ok(checkpoint_data);
//!     }
//!
//!     // Load from storage (expensive I/O)
//!     let checkpoint_data = load_from_database(checkpoint_id).await?;
//!
//!     // Cache for 24 hours
//!     cache.put(checkpoint_id.to_string(), checkpoint_data.clone()).await;
//!
//!     Ok(checkpoint_data)
//! }
//! ```
//!
//! ## Pattern 3: Conditional Caching Based on Input Size
//!
//! Only cache small inputs to avoid memory bloat:
//!
//! ```rust,ignore
//! async fn smart_cached_computation(
//!     cache: &Cache<String, String>,
//!     input: String
//! ) -> String {
//!     const MAX_CACHEABLE_SIZE: usize = 1024; // 1KB
//!
//!     if input.len() > MAX_CACHEABLE_SIZE {
//!         // Too large to cache - compute directly
//!         return expensive_computation(&input).await;
//!     }
//!
//!     // Use cache for small inputs
//!     cache.get_or_compute(input.clone(), || async {
//!         expensive_computation(&input).await
//!     }).await
//! }
//! ```
//!
//! ## Pattern 4: Selective Caching with TTL by Operation Type
//!
//! Use different TTLs for different operation types:
//!
//! ```rust,ignore
//! use std::time::Duration;
//!
//! async fn cache_with_operation_ttl(
//!     cache: &NodeCache,
//!     operation_type: &str,
//!     key: String,
//!     value: serde_json::Value
//! ) {
//!     let ttl = match operation_type {
//!         "llm_call" => Duration::from_secs(300),      // 5 minutes
//!         "database_query" => Duration::from_secs(60),  // 1 minute
//!         "api_call" => Duration::from_secs(600),       // 10 minutes
//!         _ => Duration::from_secs(180),                // 3 minutes default
//!     };
//!
//!     cache.put_with_ttl(key, value, Some(ttl)).await;
//! }
//! ```
//!
//! ## Pattern 5: Cache Metrics Monitoring
//!
//! Track cache performance and adjust configuration:
//!
//! ```rust,ignore
//! use tokio::time::{interval, Duration};
//!
//! async fn monitor_cache_performance(cache: NodeCache) {
//!     let mut ticker = interval(Duration::from_secs(60));
//!
//!     loop {
//!         ticker.tick().await;
//!
//!         let metrics = cache.metrics().await;
//!         let hit_ratio = metrics.hit_ratio() * 100.0;
//!
//!         println!("Cache Stats:");
//!         println!("  Hit Ratio: {:.2}%", hit_ratio);
//!         println!("  Hits: {}", metrics.hits);
//!         println!("  Misses: {}", metrics.misses);
//!         println!("  Evictions: {}", metrics.evictions);
//!         println!("  Entries: {}", metrics.entries);
//!
//!         // Alert if hit ratio is too low
//!         if hit_ratio < 50.0 && metrics.hits + metrics.misses > 100 {
//!             eprintln!("WARNING: Low cache hit ratio - consider increasing cache size or TTL");
//!         }
//!     }
//! }
//! ```
//!
//! # Eviction Policies
//!
//! Choose the right policy based on your access patterns:
//!
//! | Policy | Best For | Complexity |
//! |--------|----------|------------|
//! | **LRU** (Least Recently Used) | Most workloads, temporal locality | O(1) get/put |
//! | **LFU** (Least Frequently Used) | Stable hot data, checkpoint caching | O(1) get/put |
//! | **FIFO** (First In First Out) | Simple eviction, predictable behavior | O(1) get/put |
//! | **TTL** (Time To Live) | Time-sensitive data, automatic expiration | O(n) eviction |
//!
//! **Choosing a Policy:**
//! - **LRU**: Default choice - works well for most use cases with temporal locality
//! - **LFU**: Use when frequently accessed items should stay (checkpoints, common queries)
//! - **FIFO**: Simple and predictable, good for streaming data
//! - **TTL**: When data freshness matters more than access patterns
//!
//! # Performance Considerations
//!
//! ## Time Complexity
//!
//! - **Get**: O(1) average - HashMap lookup + metadata update
//! - **Put**: O(1) average - HashMap insert, O(n) worst case if eviction scans
//! - **Remove**: O(1) - HashMap remove
//! - **Eviction**: O(n) for LRU/LFU/FIFO/TTL (scans all entries)
//!
//! ## Memory Usage
//!
//! Each cache entry stores:
//! - Value data (size varies)
//! - 3 x `Instant` timestamps (24 bytes)
//! - 1 x `usize` access count (8 bytes)
//! - HashMap overhead (~32 bytes per entry)
//!
//! **Total overhead per entry: ~64 bytes + value size**
//!
//! ## Concurrency
//!
//! - Uses `RwLock` for thread-safe access
//! - **Multiple readers**: Can read concurrently without blocking
//! - **Single writer**: Writes block all other access
//! - **Contention**: High write frequency can cause lock contention
//!
//! **Best Practices:**
//! 1. **Size the cache appropriately**: Balance memory vs hit ratio
//!    ```rust,ignore
//!    // For node caching: 100-1000 entries typical
//!    let node_cache = create_node_cache(500, Duration::from_secs(300));
//!
//!    // For tool caching: 50-500 entries typical
//!    let tool_cache = create_tool_cache(200, Duration::from_secs(600));
//!
//!    // For checkpoints: 10-100 entries typical
//!    let checkpoint_cache = create_checkpoint_cache(50);
//!    ```
//!
//! 2. **Choose appropriate TTL**: Balance freshness vs cache hits
//!    - Short TTL (1-5 min): Frequently changing data
//!    - Medium TTL (5-30 min): Stable computation results
//!    - Long TTL (1-24 hours): Rarely changing data (checkpoints)
//!
//! 3. **Monitor metrics**: Track hit ratio and adjust configuration
//!    ```rust,ignore
//!    let metrics = cache.metrics().await;
//!    if metrics.hit_ratio() < 0.5 {
//!        // Consider increasing cache size or TTL
//!    }
//!    ```
//!
//! 4. **Avoid caching large values**: Keep entries < 1MB each
//!    - Large values increase eviction overhead
//!    - Can cause memory pressure and OOM
//!
//! 5. **Use get_or_compute()**: Prevents cache stampede
//!    - Multiple concurrent requests won't duplicate computation
//!    - Built-in pattern for check-compute-store
//!
//! # When to Use This vs store::Cache
//!
//! | Use cache.rs (This Module) | Use store::Cache (Store Module) |
//! |---------------------------|----------------------------------|
//! | Internal performance optimization | User-facing data storage |
//! | Node/tool execution caching | Session management |
//! | Checkpoint loading optimization | Rate limiting |
//! | Eviction policies needed (LRU/LFU) | Simple TTL sufficient |
//! | Metrics tracking needed | Persistence across restarts |
//! | Generic type support | String-keyed JSON values |
//! | In-memory only | Pluggable backends (Redis, DB) |
//!
//! # Python LangGraph Comparison
//!
//! Python LangGraph uses **functools.lru_cache** for simple caching:
//!
//! ```python
//! from functools import lru_cache
//!
//! @lru_cache(maxsize=1000)
//! def cached_node_execution(input_hash):
//!     return expensive_computation(input_hash)
//! ```
//!
//! **Rust Equivalent:**
//! ```rust,ignore
//! let cache: Cache<String, String> = Cache::new(CacheConfig {
//!     max_size: 1000,
//!     eviction_policy: EvictionPolicy::LRU,
//!     ..Default::default()
//! });
//!
//! async fn cached_node_execution(
//!     cache: &Cache<String, String>,
//!     input_hash: String
//! ) -> String {
//!     cache.get_or_compute(input_hash.clone(), || async {
//!         expensive_computation(&input_hash).await
//!     }).await
//! }
//! ```
//!
//! **Key Differences:**
//! - Python uses decorator syntax, Rust uses explicit cache parameter
//! - Python LRU is sync, Rust cache is async-first
//! - Rust provides more eviction policies and detailed metrics
//! - Rust requires explicit async/await
//!
//! # See Also
//!
//! - [`crate::store::Cache`] - Simple key-value cache with TTL for user-facing data
//! - [`crate::compiled::CompiledGraph`] - Uses node caching during execution
//! - [`crate::tool`] - Tool execution can leverage tool caching
//! - [`langgraph_checkpoint::CheckpointSaver`] - Checkpoint loading can use checkpoint cache
//! - [Rust caching libraries](https://crates.io/keywords/cache) - External caching solutions

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// The cached value
    pub value: T,

    /// When the entry was created
    pub created_at: Instant,

    /// When the entry was last accessed
    pub last_accessed: Instant,

    /// Number of times this entry has been accessed
    pub access_count: usize,

    /// Optional expiration time
    pub expires_at: Option<Instant>,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry
    pub fn new(value: T, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        let expires_at = ttl.map(|duration| now + duration);

        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            expires_at,
        }
    }

    /// Check if the entry has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            Instant::now() > expiry
        } else {
            false
        }
    }

    /// Update access metadata
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }

    /// Get age of the entry
    pub fn age(&self) -> Duration {
        Instant::now() - self.created_at
    }
}

/// Cache eviction policies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,

    /// Least Frequently Used
    LFU,

    /// First In First Out
    FIFO,

    /// Time-based (relies on TTL)
    TTL,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries
    pub max_size: usize,

    /// Default time-to-live for entries
    pub default_ttl: Option<Duration>,

    /// Eviction policy
    pub eviction_policy: EvictionPolicy,

    /// Whether to track access patterns
    pub track_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 1000,
            default_ttl: Some(Duration::from_secs(3600)), // 1 hour
            eviction_policy: EvictionPolicy::LRU,
            track_metrics: true,
        }
    }
}

/// Cache metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Total number of cache hits
    pub hits: usize,

    /// Total number of cache misses
    pub misses: usize,

    /// Total number of evictions
    pub evictions: usize,

    /// Current number of entries
    pub entries: usize,

    /// Total bytes used (estimated)
    pub bytes_used: usize,
}

impl CacheMetrics {
    /// Calculate hit ratio
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Generic cache implementation
pub struct Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    /// The cache storage
    storage: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,

    /// Cache configuration
    config: CacheConfig,

    /// Cache metrics
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl<K, V> Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    /// Create a new cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }

    /// Get a value from the cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut storage = self.storage.write().await;

        if let Some(entry) = storage.get_mut(key) {
            if entry.is_expired() {
                // Remove expired entry
                storage.remove(key);

                if self.config.track_metrics {
                    let mut metrics = self.metrics.write().await;
                    metrics.misses += 1;
                    metrics.entries = storage.len();
                }

                None
            } else {
                // Update access metadata
                entry.touch();

                if self.config.track_metrics {
                    let mut metrics = self.metrics.write().await;
                    metrics.hits += 1;
                }

                Some(entry.value.clone())
            }
        } else {
            if self.config.track_metrics {
                let mut metrics = self.metrics.write().await;
                metrics.misses += 1;
            }

            None
        }
    }

    /// Put a value in the cache
    pub async fn put(&self, key: K, value: V) {
        self.put_with_ttl(key, value, self.config.default_ttl).await;
    }

    /// Put a value with specific TTL
    pub async fn put_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
        let mut storage = self.storage.write().await;

        // Check if we need to evict
        if storage.len() >= self.config.max_size && !storage.contains_key(&key) {
            self.evict(&mut storage).await;
        }

        // Insert the new entry
        storage.insert(key, CacheEntry::new(value, ttl));

        if self.config.track_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.entries = storage.len();
        }
    }

    /// Remove a value from the cache
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut storage = self.storage.write().await;
        let entry = storage.remove(key);

        if self.config.track_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.entries = storage.len();
        }

        entry.map(|e| e.value)
    }

    /// Clear all entries
    pub async fn clear(&self) {
        let mut storage = self.storage.write().await;
        storage.clear();

        if self.config.track_metrics {
            let mut metrics = self.metrics.write().await;
            metrics.entries = 0;
        }
    }

    /// Get cache metrics
    pub async fn metrics(&self) -> CacheMetrics {
        self.metrics.read().await.clone()
    }

    /// Evict an entry based on the eviction policy
    async fn evict(&self, storage: &mut HashMap<K, CacheEntry<V>>) {
        if storage.is_empty() {
            return;
        }

        let key_to_evict = match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                // Find least recently used
                storage
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_accessed)
                    .map(|(k, _)| k.clone())
            }
            EvictionPolicy::LFU => {
                // Find least frequently used
                storage
                    .iter()
                    .min_by_key(|(_, entry)| entry.access_count)
                    .map(|(k, _)| k.clone())
            }
            EvictionPolicy::FIFO => {
                // Find oldest entry
                storage
                    .iter()
                    .min_by_key(|(_, entry)| entry.created_at)
                    .map(|(k, _)| k.clone())
            }
            EvictionPolicy::TTL => {
                // Find expired entries first, then oldest
                storage
                    .iter()
                    .filter(|(_, entry)| entry.is_expired())
                    .min_by_key(|(_, entry)| entry.created_at)
                    .or_else(|| {
                        storage
                            .iter()
                            .min_by_key(|(_, entry)| entry.created_at)
                    })
                    .map(|(k, _)| k.clone())
            }
        };

        if let Some(key) = key_to_evict {
            storage.remove(&key);

            if self.config.track_metrics {
                let mut metrics = self.metrics.write().await;
                metrics.evictions += 1;
            }
        }
    }

    /// Get or compute a value
    pub async fn get_or_compute<F, Fut>(&self, key: K, compute: F) -> V
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = V>,
    {
        if let Some(value) = self.get(&key).await {
            value
        } else {
            let value = compute().await;
            self.put(key, value.clone()).await;
            value
        }
    }
}

/// Node execution cache
pub type NodeCache = Cache<String, Value>;

/// Tool execution cache
pub type ToolCache = Cache<(String, Value), Value>; // (tool_name, input) -> output

/// Checkpoint cache
pub type CheckpointCache = Cache<String, Vec<u8>>;

/// Create a node execution cache
pub fn create_node_cache(max_size: usize, ttl: Duration) -> NodeCache {
    let config = CacheConfig {
        max_size,
        default_ttl: Some(ttl),
        eviction_policy: EvictionPolicy::LRU,
        track_metrics: true,
    };
    Cache::new(config)
}

/// Create a tool execution cache
pub fn create_tool_cache(max_size: usize, ttl: Duration) -> ToolCache {
    let config = CacheConfig {
        max_size,
        default_ttl: Some(ttl),
        eviction_policy: EvictionPolicy::LRU,
        track_metrics: true,
    };
    Cache::new(config)
}

/// Create a checkpoint cache
pub fn create_checkpoint_cache(max_size: usize) -> CheckpointCache {
    let config = CacheConfig {
        max_size,
        default_ttl: Some(Duration::from_secs(3600 * 24)), // 24 hours
        eviction_policy: EvictionPolicy::LFU, // Checkpoints accessed frequently should stay
        track_metrics: true,
    };
    Cache::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_cache_operations() {
        let cache: Cache<String, String> = Cache::new(CacheConfig::default());

        // Test put and get
        cache.put("key1".to_string(), "value1".to_string()).await;
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some("value1".to_string()));

        // Test miss
        let value = cache.get(&"key2".to_string()).await;
        assert_eq!(value, None);

        // Test remove
        let removed = cache.remove(&"key1".to_string()).await;
        assert_eq!(removed, Some("value1".to_string()));

        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache: Cache<String, String> = Cache::new(CacheConfig::default());

        // Put with very short TTL
        cache.put_with_ttl(
            "key1".to_string(),
            "value1".to_string(),
            Some(Duration::from_millis(50)),
        ).await;

        // Should exist immediately
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some("value1".to_string()));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_eviction() {
        let config = CacheConfig {
            max_size: 2,
            default_ttl: None,
            eviction_policy: EvictionPolicy::FIFO,
            track_metrics: true,
        };

        let cache: Cache<String, String> = Cache::new(config);

        // Fill cache to capacity
        cache.put("key1".to_string(), "value1".to_string()).await;
        cache.put("key2".to_string(), "value2".to_string()).await;

        // Add one more, should evict key1 (FIFO)
        cache.put("key3".to_string(), "value3".to_string()).await;

        // key1 should be evicted
        assert_eq!(cache.get(&"key1".to_string()).await, None);
        assert_eq!(cache.get(&"key2".to_string()).await, Some("value2".to_string()));
        assert_eq!(cache.get(&"key3".to_string()).await, Some("value3".to_string()));
    }

    #[tokio::test]
    async fn test_metrics() {
        let cache: Cache<String, String> = Cache::new(CacheConfig::default());

        // Generate some activity
        cache.put("key1".to_string(), "value1".to_string()).await;
        cache.get(&"key1".to_string()).await; // Hit
        cache.get(&"key2".to_string()).await; // Miss
        cache.get(&"key1".to_string()).await; // Hit

        let metrics = cache.metrics().await;
        assert_eq!(metrics.hits, 2);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.entries, 1);
        assert_eq!(metrics.hit_ratio(), 2.0 / 3.0);
    }

    #[tokio::test]
    async fn test_get_or_compute() {
        let cache: Cache<String, String> = Cache::new(CacheConfig::default());

        let mut compute_count = 0;

        // First call should compute
        let value = cache.get_or_compute("key1".to_string(), || {
            compute_count += 1;
            async { "computed_value".to_string() }
        }).await;

        assert_eq!(value, "computed_value".to_string());
        assert_eq!(compute_count, 1);

        // Second call should use cache
        let value = cache.get_or_compute("key1".to_string(), || {
            compute_count += 1;
            async { "should_not_compute".to_string() }
        }).await;

        assert_eq!(value, "computed_value".to_string());
        assert_eq!(compute_count, 1); // Should still be 1
    }

    #[test]
    fn test_cache_entry() {
        let entry = CacheEntry::new("value", Some(Duration::from_secs(60)));

        assert!(!entry.is_expired());
        assert_eq!(entry.access_count, 1);

        let mut entry = entry;
        entry.touch();
        assert_eq!(entry.access_count, 2);
    }
}