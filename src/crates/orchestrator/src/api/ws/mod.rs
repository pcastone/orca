//! WebSocket API support
//!
//! Provides WebSocket functionality for real-time event streaming and updates.

pub mod handler;
pub mod pool;
pub mod metrics;
pub mod rate_limit;
pub mod backpressure;
pub mod error;
pub mod timeout;
pub mod compression;
pub mod progress;
pub mod events;
pub mod filters;
pub mod batching;
pub mod replay;

pub use handler::{ws_handler, WsEvent, BroadcastState};
pub use pool::{ConnectionPool, PoolEntry, PoolStats};
pub use metrics::{WebSocketMetrics, MetricsSnapshot};
pub use rate_limit::RateLimiter;
pub use backpressure::{BackpressureManager, ClientBackpressure, QueuedMessage};
pub use error::{WsError, WsResult};
pub use timeout::{TimeoutManager, TimeoutConfig, ClientTimeout};
pub use compression::{MessageCompressor, CompressionLevel, CompressionStats};
pub use progress::{TaskProgress, ProgressEvent, TaskProgressTracker, ProgressManager};
pub use events::{RealtimeEvent, EventPriority};
pub use filters::{EventFilter, ClientFilter, FilterManager};
pub use batching::{EventBatch, ClientBatcher, BatchingManager};
pub use replay::{StoredEvent, ReplayCriteria, EventHistory};
