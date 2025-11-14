//! Application state management for TUI

use crate::auth::ConnectAuth;
use crate::error::Result;
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::debug;

/// Current view being displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    /// Task list view
    TaskList,
    /// Task details view
    TaskDetail,
    /// Workflow list view
    WorkflowList,
    /// Workflow details view
    WorkflowDetail,
    /// Execution streaming view
    ExecutionStream,
    /// Help/about view
    Help,
}

impl Default for View {
    fn default() -> Self {
        View::TaskList
    }
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            View::TaskList => write!(f, "Task List"),
            View::TaskDetail => write!(f, "Task Detail"),
            View::WorkflowList => write!(f, "Workflow List"),
            View::WorkflowDetail => write!(f, "Workflow Detail"),
            View::ExecutionStream => write!(f, "Execution Stream"),
            View::Help => write!(f, "Help"),
        }
    }
}

/// Application state
#[derive(Debug)]
pub struct AppState {
    /// Current view being displayed
    pub view: View,

    /// Selected task ID (if any)
    pub selected_task_id: Option<String>,

    /// Selected workflow ID (if any)
    pub selected_workflow_id: Option<String>,

    /// Server URL
    pub server_url: String,

    /// Authentication mode
    pub auth: ConnectAuth,

    /// Last update time
    pub last_update: Instant,

    /// Is the app running
    pub running: bool,

    /// Error message (if any)
    pub error: Option<String>,

    /// Status message
    pub status: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            view: View::default(),
            selected_task_id: None,
            selected_workflow_id: None,
            server_url: "http://localhost:50051".to_string(),
            auth: ConnectAuth::None,
            last_update: Instant::now(),
            running: true,
            error: None,
            status: "Ready".to_string(),
        }
    }
}

/// TUI Application
#[derive(Debug)]
pub struct App {
    /// Application state
    state: AppState,

    /// Task list items
    pub tasks: Vec<TaskItem>,

    /// Workflow list items
    pub workflows: Vec<WorkflowItem>,

    /// Scroll position for lists
    pub scroll: usize,

    /// Selected item index
    pub selected: usize,
}

/// Task list item
#[derive(Debug, Clone)]
pub struct TaskItem {
    /// Task ID
    pub id: String,

    /// Task title
    pub title: String,

    /// Task status
    pub status: String,

    /// Task created at
    pub created_at: String,
}

/// Workflow list item
#[derive(Debug, Clone)]
pub struct WorkflowItem {
    /// Workflow ID
    pub id: String,

    /// Workflow name
    pub name: String,

    /// Workflow status
    pub status: String,

    /// Workflow created at
    pub created_at: String,
}

impl App {
    /// Create a new app instance
    pub fn new(server_url: String, auth: ConnectAuth) -> Self {
        let mut state = AppState::default();
        state.server_url = server_url;
        state.auth = auth;

        Self {
            state,
            tasks: Vec::new(),
            workflows: Vec::new(),
            scroll: 0,
            selected: 0,
        }
    }

    /// Get current view
    pub fn view(&self) -> View {
        self.state.view
    }

    /// Set current view
    pub fn set_view(&mut self, view: View) {
        debug!("Switching view to: {}", view);
        self.state.view = view;
        self.selected = 0;
        self.scroll = 0;
    }

    /// Check if app is running
    pub fn is_running(&self) -> bool {
        self.state.running
    }

    /// Stop the app
    pub fn quit(&mut self) {
        self.state.running = false;
    }

    /// Set status message
    pub fn set_status(&mut self, msg: String) {
        self.state.status = msg;
        self.state.last_update = Instant::now();
    }

    /// Set error message
    pub fn set_error(&mut self, err: String) {
        self.state.error = Some(err);
        self.state.last_update = Instant::now();
    }

    /// Clear error message
    pub fn clear_error(&mut self) {
        self.state.error = None;
    }

    /// Get status message
    pub fn status(&self) -> &str {
        &self.state.status
    }

    /// Get error message
    pub fn error(&self) -> Option<&str> {
        self.state.error.as_deref()
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.update_scroll();
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let max = match self.state.view {
            View::TaskList => self.tasks.len(),
            View::WorkflowList => self.workflows.len(),
            _ => 0,
        };

        if self.selected < max.saturating_sub(1) {
            self.selected += 1;
        }
        self.update_scroll();
    }

    /// Update scroll position based on selection
    fn update_scroll(&mut self) {
        let max_height = 10; // Typical list height
        if self.selected < self.scroll {
            self.scroll = self.selected;
        } else if self.selected >= self.scroll + max_height {
            self.scroll = self.selected.saturating_sub(max_height - 1);
        }
    }

    /// Get current selected task
    pub fn selected_task(&self) -> Option<&TaskItem> {
        self.tasks.get(self.selected)
    }

    /// Get current selected workflow
    pub fn selected_workflow(&self) -> Option<&WorkflowItem> {
        self.workflows.get(self.selected)
    }

    /// Add a task to the list
    pub fn add_task(&mut self, task: TaskItem) {
        self.tasks.push(task);
    }

    /// Add a workflow to the list
    pub fn add_workflow(&mut self, workflow: WorkflowItem) {
        self.workflows.push(workflow);
    }

    /// Clear all tasks
    pub fn clear_tasks(&mut self) {
        self.tasks.clear();
        self.selected = 0;
        self.scroll = 0;
    }

    /// Clear all workflows
    pub fn clear_workflows(&mut self) {
        self.workflows.clear();
        self.selected = 0;
        self.scroll = 0;
    }

    /// Get server URL
    pub fn server_url(&self) -> &str {
        &self.state.server_url
    }

    /// Get authentication mode
    pub fn auth(&self) -> &ConnectAuth {
        &self.state.auth
    }

    /// Get view area rectangle (placeholder for UI rendering)
    pub fn view_area(&self) -> Rect {
        Rect::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new("http://localhost:50051".to_string(), ConnectAuth::None);
        assert!(app.is_running());
        assert_eq!(app.view(), View::TaskList);
    }

    #[test]
    fn test_app_quit() {
        let mut app = App::new("http://localhost:50051".to_string(), ConnectAuth::None);
        app.quit();
        assert!(!app.is_running());
    }

    #[test]
    fn test_set_status() {
        let mut app = App::new("http://localhost:50051".to_string(), ConnectAuth::None);
        app.set_status("Loading...".to_string());
        assert_eq!(app.status(), "Loading...");
    }

    #[test]
    fn test_set_error() {
        let mut app = App::new("http://localhost:50051".to_string(), ConnectAuth::None);
        app.set_error("Error occurred".to_string());
        assert!(app.error().is_some());
        app.clear_error();
        assert!(app.error().is_none());
    }

    #[test]
    fn test_view_switching() {
        let mut app = App::new("http://localhost:50051".to_string(), ConnectAuth::None);
        assert_eq!(app.view(), View::TaskList);
        app.set_view(View::Help);
        assert_eq!(app.view(), View::Help);
    }

    #[test]
    fn test_task_selection() {
        let mut app = App::new("http://localhost:50051".to_string(), ConnectAuth::None);
        app.add_task(TaskItem {
            id: "task-1".to_string(),
            title: "Task 1".to_string(),
            status: "pending".to_string(),
            created_at: "2024-01-01".to_string(),
        });
        app.add_task(TaskItem {
            id: "task-2".to_string(),
            title: "Task 2".to_string(),
            status: "completed".to_string(),
            created_at: "2024-01-02".to_string(),
        });

        assert_eq!(app.selected, 0);
        app.select_next();
        assert_eq!(app.selected, 1);
        app.select_previous();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_add_tasks_and_workflows() {
        let mut app = App::new("http://localhost:50051".to_string(), ConnectAuth::None);
        app.add_task(TaskItem {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            status: "pending".to_string(),
            created_at: "2024-01-01".to_string(),
        });
        app.add_workflow(WorkflowItem {
            id: "wf-1".to_string(),
            name: "Test Workflow".to_string(),
            status: "draft".to_string(),
            created_at: "2024-01-01".to_string(),
        });

        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.workflows.len(), 1);
    }

    #[test]
    fn test_clear_lists() {
        let mut app = App::new("http://localhost:50051".to_string(), ConnectAuth::None);
        app.add_task(TaskItem {
            id: "task-1".to_string(),
            title: "Task".to_string(),
            status: "pending".to_string(),
            created_at: "2024-01-01".to_string(),
        });
        assert_eq!(app.tasks.len(), 1);
        app.clear_tasks();
        assert_eq!(app.tasks.len(), 0);
    }

    #[test]
    fn test_view_display() {
        assert_eq!(View::TaskList.to_string(), "Task List");
        assert_eq!(View::Help.to_string(), "Help");
    }
}
