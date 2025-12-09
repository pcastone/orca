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
    /// Bug list view
    BugList,
    /// Bug details view
    BugDetail,
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
            View::BugList => write!(f, "Bug List"),
            View::BugDetail => write!(f, "Bug Detail"),
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

    /// Selected bug ID (if any)
    pub selected_bug_id: Option<String>,

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
            selected_bug_id: None,
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

    /// Bug list items
    pub bugs: Vec<BugItem>,

    /// Execution events
    pub execution_events: Vec<ExecutionEvent>,

    /// Executing task/workflow ID
    pub executing_id: Option<String>,

    /// Scroll position for lists
    pub scroll: usize,

    /// Selected item index
    pub selected: usize,

    /// Pending backup operation
    pub pending_backup: bool,

    /// Pending restore operation
    pub pending_restore: bool,

    /// Pending export operation
    pub pending_export: bool,

    /// Pending import operation
    pub pending_import: bool,
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

/// Execution event
#[derive(Debug, Clone)]
pub struct ExecutionEvent {
    /// Event timestamp
    pub timestamp: String,

    /// Event type (started, progress, output, reasoning, tool_call, tool_result, completed, failed)
    pub event_type: String,

    /// Event message
    pub message: String,

    /// Event status
    pub status: String,
}

/// Bug list item
#[derive(Debug, Clone)]
pub struct BugItem {
    /// Bug ID
    pub id: String,

    /// Bug title
    pub title: String,

    /// Bug description
    pub description: Option<String>,

    /// Bug status (open, in_progress, fixed, wontfix, duplicate)
    pub status: String,

    /// Bug priority (1=Critical, 2=High, 3=Medium, 4=Low, 5=Trivial)
    pub priority: i64,

    /// Severity
    pub severity: Option<String>,

    /// Assignee
    pub assignee: Option<String>,

    /// Reporter
    pub reporter: Option<String>,

    /// Bug created at
    pub created_at: String,

    /// Bug updated at
    pub updated_at: String,
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
            bugs: Vec::new(),
            execution_events: Vec::new(),
            executing_id: None,
            scroll: 0,
            selected: 0,
            pending_backup: false,
            pending_restore: false,
            pending_export: false,
            pending_import: false,
        }
    }

    /// Handle backup operation via orchestrator API
    pub async fn handle_backup(&mut self) -> Result<()> {
        self.set_status("Creating backup...".to_string());

        let url = format!("{}/api/v1/data/backup", self.state.server_url);
        let client = reqwest::Client::new();

        match client.post(&url)
            .json(&serde_json::json!({ "include_project": true }))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let result: serde_json::Value = response.json().await
                        .map_err(|e| crate::error::AcoError::General(format!("Failed to parse response: {}", e)))?;
                    let path = result.get("path")
                        .and_then(|p| p.as_str())
                        .unwrap_or("unknown");
                    self.set_status(format!("Backup created: {}", path));
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    self.set_error(format!("Backup failed: {}", error_text));
                }
            }
            Err(e) => {
                self.set_error(format!("Backup failed: {}", e));
            }
        }

        Ok(())
    }

    /// Handle restore operation via orchestrator API
    pub async fn handle_restore(&mut self) -> Result<()> {
        self.set_status("Listing backups...".to_string());

        let url = format!("{}/api/v1/data/backups", self.state.server_url);
        let client = reqwest::Client::new();

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let backups: Vec<serde_json::Value> = response.json().await
                        .map_err(|e| crate::error::AcoError::General(format!("Failed to parse response: {}", e)))?;

                    if backups.is_empty() {
                        self.set_status("No backups available to restore".to_string());
                        return Ok(());
                    }

                    // Restore the most recent backup
                    let latest = &backups[0];
                    let backup_path = latest.get("path")
                        .and_then(|p| p.as_str())
                        .ok_or_else(|| crate::error::AcoError::General("No backup path found".to_string()))?;

                    let restore_url = format!("{}/api/v1/data/restore", self.state.server_url);
                    match client.post(&restore_url)
                        .json(&serde_json::json!({ "backup_file": backup_path }))
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                self.set_status(format!("Restored from: {}", backup_path));
                            } else {
                                let error_text = resp.text().await.unwrap_or_default();
                                self.set_error(format!("Restore failed: {}", error_text));
                            }
                        }
                        Err(e) => {
                            self.set_error(format!("Restore failed: {}", e));
                        }
                    }
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    self.set_error(format!("Failed to list backups: {}", error_text));
                }
            }
            Err(e) => {
                self.set_error(format!("Failed to list backups: {}", e));
            }
        }

        Ok(())
    }

    /// Handle export operation via orchestrator API
    pub async fn handle_export(&mut self) -> Result<()> {
        self.set_status("Exporting data...".to_string());

        let url = format!("{}/api/v1/data/export", self.state.server_url);
        let client = reqwest::Client::new();

        match client.post(&url)
            .json(&serde_json::json!({ "tables": ["all"] }))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    let content = response.text().await.unwrap_or_default();
                    let lines = content.lines().count();
                    self.set_status(format!("Export completed: {} lines exported", lines));
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    self.set_error(format!("Export failed: {}", error_text));
                }
            }
            Err(e) => {
                self.set_error(format!("Export failed: {}", e));
            }
        }

        Ok(())
    }

    /// Handle import operation via orchestrator API
    pub async fn handle_import(&mut self) -> Result<()> {
        self.set_status("Import requires SQL file content...".to_string());
        // Note: Import in TUI would need file picker dialog
        // For now, just show a message
        self.set_status("Import not available in TUI. Use CLI: aco data import <file>".to_string());
        Ok(())
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

    /// Refresh bugs from server
    pub async fn refresh_bugs(&mut self) -> Result<()> {
        debug!("Refreshing bugs");
        self.set_status("Refreshing bugs...".to_string());

        match self.grpc_client.fetch_bugs().await {
            Ok(bug_infos) => {
                self.clear_bugs();
                for bug_info in bug_infos {
                    self.add_bug(BugItem {
                        id: bug_info.id,
                        title: bug_info.title,
                        description: bug_info.description,
                        status: bug_info.status,
                        priority: bug_info.priority,
                        severity: bug_info.severity,
                        assignee: bug_info.assignee,
                        reporter: bug_info.reporter,
                        created_at: bug_info.created_at,
                        updated_at: bug_info.updated_at,
                    });
                }
                self.state.last_refresh = Instant::now();
                self.set_status(format!("Loaded {} bugs", self.bugs.len()));
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to refresh bugs: {}", e);
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

    /// Start executing a task
    pub async fn execute_task(&mut self, task_id: String) -> Result<()> {
        debug!("Starting task execution: {}", task_id);
        self.executing_id = Some(task_id.clone());
        self.execution_events.clear();
        self.set_view(View::ExecutionStream);
        self.set_status(format!("Executing task: {}", task_id));

        // Start streaming execution events (async)
        let events = self.grpc_client.execute_task(&task_id).await?;
        for event in events {
            self.add_execution_event(event);
        }

        Ok(())
    }

    /// Start executing a workflow
    pub async fn execute_workflow(&mut self, workflow_id: String) -> Result<()> {
        debug!("Starting workflow execution: {}", workflow_id);
        self.executing_id = Some(workflow_id.clone());
        self.execution_events.clear();
        self.set_view(View::ExecutionStream);
        self.set_status(format!("Executing workflow: {}", workflow_id));

        // Start streaming execution events (async)
        let events = self.grpc_client.execute_workflow(&workflow_id).await?;
        for event in events {
            self.add_execution_event(event);
        }

        Ok(())
    }

    /// Add an execution event
    pub fn add_execution_event(&mut self, event: ExecutionEvent) {
        self.execution_events.push(event);
    }

    /// Clear execution events
    pub fn clear_execution(&mut self) {
        self.execution_events.clear();
        self.executing_id = None;
    }

    /// Get executing ID
    pub fn executing_id(&self) -> Option<&str> {
        self.executing_id.as_deref()
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
            WorkflowList => BugList,
            BugList => Help,
            Help => TaskList,
            TaskDetail => WorkflowDetail,
            WorkflowDetail => BugDetail,
            BugDetail => ExecutionStream,
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
            BugList => WorkflowList,
            Help => BugList,
            TaskDetail => ExecutionStream,
            WorkflowDetail => TaskDetail,
            BugDetail => WorkflowDetail,
            ExecutionStream => BugDetail,
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
            View::BugList => {
                if let Some(bug) = self.selected_bug() {
                    self.state.selected_bug_id = Some(bug.id.clone());
                    self.set_view(View::BugDetail);
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
            View::BugDetail => {
                self.state.selected_bug_id = None;
                self.set_view(View::BugList);
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
            View::BugList => self.bugs.len(),
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

    /// Jump to first item
    pub fn first_item(&mut self) {
        self.selected = 0;
        self.update_scroll();
    }

    /// Jump to last item
    pub fn last_item(&mut self) {
        let max = match self.state.view {
            View::TaskList => self.tasks.len(),
            View::WorkflowList => self.workflows.len(),
            View::BugList => self.bugs.len(),
            _ => 0,
        };
        if max > 0 {
            self.selected = max - 1;
        }
        self.update_scroll();
    }

    /// Page up (move up by 10 items)
    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(10);
        self.update_scroll();
    }

    /// Page down (move down by 10 items)
    pub fn page_down(&mut self) {
        let max = match self.state.view {
            View::TaskList => self.tasks.len(),
            View::WorkflowList => self.workflows.len(),
            View::BugList => self.bugs.len(),
            _ => 0,
        };
        if max > 0 {
            self.selected = (self.selected + 10).min(max - 1);
        }
        self.update_scroll();
    }

    /// Go directly to a specific view
    pub fn go_to_view(&mut self, view: View) {
        self.set_view(view);
    }

    /// Get current selected task
    pub fn selected_task(&self) -> Option<&TaskItem> {
        self.tasks.get(self.selected)
    }

    /// Get current selected workflow
    pub fn selected_workflow(&self) -> Option<&WorkflowItem> {
        self.workflows.get(self.selected)
    }

    /// Get current selected bug
    pub fn selected_bug(&self) -> Option<&BugItem> {
        self.bugs.get(self.selected)
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

    /// Add a bug to the list
    pub fn add_bug(&mut self, bug: BugItem) {
        self.bugs.push(bug);
    }

    /// Clear all bugs
    pub fn clear_bugs(&mut self) {
        self.bugs.clear();
        self.selected = 0;
        self.scroll = 0;
    }

    /// Set selected bug ID
    pub fn set_selected_bug_id(&mut self, id: Option<String>) {
        self.state.selected_bug_id = id;
    }

    /// Get selected bug ID
    pub fn selected_bug_id(&self) -> Option<&str> {
        self.state.selected_bug_id.as_deref()
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

    #[test]
    fn test_first_and_last_item() {
        let mut app = App::new(test_config());

        // Add 10 tasks
        for i in 0..10 {
            app.add_task(TaskItem {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                description: "Description".to_string(),
                status: "pending".to_string(),
                task_type: "execution".to_string(),
                config: "{}".to_string(),
                metadata: "{}".to_string(),
                workspace_path: format!("/tmp/task-{}", i),
                created_at: "2024-01-01".to_string(),
                updated_at: "2024-01-01".to_string(),
            });
        }

        // Test first_item
        app.selected = 5;
        app.first_item();
        assert_eq!(app.selected, 0);

        // Test last_item
        app.last_item();
        assert_eq!(app.selected, 9);
    }

    #[test]
    fn test_page_navigation() {
        let mut app = App::new(test_config());

        // Add 30 tasks
        for i in 0..30 {
            app.add_task(TaskItem {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                description: "Description".to_string(),
                status: "pending".to_string(),
                task_type: "execution".to_string(),
                config: "{}".to_string(),
                metadata: "{}".to_string(),
                workspace_path: format!("/tmp/task-{}", i),
                created_at: "2024-01-01".to_string(),
                updated_at: "2024-01-01".to_string(),
            });
        }

        // Test page_down
        app.selected = 0;
        app.page_down();
        assert_eq!(app.selected, 10);

        app.page_down();
        assert_eq!(app.selected, 20);

        // Test page_up
        app.page_up();
        assert_eq!(app.selected, 10);

        app.page_up();
        assert_eq!(app.selected, 0);

        // Test page_down doesn't exceed bounds
        app.selected = 25;
        app.page_down();
        assert_eq!(app.selected, 29); // Last item
    }

    #[test]
    fn test_go_to_view() {
        let mut app = App::new(test_config());

        app.go_to_view(View::WorkflowList);
        assert_eq!(app.view(), View::WorkflowList);

        app.go_to_view(View::ExecutionStream);
        assert_eq!(app.view(), View::ExecutionStream);

        app.go_to_view(View::Help);
        assert_eq!(app.view(), View::Help);

        app.go_to_view(View::TaskList);
        assert_eq!(app.view(), View::TaskList);
    }

    #[test]
    fn test_empty_list_navigation() {
        let mut app = App::new(test_config());

        // Test with no items
        app.first_item();
        assert_eq!(app.selected, 0);

        app.last_item();
        assert_eq!(app.selected, 0);

        app.page_up();
        assert_eq!(app.selected, 0);

        app.page_down();
        assert_eq!(app.selected, 0);
    }
}
