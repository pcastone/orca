//! Application state management for TUI

use crate::auth::ConnectAuth;
use crate::error::Result;
use crate::tui::{TuiConfig, TuiGrpcClient};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
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

    /// Last refresh time
    pub last_refresh: Instant,

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
            last_refresh: Instant::now(),
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

    /// gRPC client for data loading
    grpc_client: TuiGrpcClient,

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

    /// Task description
    pub description: String,

    /// Task status
    pub status: String,

    /// Task type
    pub task_type: String,

    /// Task config (JSON)
    pub config: String,

    /// Task metadata (JSON)
    pub metadata: String,

    /// Workspace path
    pub workspace_path: String,

    /// Task created at
    pub created_at: String,

    /// Task updated at
    pub updated_at: String,
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
    /// Create a new app instance from config
    pub fn new(config: TuiConfig) -> Self {
        let mut state = AppState::default();
        state.server_url = config.server_url.clone();
        state.auth = ConnectAuth::None;

        let grpc_client = TuiGrpcClient::new(config.server_url);

        Self {
            state,
            grpc_client,
            tasks: Vec::new(),
            workflows: Vec::new(),
            scroll: 0,
            selected: 0,
        }
    }

    /// Refresh tasks from server
    pub async fn refresh_tasks(&mut self) -> Result<()> {
        debug!("Refreshing tasks");
        self.set_status("Refreshing tasks...".to_string());

        match self.grpc_client.fetch_tasks().await {
            Ok(task_infos) => {
                self.clear_tasks();
                for task_info in task_infos {
                    self.add_task(TaskItem {
                        id: task_info.id,
                        title: task_info.title,
                        description: task_info.description,
                        status: task_info.status,
                        task_type: task_info.task_type,
                        config: task_info.config,
                        metadata: task_info.metadata,
                        workspace_path: task_info.workspace_path,
                        created_at: task_info.created_at,
                        updated_at: task_info.updated_at,
                    });
                }
                self.state.last_refresh = Instant::now();
                self.set_status(format!("Loaded {} tasks", self.tasks.len()));
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to refresh tasks: {}", e);
                self.set_error(err_msg.clone());
                Err(e)
            }
        }
    }

    /// Refresh workflows from server
    pub async fn refresh_workflows(&mut self) -> Result<()> {
        debug!("Refreshing workflows");
        self.set_status("Refreshing workflows...".to_string());

        match self.grpc_client.fetch_workflows().await {
            Ok(workflow_infos) => {
                self.clear_workflows();
                for workflow_info in workflow_infos {
                    self.add_workflow(WorkflowItem {
                        id: workflow_info.id,
                        name: workflow_info.name,
                        status: workflow_info.status,
                        created_at: workflow_info.created_at,
                    });
                }
                self.state.last_refresh = Instant::now();
                self.set_status(format!("Loaded {} workflows", self.workflows.len()));
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to refresh workflows: {}", e);
                self.set_error(err_msg.clone());
                Err(e)
            }
        }
    }

    /// Check if data should be auto-refreshed
    pub fn should_refresh(&self) -> bool {
        // Auto-refresh every 10 seconds
        self.state.last_refresh.elapsed() > Duration::from_secs(10)
    }

    /// Check if app should quit
    pub fn should_quit(&self) -> bool {
        !self.state.running
    }

    /// Move to next view
    pub fn next_view(&mut self) {
        use View::*;
        let new_view = match self.state.view {
            TaskList => WorkflowList,
            WorkflowList => Help,
            Help => TaskList,
            TaskDetail => WorkflowDetail,
            WorkflowDetail => ExecutionStream,
            ExecutionStream => TaskDetail,
        };
        self.set_view(new_view);
    }

    /// Move to previous view
    pub fn previous_view(&mut self) {
        use View::*;
        let new_view = match self.state.view {
            TaskList => Help,
            WorkflowList => TaskList,
            Help => WorkflowList,
            TaskDetail => ExecutionStream,
            WorkflowDetail => TaskDetail,
            ExecutionStream => WorkflowDetail,
        };
        self.set_view(new_view);
    }

    /// Move selection to next item
    pub fn next_item(&mut self) {
        self.select_next();
    }

    /// Move selection to previous item
    pub fn previous_item(&mut self) {
        self.select_previous();
    }

    /// Select current item (enter detail view)
    pub fn select_item(&mut self) {
        match self.state.view {
            View::TaskList => {
                if let Some(task) = self.selected_task() {
                    self.state.selected_task_id = Some(task.id.clone());
                    self.set_view(View::TaskDetail);
                }
            }
            View::WorkflowList => {
                if let Some(workflow) = self.selected_workflow() {
                    self.state.selected_workflow_id = Some(workflow.id.clone());
                    self.set_view(View::WorkflowDetail);
                }
            }
            _ => {}
        }
    }

    /// Deselect item (return to list view)
    pub fn deselect_item(&mut self) {
        match self.state.view {
            View::TaskDetail => {
                self.state.selected_task_id = None;
                self.set_view(View::TaskList);
            }
            View::WorkflowDetail => {
                self.state.selected_workflow_id = None;
                self.set_view(View::WorkflowList);
            }
            _ => {}
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

    /// Get selected task ID
    pub fn selected_task_id(&self) -> Option<&str> {
        self.state.selected_task_id.as_deref()
    }

    /// Get selected workflow ID
    pub fn selected_workflow_id(&self) -> Option<&str> {
        self.state.selected_workflow_id.as_deref()
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
    use std::path::PathBuf;

    fn test_config() -> TuiConfig {
        TuiConfig {
            server_url: "http://localhost:50051".to_string(),
            workspace: PathBuf::from("/tmp"),
            verbose: false,
        }
    }

    #[test]
    fn test_app_creation() {
        let app = App::new(test_config());
        assert!(app.is_running());
        assert_eq!(app.view(), View::TaskList);
    }

    #[test]
    fn test_app_quit() {
        let mut app = App::new(test_config());
        app.quit();
        assert!(!app.is_running());
    }

    #[test]
    fn test_set_status() {
        let mut app = App::new(test_config());
        app.set_status("Loading...".to_string());
        assert_eq!(app.status(), "Loading...");
    }

    #[test]
    fn test_set_error() {
        let mut app = App::new(test_config());
        app.set_error("Error occurred".to_string());
        assert!(app.error().is_some());
        app.clear_error();
        assert!(app.error().is_none());
    }

    #[test]
    fn test_view_switching() {
        let mut app = App::new(test_config());
        assert_eq!(app.view(), View::TaskList);
        app.set_view(View::Help);
        assert_eq!(app.view(), View::Help);
    }

    #[test]
    fn test_view_navigation() {
        let mut app = App::new(test_config());
        assert_eq!(app.view(), View::TaskList);
        app.next_view();
        assert_eq!(app.view(), View::WorkflowList);
        app.next_view();
        assert_eq!(app.view(), View::Help);
        app.previous_view();
        assert_eq!(app.view(), View::WorkflowList);
    }

    #[test]
    fn test_task_selection() {
        let mut app = App::new(test_config());
        app.add_task(TaskItem {
            id: "task-1".to_string(),
            title: "Task 1".to_string(),
            description: "Description 1".to_string(),
            status: "pending".to_string(),
            task_type: "execution".to_string(),
            config: "{}".to_string(),
            metadata: "{}".to_string(),
            workspace_path: "/tmp/task-1".to_string(),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        });
        app.add_task(TaskItem {
            id: "task-2".to_string(),
            title: "Task 2".to_string(),
            description: "Description 2".to_string(),
            status: "completed".to_string(),
            task_type: "workflow".to_string(),
            config: "{}".to_string(),
            metadata: "{}".to_string(),
            workspace_path: "/tmp/task-2".to_string(),
            created_at: "2024-01-02".to_string(),
            updated_at: "2024-01-02".to_string(),
        });

        assert_eq!(app.selected, 0);
        app.select_next();
        assert_eq!(app.selected, 1);
        app.select_previous();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_select_and_deselect_item() {
        let mut app = App::new(test_config());
        app.add_task(TaskItem {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            description: "Test Description".to_string(),
            status: "pending".to_string(),
            task_type: "execution".to_string(),
            config: "{}".to_string(),
            metadata: "{}".to_string(),
            workspace_path: "/tmp/task-1".to_string(),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        });

        assert_eq!(app.view(), View::TaskList);
        app.select_item();
        assert_eq!(app.view(), View::TaskDetail);
        assert!(app.selected_task_id().is_some());
        app.deselect_item();
        assert_eq!(app.view(), View::TaskList);
        assert!(app.selected_task_id().is_none());
    }

    #[test]
    fn test_add_tasks_and_workflows() {
        let mut app = App::new(test_config());
        app.add_task(TaskItem {
            id: "task-1".to_string(),
            title: "Test Task".to_string(),
            description: "Test Description".to_string(),
            status: "pending".to_string(),
            task_type: "execution".to_string(),
            config: "{}".to_string(),
            metadata: "{}".to_string(),
            workspace_path: "/tmp/task-1".to_string(),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
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
        let mut app = App::new(test_config());
        app.add_task(TaskItem {
            id: "task-1".to_string(),
            title: "Task".to_string(),
            description: "Description".to_string(),
            status: "pending".to_string(),
            task_type: "execution".to_string(),
            config: "{}".to_string(),
            metadata: "{}".to_string(),
            workspace_path: "/tmp/task-1".to_string(),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
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

    #[test]
    fn test_should_refresh() {
        let app = App::new(test_config());
        // Should not refresh immediately
        assert!(!app.should_refresh());
    }
}
