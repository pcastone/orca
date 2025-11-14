//! Event filtering for selective streaming
//!
//! Allows clients to filter events by type, task, and status.

use crate::api::ws::events::RealtimeEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

/// Event filter criteria
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventFilter {
    /// Event types to include (empty = all)
    pub event_types: HashSet<String>,
    /// Task IDs to include (empty = all)
    pub task_ids: HashSet<String>,
    /// Execution IDs to include (empty = all)
    pub execution_ids: HashSet<String>,
    /// Workflow IDs to include (empty = all)
    pub workflow_ids: HashSet<String>,
    /// Minimum priority (0 = all, 1 = normal+, 2 = high only)
    pub min_priority: u32,
}

impl EventFilter {
    /// Create new empty filter (matches all events)
    pub fn new() -> Self {
        Self::default()
    }

    /// Add event type to filter
    pub fn with_event_type(mut self, event_type: String) -> Self {
        self.event_types.insert(event_type);
        self
    }

    /// Add task ID to filter
    pub fn with_task_id(mut self, task_id: String) -> Self {
        self.task_ids.insert(task_id);
        self
    }

    /// Add execution ID to filter
    pub fn with_execution_id(mut self, execution_id: String) -> Self {
        self.execution_ids.insert(execution_id);
        self
    }

    /// Add workflow ID to filter
    pub fn with_workflow_id(mut self, workflow_id: String) -> Self {
        self.workflow_ids.insert(workflow_id);
        self
    }

    /// Set minimum priority
    pub fn with_min_priority(mut self, priority: u32) -> Self {
        self.min_priority = priority;
        self
    }

    /// Check if event matches filter
    pub fn matches(&self, event: &RealtimeEvent) -> bool {
        // Check event type
        if !self.event_types.is_empty() && !self.event_types.contains(event.event_type()) {
            return false;
        }

        // Check priority
        let event_priority = event.priority() as u32;
        if event_priority < self.min_priority {
            return false;
        }

        // Check task ID
        if !self.task_ids.is_empty() {
            if let Some(task_id) = event.task_id() {
                if !self.task_ids.contains(task_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check execution ID
        if !self.execution_ids.is_empty() {
            if let Some(execution_id) = event.execution_id() {
                if !self.execution_ids.contains(execution_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check workflow ID
        if !self.workflow_ids.is_empty() {
            if let Some(workflow_id) = event.workflow_id() {
                if !self.workflow_ids.contains(workflow_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Check if filter is empty (matches all)
    pub fn is_empty(&self) -> bool {
        self.event_types.is_empty()
            && self.task_ids.is_empty()
            && self.execution_ids.is_empty()
            && self.workflow_ids.is_empty()
            && self.min_priority == 0
    }
}

/// Per-client filter manager
#[derive(Clone)]
pub struct ClientFilter {
    /// Client ID
    client_id: String,
    /// Active filter
    filter: Arc<parking_lot::Mutex<EventFilter>>,
}

impl ClientFilter {
    /// Create new client filter
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            filter: Arc::new(parking_lot::Mutex::new(EventFilter::new())),
        }
    }

    /// Update filter
    pub fn set_filter(&self, filter: EventFilter) {
        *self.filter.lock() = filter;
    }

    /// Check if event passes filter
    pub fn matches(&self, event: &RealtimeEvent) -> bool {
        self.filter.lock().matches(event)
    }

    /// Get current filter
    pub fn get_filter(&self) -> EventFilter {
        self.filter.lock().clone()
    }

    /// Clear filter (match all)
    pub fn clear(&self) {
        *self.filter.lock() = EventFilter::new();
    }
}

/// Global filter manager
pub struct FilterManager {
    /// Per-client filters
    filters: Arc<dashmap::DashMap<String, ClientFilter>>,
}

impl FilterManager {
    /// Create new filter manager
    pub fn new() -> Self {
        Self {
            filters: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Get or create filter for a client
    pub fn get_client_filter(&self, client_id: &str) -> ClientFilter {
        self.filters
            .entry(client_id.to_string())
            .or_insert_with(|| ClientFilter::new(client_id.to_string()))
            .clone()
    }

    /// Set filter for a client
    pub fn set_filter(&self, client_id: &str, filter: EventFilter) {
        let client_filter = self.get_client_filter(client_id);
        client_filter.set_filter(filter);
    }

    /// Check if event should be sent to a client
    pub fn should_send(&self, client_id: &str, event: &RealtimeEvent) -> bool {
        let client_filter = self.get_client_filter(client_id);
        client_filter.matches(event)
    }

    /// Get all clients interested in an event
    pub fn get_interested_clients(&self, event: &RealtimeEvent) -> Vec<String> {
        self.filters
            .iter()
            .filter(|entry| entry.value().matches(event))
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Remove client filter
    pub fn remove_client(&self, client_id: &str) {
        self.filters.remove(client_id);
    }
}

impl Default for FilterManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filter() {
        let filter = EventFilter::new();
        assert!(filter.is_empty());
    }

    #[test]
    fn test_filter_by_event_type() {
        let filter = EventFilter::new()
            .with_event_type("task.progress".to_string());

        let event = RealtimeEvent::TaskProgress {
            task_id: "task1".to_string(),
            progress: 50,
            message: "Processing".to_string(),
        };

        assert!(filter.matches(&event));

        let other_event = RealtimeEvent::Heartbeat {
            timestamp: "2025-11-10T00:00:00Z".to_string(),
        };
        assert!(!filter.matches(&other_event));
    }

    #[test]
    fn test_filter_by_task_id() {
        let filter = EventFilter::new()
            .with_task_id("task1".to_string());

        let event = RealtimeEvent::TaskProgress {
            task_id: "task1".to_string(),
            progress: 50,
            message: "Processing".to_string(),
        };

        assert!(filter.matches(&event));

        let other_event = RealtimeEvent::TaskProgress {
            task_id: "task2".to_string(),
            progress: 25,
            message: "Processing".to_string(),
        };
        assert!(!filter.matches(&other_event));
    }

    #[test]
    fn test_client_filter() {
        let client_filter = ClientFilter::new("client1".to_string());

        let filter = EventFilter::new()
            .with_event_type("task.progress".to_string());
        client_filter.set_filter(filter);

        let event = RealtimeEvent::TaskProgress {
            task_id: "task1".to_string(),
            progress: 50,
            message: "Processing".to_string(),
        };

        assert!(client_filter.matches(&event));
    }

    #[test]
    fn test_filter_manager() {
        let manager = FilterManager::new();

        let filter = EventFilter::new()
            .with_task_id("task1".to_string());
        manager.set_filter("client1", filter);

        let event = RealtimeEvent::TaskProgress {
            task_id: "task1".to_string(),
            progress: 50,
            message: "Processing".to_string(),
        };

        assert!(manager.should_send("client1", &event));
    }

    #[test]
    fn test_interested_clients() {
        let manager = FilterManager::new();

        let filter1 = EventFilter::new()
            .with_event_type("task.progress".to_string());
        manager.set_filter("client1", filter1);

        let filter2 = EventFilter::new()
            .with_event_type("task.failed".to_string());
        manager.set_filter("client2", filter2);

        let event = RealtimeEvent::TaskProgress {
            task_id: "task1".to_string(),
            progress: 50,
            message: "Processing".to_string(),
        };

        let interested = manager.get_interested_clients(&event);
        assert_eq!(interested.len(), 1);
        assert!(interested.contains(&"client1".to_string()));
    }
}
