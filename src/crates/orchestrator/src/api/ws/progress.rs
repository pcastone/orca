//! Task progress streaming
//!
//! Provides real-time task progress streaming with updates and status tracking.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Task progress data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskProgress {
    /// Task ID
    pub task_id: String,
    /// Progress percentage (0-100)
    pub progress: u32,
    /// Progress message
    pub message: String,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
}

impl TaskProgress {
    /// Create new task progress
    pub fn new(task_id: String, progress: u32, message: String) -> Self {
        Self {
            task_id,
            progress: std::cmp::min(progress, 100),
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Validate progress percentage
    pub fn is_valid(&self) -> bool {
        self.progress <= 100 && !self.task_id.is_empty()
    }
}

/// Progress event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Event type
    pub event_type: String,
    /// Task progress data
    pub data: TaskProgress,
    /// Event timestamp
    pub timestamp: String,
}

impl ProgressEvent {
    /// Create new progress event
    pub fn new(progress: TaskProgress) -> Self {
        Self {
            event_type: "task.progress".to_string(),
            data: progress,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Task progress tracker
pub struct TaskProgressTracker {
    /// Task ID
    task_id: String,
    /// Current progress percentage
    progress: Arc<AtomicU64>,
    /// Last update timestamp
    last_update: Arc<parking_lot::Mutex<i64>>,
    /// Last update message
    last_message: Arc<parking_lot::Mutex<String>>,
}

impl TaskProgressTracker {
    /// Create new progress tracker
    pub fn new(task_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            task_id,
            progress: Arc::new(AtomicU64::new(0)),
            last_update: Arc::new(parking_lot::Mutex::new(now)),
            last_message: Arc::new(parking_lot::Mutex::new(String::new())),
        }
    }

    /// Update progress
    pub fn update(&self, progress: u32, message: String) -> TaskProgress {
        let progress = std::cmp::min(progress, 100) as u64;
        self.progress.store(progress, Ordering::Relaxed);

        let mut last_msg = self.last_message.lock();
        *last_msg = message.clone();

        let mut last_update = self.last_update.lock();
        *last_update = chrono::Utc::now().timestamp();

        TaskProgress::new(self.task_id.clone(), progress as u32, message)
    }

    /// Get current progress
    pub fn get_progress(&self) -> u32 {
        self.progress.load(Ordering::Relaxed) as u32
    }

    /// Get last message
    pub fn get_message(&self) -> String {
        self.last_message.lock().clone()
    }

    /// Check if progress has changed since timestamp
    pub fn has_changed_since(&self, timestamp: i64) -> bool {
        let last_update = self.last_update.lock();
        *last_update > timestamp
    }

    /// Check if task is complete
    pub fn is_complete(&self) -> bool {
        self.progress.load(Ordering::Relaxed) >= 100
    }

    /// Get progress snapshot
    pub fn snapshot(&self) -> TaskProgress {
        TaskProgress::new(
            self.task_id.clone(),
            self.get_progress(),
            self.get_message(),
        )
    }
}

/// Global progress manager
pub struct ProgressManager {
    /// Trackers per task
    trackers: Arc<dashmap::DashMap<String, TaskProgressTracker>>,
}

impl ProgressManager {
    /// Create new progress manager
    pub fn new() -> Self {
        Self {
            trackers: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Get or create tracker for a task
    pub fn get_tracker(&self, task_id: &str) -> TaskProgressTracker {
        self.trackers
            .entry(task_id.to_string())
            .or_insert_with(|| TaskProgressTracker::new(task_id.to_string()))
            .clone()
    }

    /// Update task progress
    pub fn update(&self, task_id: &str, progress: u32, message: String) -> TaskProgress {
        let tracker = self.get_tracker(task_id);
        tracker.update(progress, message)
    }

    /// Get current progress for a task
    pub fn get_progress(&self, task_id: &str) -> Option<TaskProgress> {
        self.trackers.get(task_id).map(|tracker| tracker.snapshot())
    }

    /// Get all task progresses
    pub fn get_all_progress(&self) -> Vec<TaskProgress> {
        self.trackers
            .iter()
            .map(|entry| entry.value().snapshot())
            .collect()
    }

    /// Remove task tracker
    pub fn remove_task(&self, task_id: &str) {
        self.trackers.remove(task_id);
    }

    /// Get all completed tasks
    pub fn get_completed_tasks(&self) -> Vec<String> {
        self.trackers
            .iter()
            .filter(|entry| entry.value().is_complete())
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get active tasks (not completed)
    pub fn get_active_tasks(&self) -> Vec<String> {
        self.trackers
            .iter()
            .filter(|entry| !entry.value().is_complete())
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TaskProgressTracker {
    fn clone(&self) -> Self {
        Self {
            task_id: self.task_id.clone(),
            progress: Arc::clone(&self.progress),
            last_update: Arc::clone(&self.last_update),
            last_message: Arc::clone(&self.last_message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_progress_creation() {
        let progress = TaskProgress::new(
            "task1".to_string(),
            50,
            "Processing...".to_string(),
        );
        assert_eq!(progress.task_id, "task1");
        assert_eq!(progress.progress, 50);
        assert!(progress.is_valid());
    }

    #[test]
    fn test_task_progress_clamping() {
        let progress = TaskProgress::new(
            "task1".to_string(),
            150,
            "Done".to_string(),
        );
        assert_eq!(progress.progress, 100);
    }

    #[test]
    fn test_progress_event() {
        let progress = TaskProgress::new(
            "task1".to_string(),
            50,
            "Half done".to_string(),
        );
        let event = ProgressEvent::new(progress);
        assert_eq!(event.event_type, "task.progress");
        assert_eq!(event.data.progress, 50);
    }

    #[test]
    fn test_progress_tracker() {
        let tracker = TaskProgressTracker::new("task1".to_string());
        assert_eq!(tracker.get_progress(), 0);

        tracker.update(25, "Quarter done".to_string());
        assert_eq!(tracker.get_progress(), 25);

        tracker.update(100, "Complete".to_string());
        assert!(tracker.is_complete());
    }

    #[test]
    fn test_progress_manager() {
        let manager = ProgressManager::new();

        manager.update("task1", 50, "Half done".to_string());
        manager.update("task2", 100, "Complete".to_string());

        assert_eq!(manager.get_all_progress().len(), 2);
        assert_eq!(manager.get_completed_tasks().len(), 1);
        assert_eq!(manager.get_active_tasks().len(), 1);
    }

    #[test]
    fn test_progress_changed_since() {
        let tracker = TaskProgressTracker::new("task1".to_string());
        let before = chrono::Utc::now().timestamp();

        std::thread::sleep(std::time::Duration::from_millis(10));
        tracker.update(50, "Updated".to_string());

        assert!(tracker.has_changed_since(before));
    }
}
