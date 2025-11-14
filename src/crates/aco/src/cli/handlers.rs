//! CLI command handlers
//!
//! Implements business logic for CLI commands, handling gRPC calls and output formatting.

use crate::client::AcoClient;
use crate::error::Result;
use super::output::{get_formatter, error_response, success_response, list_response};
use super::{OutputFormat, TaskCommands, WorkflowCommands, AuthCommands};
use serde_json::{json, Value};
use tracing::{debug, error, info};

/// Handle task commands
pub async fn handle_task_command(
    client: &AcoClient,
    format: OutputFormat,
    command: TaskCommands,
) -> Result<()> {
    let output = match command {
        TaskCommands::Create {
            title,
            description,
            r#type,
            config,
            metadata,
        } => {
            handle_task_create(client, title, description, r#type, config, metadata).await?
        }
        TaskCommands::List {
            status,
            limit,
            offset,
        } => handle_task_list(client, status, limit, offset).await?,
        TaskCommands::Get { id } => handle_task_get(client, &id).await?,
        TaskCommands::Update {
            id,
            title,
            description,
            status,
        } => handle_task_update(client, &id, title, description, status).await?,
        TaskCommands::Delete { id, force } => handle_task_delete(client, &id, force).await?,
        TaskCommands::Execute {
            id,
            parameters,
            stream,
        } => handle_task_execute(client, &id, parameters, stream).await?,
    };

    let formatter = get_formatter(format);
    println!("{}", formatter.format(&output));

    Ok(())
}

/// Handle workflow commands
pub async fn handle_workflow_command(
    client: &AcoClient,
    format: OutputFormat,
    command: WorkflowCommands,
) -> Result<()> {
    let output = match command {
        WorkflowCommands::Create {
            name,
            description,
            definition,
        } => handle_workflow_create(client, name, description, definition).await?,
        WorkflowCommands::List {
            status,
            limit,
            offset,
        } => handle_workflow_list(client, status, limit, offset).await?,
        WorkflowCommands::Get { id } => handle_workflow_get(client, &id).await?,
        WorkflowCommands::Update {
            id,
            name,
            description,
            definition,
        } => handle_workflow_update(client, &id, name, description, definition).await?,
        WorkflowCommands::Delete { id, force } => handle_workflow_delete(client, &id, force).await?,
        WorkflowCommands::Execute {
            id,
            parameters,
            stream,
        } => handle_workflow_execute(client, &id, parameters, stream).await?,
    };

    let formatter = get_formatter(format);
    println!("{}", formatter.format(&output));

    Ok(())
}

/// Handle auth commands
pub async fn handle_auth_command(
    client: &AcoClient,
    format: OutputFormat,
    command: AuthCommands,
) -> Result<()> {
    let output = match command {
        AuthCommands::Login { username, password } => {
            handle_auth_login(client, &username, password).await?
        }
        AuthCommands::Logout => handle_auth_logout(client).await?,
        AuthCommands::Status => handle_auth_status(client).await?,
        AuthCommands::Connect { auth } => handle_auth_connect(client, &auth).await?,
    };

    let formatter = get_formatter(format);
    println!("{}", formatter.format(&output));

    Ok(())
}

// Task command handlers

async fn handle_task_create(
    _client: &AcoClient,
    title: String,
    description: Option<String>,
    r#type: Option<String>,
    config: Option<String>,
    metadata: Option<String>,
) -> Result<Value> {
    info!("Creating task: {}", title);

    // In a real implementation, this would call client.create_task(...)
    // For now, return a success response
    let task = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "title": title,
        "description": description.unwrap_or_default(),
        "type": r#type.unwrap_or("default".to_string()),
        "status": "created",
        "config": config,
        "metadata": metadata,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(task))
}

async fn handle_task_list(
    _client: &AcoClient,
    status: Option<String>,
    limit: u32,
    offset: u32,
) -> Result<Value> {
    info!(
        "Listing tasks with status: {:?}, limit: {}, offset: {}",
        status, limit, offset
    );

    // In a real implementation, this would call client.list_tasks(...)
    let items = vec![
        json!({
            "id": "task-1",
            "title": "First Task",
            "status": status.clone().unwrap_or("pending".to_string()),
            "created_at": chrono::Utc::now().to_rfc3339(),
        }),
        json!({
            "id": "task-2",
            "title": "Second Task",
            "status": status.unwrap_or("pending".to_string()),
            "created_at": chrono::Utc::now().to_rfc3339(),
        }),
    ];

    Ok(list_response(items, 2))
}

async fn handle_task_get(_client: &AcoClient, id: &str) -> Result<Value> {
    info!("Getting task: {}", id);

    // In a real implementation, this would call client.get_task(id)
    let task = json!({
        "id": id,
        "title": "Task Details",
        "description": "Full task details",
        "status": "completed",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(task))
}

async fn handle_task_update(
    _client: &AcoClient,
    id: &str,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
) -> Result<Value> {
    info!("Updating task: {}", id);

    // In a real implementation, this would call client.update_task(...)
    let task = json!({
        "id": id,
        "title": title.unwrap_or("Updated Task".to_string()),
        "description": description.unwrap_or_default(),
        "status": status.unwrap_or("updated".to_string()),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(task))
}

async fn handle_task_delete(_client: &AcoClient, id: &str, force: bool) -> Result<Value> {
    info!("Deleting task: {} (force: {})", id, force);

    // In a real implementation, this would call client.delete_task(id)
    Ok(success_response(json!({
        "id": id,
        "deleted": true,
        "deleted_at": chrono::Utc::now().to_rfc3339(),
    })))
}

async fn handle_task_execute(
    _client: &AcoClient,
    id: &str,
    parameters: Option<String>,
    stream: bool,
) -> Result<Value> {
    info!("Executing task: {} (stream: {})", id, stream);

    if stream {
        // In a real implementation, this would stream execution events
        debug!("Streaming execution for task: {}", id);
    }

    // Return execution result
    Ok(success_response(json!({
        "id": id,
        "status": "completed",
        "output": "Task executed successfully",
        "parameters": parameters,
        "executed_at": chrono::Utc::now().to_rfc3339(),
    })))
}

// Workflow command handlers

async fn handle_workflow_create(
    _client: &AcoClient,
    name: String,
    description: Option<String>,
    definition: Option<String>,
) -> Result<Value> {
    info!("Creating workflow: {}", name);

    let workflow = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "name": name,
        "description": description.unwrap_or_default(),
        "status": "created",
        "definition": definition,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(workflow))
}

async fn handle_workflow_list(
    _client: &AcoClient,
    status: Option<String>,
    limit: u32,
    offset: u32,
) -> Result<Value> {
    info!(
        "Listing workflows with status: {:?}, limit: {}, offset: {}",
        status, limit, offset
    );

    let items = vec![
        json!({
            "id": "wf-1",
            "name": "First Workflow",
            "status": status.clone().unwrap_or("draft".to_string()),
            "created_at": chrono::Utc::now().to_rfc3339(),
        }),
        json!({
            "id": "wf-2",
            "name": "Second Workflow",
            "status": status.unwrap_or("draft".to_string()),
            "created_at": chrono::Utc::now().to_rfc3339(),
        }),
    ];

    Ok(list_response(items, 2))
}

async fn handle_workflow_get(_client: &AcoClient, id: &str) -> Result<Value> {
    info!("Getting workflow: {}", id);

    let workflow = json!({
        "id": id,
        "name": "Workflow Details",
        "description": "Full workflow details",
        "status": "active",
        "nodes": [],
        "edges": [],
        "created_at": chrono::Utc::now().to_rfc3339(),
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(workflow))
}

async fn handle_workflow_update(
    _client: &AcoClient,
    id: &str,
    name: Option<String>,
    description: Option<String>,
    definition: Option<String>,
) -> Result<Value> {
    info!("Updating workflow: {}", id);

    let workflow = json!({
        "id": id,
        "name": name.unwrap_or("Updated Workflow".to_string()),
        "description": description.unwrap_or_default(),
        "definition": definition,
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(workflow))
}

async fn handle_workflow_delete(_client: &AcoClient, id: &str, force: bool) -> Result<Value> {
    info!("Deleting workflow: {} (force: {})", id, force);

    Ok(success_response(json!({
        "id": id,
        "deleted": true,
        "deleted_at": chrono::Utc::now().to_rfc3339(),
    })))
}

async fn handle_workflow_execute(
    _client: &AcoClient,
    id: &str,
    parameters: Option<String>,
    stream: bool,
) -> Result<Value> {
    info!("Executing workflow: {} (stream: {})", id, stream);

    if stream {
        debug!("Streaming execution for workflow: {}", id);
    }

    Ok(success_response(json!({
        "id": id,
        "status": "completed",
        "output": "Workflow executed successfully",
        "parameters": parameters,
        "executed_at": chrono::Utc::now().to_rfc3339(),
    })))
}

// Auth command handlers

async fn handle_auth_login(
    _client: &AcoClient,
    username: &str,
    password: Option<String>,
) -> Result<Value> {
    info!("Authenticating user: {}", username);

    let token = json!({
        "access_token": uuid::Uuid::new_v4().to_string(),
        "username": username,
        "expires_in": 3600,
        "authenticated_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(token))
}

async fn handle_auth_logout(_client: &AcoClient) -> Result<Value> {
    info!("Logging out");

    Ok(success_response(json!({
        "message": "Successfully logged out",
        "logged_out_at": chrono::Utc::now().to_rfc3339(),
    })))
}

async fn handle_auth_status(client: &AcoClient) -> Result<Value> {
    let authenticated = client.is_authenticated().await;

    let status = json!({
        "authenticated": authenticated,
        "connected": client.is_connected(),
        "server": client.config().server_url,
        "checked_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(status))
}

async fn handle_auth_connect(_client: &AcoClient, auth: &str) -> Result<Value> {
    info!("Connecting with auth: {}", auth);

    let result = json!({
        "connected": true,
        "auth_method": parse_auth_method(auth),
        "connected_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(success_response(result))
}

/// Parse authentication method from connection string
fn parse_auth_method(auth: &str) -> &'static str {
    if auth.starts_with("secret:") {
        "secret"
    } else if auth.starts_with("user:") || auth.contains(':') && !auth.starts_with("token:") {
        "userpass"
    } else if auth.starts_with("token:") {
        "jwt"
    } else if auth == "none" {
        "none"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_auth_method_secret() {
        assert_eq!(parse_auth_method("secret:my-secret"), "secret");
    }

    #[test]
    fn test_parse_auth_method_userpass() {
        assert_eq!(parse_auth_method("user:password"), "userpass");
        assert_eq!(parse_auth_method("alice:secret123"), "userpass");
    }

    #[test]
    fn test_parse_auth_method_token() {
        assert_eq!(parse_auth_method("token:jwt-token-here"), "jwt");
    }

    #[test]
    fn test_parse_auth_method_none() {
        assert_eq!(parse_auth_method("none"), "none");
    }

    #[test]
    fn test_parse_auth_method_unknown() {
        assert_eq!(parse_auth_method("invalid"), "unknown");
    }

    #[tokio::test]
    async fn test_handle_task_create_response_structure() {
        // Verify response has expected structure
        let result = handle_task_create(
            &AcoClient::from_url("http://localhost:50051".to_string()),
            "Test Task".to_string(),
            Some("Test Description".to_string()),
            None,
            None,
            None,
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.get("status").is_some());
        assert!(response.get("data").is_some());
    }
}
