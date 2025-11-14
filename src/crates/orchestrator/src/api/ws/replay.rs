//! Event history replay for reconnections
//!
//! Maintains event history and supports replay for reconnecting clients.

use crate::api::ws::events::RealtimeEvent;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Stored event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    /// Event data
    pub event: RealtimeEvent,
    /// Storage timestamp (seconds)
    pub stored_at: i64,
    /// Sequence number
    pub sequence: u64,
}

/// Replay criteria
#[derive(Debug, Clone)]
pub struct ReplayCriteria {
    /// From sequence number (inclusive)
    pub from_sequence: Option<u64>,
    /// To sequence number (inclusive)
    pub to_sequence: Option<u64>,
    /// From timestamp (seconds)
    pub from_timestamp: Option<i64>,
    /// Event types to include
    pub event_types: Option<Vec<String>>,
}

impl ReplayCriteria {
    /// Create new replay criteria
    pub fn new() -> Self {
        Self {
            from_sequence: None,
            to_sequence: None,
            from_timestamp: None,
            event_types: None,
        }
    }

    /// Matches a stored event
    pub fn matches(&self, event: &StoredEvent) -> bool {
        if let Some(from_seq) = self.from_sequence {
            if event.sequence < from_seq {
                return false;
            }
        }

        if let Some(to_seq) = self.to_sequence {
            if event.sequence > to_seq {
                return false;
            }
        }

        if let Some(from_time) = self.from_timestamp {
            if event.stored_at < from_time {
                return false;
            }
        }

        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event.event_type().to_string()) {
                return false;
            }
        }

        true
    }
}

impl Default for ReplayCriteria {
    fn default() -> Self {
        Self::new()
    }
}

/// Event replay history manager
pub struct EventHistory {
    /// Stored events
    history: Arc<parking_lot::Mutex<VecDeque<StoredEvent>>>,
    /// Max history size (default: 100)
    max_size: usize,
    /// Sequence counter
    sequence: Arc<std::sync::atomic::AtomicU64>,
}

impl EventHistory {
    /// Create new event history
    pub fn new(max_size: usize) -> Self {
        Self {
            history: Arc::new(parking_lot::Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Create with default size (100 events)
    pub fn new_default() -> Self {
        Self::new(100)
    }

    /// Store an event
    pub fn store(&self, event: RealtimeEvent) {
        let sequence = self.sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let stored = StoredEvent {
            event,
            stored_at: chrono::Utc::now().timestamp(),
            sequence,
        };

        let mut history = self.history.lock();

        // Remove oldest if at capacity
        if history.len() >= self.max_size {
            history.pop_front();
        }

        history.push_back(stored);
    }

    /// Get events matching criteria
    pub fn get_events(&self, criteria: &ReplayCriteria) -> Vec<StoredEvent> {
        let history = self.history.lock();
        history
            .iter()
            .filter(|e| criteria.matches(e))
            .cloned()
            .collect()
    }

    /// Get events since sequence number
    pub fn get_since_sequence(&self, sequence: u64) -> Vec<StoredEvent> {
        let criteria = ReplayCriteria {
            from_sequence: Some(sequence + 1),
            ..Default::default()
        };
        self.get_events(&criteria)
    }

    /// Get events since timestamp
    pub fn get_since_timestamp(&self, timestamp: i64) -> Vec<StoredEvent> {
        let criteria = ReplayCriteria {
            from_timestamp: Some(timestamp),
            ..Default::default()
        };
        self.get_events(&criteria)
    }

    /// Get all stored events
    pub fn get_all(&self) -> Vec<StoredEvent> {
        self.history.lock().iter().cloned().collect()
    }

    /// Get last N events
    pub fn get_last(&self, n: usize) -> Vec<StoredEvent> {
        let history = self.history.lock();
        history
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Get history size
    pub fn size(&self) -> usize {
        self.history.lock().len()
    }

    /// Get next sequence number
    pub fn next_sequence(&self) -> u64 {
        self.sequence.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Clear all history
    pub fn clear(&self) {
        self.history.lock().clear();
    }
}

impl Clone for EventHistory {
    fn clone(&self) -> Self {
        Self {
            history: Arc::clone(&self.history),
            max_size: self.max_size,
            sequence: Arc::clone(&self.sequence),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_criteria() {
        let criteria = ReplayCriteria {
            from_sequence: Some(5),
            to_sequence: None,
            from_timestamp: None,
            event_types: None,
        };

        let event = StoredEvent {
            event: RealtimeEvent::Heartbeat {
                timestamp: "2025-11-10T00:00:00Z".to_string(),
            },
            stored_at: 0,
            sequence: 10,
        };

        assert!(criteria.matches(&event));
    }

    #[test]
    fn test_event_history() {
        let history = EventHistory::new(10);

        for i in 0..5 {
            let event = RealtimeEvent::Heartbeat {
                timestamp: format!("2025-11-10T00:00:{:02}Z", i),
            };
            history.store(event);
        }

        assert_eq!(history.size(), 5);
        assert_eq!(history.next_sequence(), 5);
    }

    #[test]
    fn test_store_and_replay() {
        let history = EventHistory::new(10);

        for i in 0..3 {
            let event = RealtimeEvent::Heartbeat {
                timestamp: format!("2025-11-10T00:00:{:02}Z", i),
            };
            history.store(event);
        }

        let criteria = ReplayCriteria {
            from_sequence: Some(1),
            to_sequence: None,
            from_timestamp: None,
            event_types: None,
        };

        let events = history.get_events(&criteria);
        assert!(events.len() <= 2);
    }

    #[test]
    fn test_get_last() {
        let history = EventHistory::new(10);

        for i in 0..5 {
            let event = RealtimeEvent::Heartbeat {
                timestamp: format!("2025-11-10T00:00:{:02}Z", i),
            };
            history.store(event);
        }

        let last_2 = history.get_last(2);
        assert_eq!(last_2.len(), 2);
        assert_eq!(last_2[0].sequence, 3);
        assert_eq!(last_2[1].sequence, 4);
    }
}
