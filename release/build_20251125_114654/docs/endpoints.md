# API Endpoints Reference

Complete REST API endpoint documentation for acolib.

## Base URL

```
http://localhost:8080/api
https://your-server.com/api
```

## Authentication

All endpoints (except `/health`) require JWT authentication:

```bash
# Include in Authorization header
Authorization: Bearer <jwt_token>

# Example:
curl -H "Authorization: Bearer eyJhbGc..." http://localhost:8080/api/tasks
```

## Table of Contents

- [Authentication](#authentication)
- [Tasks](#tasks)
- [Workflows](#workflows)
- [Executions](#executions)
- [Checkpoints](#checkpoints)
- [Bugs](#bugs)
- [Prompt History](#prompt-history)
- [Tools](#tools)
- [System](#system)
- [Error Codes](#error-codes)

## Authentication

### Generate API Token

**POST** `/auth/token`

Generate JWT token for API access.

**Request:**
```json
{
  "username": "user@example.com",
  "password": "secure_password"
}
```

**Response (200):**
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 3600
}
```

### List API Keys

**GET** `/auth/keys`

List all API keys for current user.

**Query Parameters:**
- `limit` (int): Max results (default: 20)
- `offset` (int): Pagination offset (default: 0)

**Response (200):**
```json
{
  "keys": [
    {
      "id": "key_1",
      "name": "my-app",
      "created_at": "2025-11-10T10:00:00Z",
      "last_used": "2025-11-10T11:30:00Z"
    }
  ]
}
```

## Tasks

### Create Task

**POST** `/tasks`

Create a new task.

**Request:**
```json
{
  "name": "Research AI frameworks",
  "description": "Find top 10 AI frameworks",
  "priority": "high",
  "tags": ["research", "ai"],
  "assignee": "john@example.com",
  "due_date": "2025-12-01"
}
```

**Response (201):**
```json
{
  "id": "task_1",
  "name": "Research AI frameworks",
  "status": "pending",
  "created_at": "2025-11-10T10:00:00Z",
  "created_by": "user@example.com"
}
```

### List Tasks

**GET** `/tasks`

List all tasks with filtering.

**Query Parameters:**
- `status` (string): Filter by status (pending, in-progress, completed, failed)
- `priority` (string): Filter by priority (low, normal, high)
- `tag` (string): Filter by tag (repeatable)
- `assignee` (string): Filter by assignee
- `limit` (int): Max results (default: 20)
- `offset` (int): Pagination offset (default: 0)
- `sort` (string): Sort field (default: created_at)
- `order` (string): Sort order (asc, desc; default: desc)

**Response (200):**
```json
{
  "tasks": [
    {
      "id": "task_1",
      "name": "Research AI frameworks",
      "status": "pending",
      "priority": "high",
      "tags": ["research", "ai"],
      "created_at": "2025-11-10T10:00:00Z"
    }
  ],
  "total": 42,
  "limit": 20,
  "offset": 0
}
```

### Get Task

**GET** `/tasks/{id}`

Get single task details.

**Response (200):**
```json
{
  "id": "task_1",
  "name": "Research AI frameworks",
  "description": "Find top 10 AI frameworks",
  "status": "pending",
  "priority": "high",
  "tags": ["research", "ai"],
  "assignee": "john@example.com",
  "due_date": "2025-12-01",
  "created_at": "2025-11-10T10:00:00Z",
  "updated_at": "2025-11-10T11:00:00Z"
}
```

### Update Task

**PUT** `/tasks/{id}`

Update task fields.

**Request:**
```json
{
  "status": "in-progress",
  "priority": "normal",
  "assignee": "jane@example.com"
}
```

**Response (200):**
```json
{
  "id": "task_1",
  "status": "in-progress",
  "priority": "normal",
  "assignee": "jane@example.com",
  "updated_at": "2025-11-10T12:00:00Z"
}
```

### Delete Task

**DELETE** `/tasks/{id}`

Delete a task.

**Response (204):** No content

## Workflows

### Create Workflow

**POST** `/workflows`

Create a new workflow.

**Request:**
```json
{
  "name": "research-workflow",
  "description": "Multi-step research workflow",
  "nodes": [
    {
      "id": "planner",
      "display_name": "Plan Research",
      "handler": "llm"
    }
  ],
  "edges": [
    {
      "source": "start",
      "target": "planner"
    }
  ]
}
```

**Response (201):**
```json
{
  "id": "workflow_1",
  "name": "research-workflow",
  "version": 1,
  "created_at": "2025-11-10T10:00:00Z"
}
```

### List Workflows

**GET** `/workflows`

List all workflows.

**Query Parameters:**
- `limit` (int): Max results (default: 20)
- `offset` (int): Pagination offset
- `sort` (string): Sort field
- `order` (string): Sort order

**Response (200):**
```json
{
  "workflows": [
    {
      "id": "workflow_1",
      "name": "research-workflow",
      "version": 1,
      "nodes_count": 4,
      "created_at": "2025-11-10T10:00:00Z"
    }
  ]
}
```

### Get Workflow

**GET** `/workflows/{id}`

Get workflow definition.

**Response (200):**
```json
{
  "id": "workflow_1",
  "name": "research-workflow",
  "version": 1,
  "nodes": [...],
  "edges": [...],
  "created_at": "2025-11-10T10:00:00Z"
}
```

### Update Workflow

**PUT** `/workflows/{id}`

Update workflow.

**Request:**
```json
{
  "description": "Updated description",
  "nodes": [...],
  "edges": [...]
}
```

**Response (200):**
```json
{
  "id": "workflow_1",
  "version": 2,
  "updated_at": "2025-11-10T12:00:00Z"
}
```

### Delete Workflow

**DELETE** `/workflows/{id}`

Delete workflow.

**Response (204):** No content

### Compile Workflow

**POST** `/workflows/{id}/compile`

Validate and compile workflow.

**Response (200):**
```json
{
  "compiled": true,
  "errors": []
}
```

## Executions

### Execute Workflow

**POST** `/executions`

Start workflow execution.

**Request:**
```json
{
  "workflow_id": "workflow_1",
  "input": {
    "topic": "machine learning"
  },
  "stream_modes": ["values", "tokens"]
}
```

**Response (201):**
```json
{
  "execution_id": "exec_1",
  "workflow_id": "workflow_1",
  "status": "running",
  "created_at": "2025-11-10T10:00:00Z"
}
```

### List Executions

**GET** `/executions`

List executions.

**Query Parameters:**
- `workflow_id` (string): Filter by workflow
- `status` (string): Filter by status
- `limit` (int): Max results
- `offset` (int): Pagination

**Response (200):**
```json
{
  "executions": [
    {
      "execution_id": "exec_1",
      "workflow_id": "workflow_1",
      "status": "running",
      "started_at": "2025-11-10T10:00:00Z"
    }
  ]
}
```

### Get Execution

**GET** `/executions/{id}`

Get execution details.

**Response (200):**
```json
{
  "execution_id": "exec_1",
  "workflow_id": "workflow_1",
  "status": "running",
  "started_at": "2025-11-10T10:00:00Z",
  "nodes": [
    {
      "node_id": "planner",
      "status": "completed",
      "duration_ms": 2500
    }
  ]
}
```

### Cancel Execution

**POST** `/executions/{id}/cancel`

Cancel running execution.

**Response (200):**
```json
{
  "execution_id": "exec_1",
  "status": "cancelled"
}
```

### Get Execution Result

**GET** `/executions/{id}/result`

Get execution final result.

**Response (200):**
```json
{
  "execution_id": "exec_1",
  "status": "completed",
  "final_state": {...},
  "duration_ms": 15000,
  "nodes_executed": 4
}
```

### Get Execution Logs

**GET** `/executions/{id}/logs`

Get execution logs.

**Query Parameters:**
- `level` (string): Filter by log level
- `limit` (int): Max logs
- `offset` (int): Pagination

**Response (200):**
```json
{
  "logs": [
    {
      "timestamp": "2025-11-10T10:00:01Z",
      "level": "info",
      "message": "Node planner started"
    }
  ]
}
```

## Checkpoints

### List Checkpoints

**GET** `/checkpoints`

List checkpoints.

**Query Parameters:**
- `execution_id` (string): Filter by execution
- `limit` (int): Max results

**Response (200):**
```json
{
  "checkpoints": [
    {
      "checkpoint_id": "cp_1",
      "execution_id": "exec_1",
      "node_id": "planner",
      "created_at": "2025-11-10T10:00:05Z"
    }
  ]
}
```

### Get Checkpoint

**GET** `/checkpoints/{id}`

Get checkpoint state.

**Response (200):**
```json
{
  "checkpoint_id": "cp_1",
  "execution_id": "exec_1",
  "node_id": "planner",
  "state": {...},
  "created_at": "2025-11-10T10:00:05Z"
}
```

### Delete Checkpoint

**DELETE** `/checkpoints/{id}`

Delete checkpoint.

**Response (204):** No content

## Bugs

### Create Bug

**POST** `/v1/bugs`

Create a new bug report.

**Request:**
```json
{
  "title": "Workflow execution fails on large inputs",
  "description": "When input exceeds 10MB, workflow times out",
  "severity": "high",
  "task_id": "task_1",
  "workflow_id": "workflow_1",
  "error_message": "Timeout after 30000ms",
  "reproduction_steps": "1. Create workflow\n2. Submit 15MB input",
  "expected_behavior": "Process completes within timeout",
  "actual_behavior": "Execution times out",
  "reporter": "user@example.com"
}
```

**Response (201):**
```json
{
  "id": "bug_1",
  "title": "Workflow execution fails on large inputs",
  "severity": "high",
  "status": "open",
  "created_at": "2025-11-10T10:00:00Z"
}
```

### List Bugs

**GET** `/v1/bugs`

List all bugs with filtering.

**Query Parameters:**
- `status` (string): Filter by status (open, in_progress, resolved, closed, wont_fix)
- `severity` (string): Filter by severity (low, medium, high, critical)
- `task_id` (string): Filter by task
- `assignee` (string): Filter by assignee
- `search` (string): Search in title and description
- `page` (int): Page number (default: 0)
- `per_page` (int): Items per page (default: 20, max: 100)

**Response (200):**
```json
{
  "data": [
    {
      "id": "bug_1",
      "title": "Workflow execution fails on large inputs",
      "severity": "high",
      "status": "open",
      "assignee": "dev@example.com",
      "created_at": "2025-11-10T10:00:00Z"
    }
  ],
  "page": 0,
  "per_page": 20,
  "total": 42
}
```

### Get Bug

**GET** `/v1/bugs/{id}`

Get bug details.

**Response (200):**
```json
{
  "id": "bug_1",
  "title": "Workflow execution fails on large inputs",
  "description": "When input exceeds 10MB, workflow times out",
  "severity": "high",
  "status": "open",
  "task_id": "task_1",
  "workflow_id": "workflow_1",
  "error_message": "Timeout after 30000ms",
  "stack_trace": "...",
  "reproduction_steps": "...",
  "expected_behavior": "...",
  "actual_behavior": "...",
  "assignee": "dev@example.com",
  "reporter": "user@example.com",
  "labels": "[\"performance\", \"timeout\"]",
  "created_at": "2025-11-10T10:00:00Z",
  "updated_at": "2025-11-10T11:00:00Z",
  "resolved_at": null
}
```

### Update Bug

**PUT** `/v1/bugs/{id}`

Update bug fields.

**Request:**
```json
{
  "status": "in_progress",
  "assignee": "dev@example.com",
  "severity": "critical"
}
```

**Response (200):**
```json
{
  "id": "bug_1",
  "status": "in_progress",
  "assignee": "dev@example.com",
  "severity": "critical",
  "updated_at": "2025-11-10T12:00:00Z"
}
```

### Delete Bug

**DELETE** `/v1/bugs/{id}`

Delete a bug.

**Response (204):** No content

### Get Bug Statistics

**GET** `/v1/bugs/stats`

Get bug statistics.

**Response (200):**
```json
{
  "total": 42,
  "open": 15,
  "in_progress": 8,
  "resolved": 19
}
```

## Prompt History

### Create Prompt History

**POST** `/v1/prompts`

Record an LLM interaction.

**Request:**
```json
{
  "provider": "anthropic",
  "model": "claude-3-opus",
  "user_prompt": "Explain quantum computing",
  "system_prompt": "You are a helpful assistant",
  "assistant_response": "Quantum computing is...",
  "task_id": "task_1",
  "session_id": "session_1",
  "input_tokens": 150,
  "output_tokens": 500,
  "cost_usd": 0.025,
  "latency_ms": 2500
}
```

**Response (201):**
```json
{
  "id": "prompt_1",
  "provider": "anthropic",
  "model": "claude-3-opus",
  "status": "completed",
  "created_at": "2025-11-10T10:00:00Z"
}
```

### List Prompt History

**GET** `/v1/prompts`

List prompt history with filtering.

**Query Parameters:**
- `task_id` (string): Filter by task
- `workflow_id` (string): Filter by workflow
- `execution_id` (string): Filter by execution
- `session_id` (string): Filter by session
- `provider` (string): Filter by provider
- `model` (string): Filter by model
- `page` (int): Page number (default: 0)
- `per_page` (int): Items per page (default: 20, max: 100)

**Response (200):**
```json
{
  "data": [
    {
      "id": "prompt_1",
      "provider": "anthropic",
      "model": "claude-3-opus",
      "input_tokens": 150,
      "output_tokens": 500,
      "cost_usd": 0.025,
      "created_at": "2025-11-10T10:00:00Z"
    }
  ],
  "page": 0,
  "per_page": 20,
  "total": 100
}
```

### Get Prompt History

**GET** `/v1/prompts/{id}`

Get prompt details.

**Response (200):**
```json
{
  "id": "prompt_1",
  "task_id": "task_1",
  "session_id": "session_1",
  "provider": "anthropic",
  "model": "claude-3-opus",
  "prompt_type": "chat",
  "system_prompt": "You are a helpful assistant",
  "user_prompt": "Explain quantum computing",
  "assistant_response": "Quantum computing is...",
  "input_tokens": 150,
  "output_tokens": 500,
  "total_tokens": 650,
  "cost_usd": 0.025,
  "latency_ms": 2500,
  "temperature": 0.7,
  "status": "completed",
  "created_at": "2025-11-10T10:00:00Z"
}
```

### Delete Prompt History

**DELETE** `/v1/prompts/{id}`

Delete a prompt history entry.

**Response (204):** No content

### Get Prompt Statistics

**GET** `/v1/prompts/stats`

Get prompt usage statistics.

**Response (200):**
```json
{
  "total_prompts": 1000,
  "total_tokens": 650000,
  "total_cost_usd": 25.50
}
```

### List Task Prompts

**GET** `/v1/tasks/{task_id}/prompts`

Get all prompts for a specific task.

**Response (200):**
```json
[
  {
    "id": "prompt_1",
    "provider": "anthropic",
    "model": "claude-3-opus",
    "user_prompt": "...",
    "assistant_response": "...",
    "created_at": "2025-11-10T10:00:00Z"
  }
]
```

### List Session Prompts

**GET** `/v1/sessions/{session_id}/prompts`

Get all prompts for a specific session.

**Response (200):**
```json
[
  {
    "id": "prompt_1",
    "provider": "anthropic",
    "model": "claude-3-opus",
    "user_prompt": "...",
    "assistant_response": "...",
    "created_at": "2025-11-10T10:00:00Z"
  }
]
```

### List Execution Checkpoints

**GET** `/v1/executions/{execution_id}/checkpoints`

Get all checkpoints for an execution.

**Response (200):**
```json
[
  {
    "id": "cp_1",
    "execution_id": "exec_1",
    "workflow_id": "workflow_1",
    "node_id": "planner",
    "superstep": 3,
    "created_at": "2025-11-10T10:00:05Z"
  }
]
```

### Get Latest Checkpoint

**GET** `/v1/executions/{execution_id}/checkpoints/latest`

Get the latest checkpoint for an execution.

**Response (200):**
```json
{
  "id": "cp_5",
  "execution_id": "exec_1",
  "workflow_id": "workflow_1",
  "node_id": "executor",
  "superstep": 7,
  "state": "{...}",
  "created_at": "2025-11-10T10:05:00Z"
}
```

## Tools

### List Tools

**GET** `/tools`

List registered tools.

**Response (200):**
```json
{
  "tools": [
    {
      "name": "web_search",
      "description": "Search the web",
      "schema": {
        "type": "object",
        "properties": {
          "query": {"type": "string"}
        }
      }
    }
  ]
}
```

### Get Tool

**GET** `/tools/{name}`

Get tool details.

**Response (200):**
```json
{
  "name": "web_search",
  "description": "Search the web",
  "schema": {...}
}
```

### Register Tool

**POST** `/tools`

Register new tool.

**Request:**
```json
{
  "name": "web_search",
  "description": "Search the web",
  "schema": {
    "type": "object",
    "properties": {
      "query": {"type": "string", "description": "Search query"}
    },
    "required": ["query"]
  }
}
```

**Response (201):**
```json
{
  "name": "web_search",
  "registered_at": "2025-11-10T10:00:00Z"
}
```

### Unregister Tool

**DELETE** `/tools/{name}`

Unregister tool.

**Response (204):** No content

## System

### Health Check

**GET** `/health`

System health status (no auth required).

**Response (200):**
```json
{
  "status": "healthy",
  "database": "connected",
  "uptime_seconds": 3600
}
```

### System Info

**GET** `/system/info`

Get system information.

**Response (200):**
```json
{
  "version": "0.2.0",
  "environment": "production",
  "uptime_seconds": 3600,
  "database_type": "postgresql"
}
```

### System Config

**GET** `/system/config`

Get system configuration (sensitive fields omitted).

**Response (200):**
```json
{
  "api_port": 8080,
  "execution_timeout": 3600,
  "max_parallel_tasks": 10
}
```

### System Metrics

**GET** `/system/metrics`

Get system metrics.

**Response (200):**
```json
{
  "cpu_usage_percent": 45.2,
  "memory_usage_bytes": 524288000,
  "active_executions": 3,
  "total_executions": 42
}
```

## Error Codes

### HTTP Status Codes

| Code | Meaning |
|------|---------|
| 200 | OK |
| 201 | Created |
| 204 | No Content |
| 400 | Bad Request |
| 401 | Unauthorized |
| 403 | Forbidden |
| 404 | Not Found |
| 409 | Conflict |
| 422 | Unprocessable Entity |
| 500 | Internal Server Error |
| 503 | Service Unavailable |

### Error Response Format

**400 Bad Request:**
```json
{
  "error": "invalid_input",
  "message": "Field 'name' is required",
  "details": {
    "field": "name"
  }
}
```

**401 Unauthorized:**
```json
{
  "error": "unauthorized",
  "message": "Invalid or missing authentication token"
}
```

**404 Not Found:**
```json
{
  "error": "not_found",
  "message": "Task with ID 'task_999' not found"
}
```

**500 Internal Server Error:**
```json
{
  "error": "internal_error",
  "message": "An unexpected error occurred",
  "request_id": "req_abc123"
}
```

## Example Usage

### Create and Execute Workflow

```bash
# 1. Create workflow
curl -X POST http://localhost:8080/api/workflows \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "example",
    "nodes": [...],
    "edges": [...]
  }'

# 2. Compile workflow
curl -X POST http://localhost:8080/api/workflows/workflow_1/compile \
  -H "Authorization: Bearer $TOKEN"

# 3. Execute workflow
curl -X POST http://localhost:8080/api/executions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "workflow_id": "workflow_1",
    "input": {"topic": "AI"}
  }'

# 4. Check result
curl http://localhost:8080/api/executions/exec_1/result \
  -H "Authorization: Bearer $TOKEN"
```

---

For complete OpenAPI specification, see [openapi.yaml](openapi.yaml).
