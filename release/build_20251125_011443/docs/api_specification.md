# API Specification

**Project**: acolib v0.2.0
**API Version**: v1
**Base URL**: `http://localhost:8080/api/v1`
**Protocol**: HTTP/REST + WebSocket
**Authentication**: None (localhost trust)

---

## Table of Contents

1. [Overview](#overview)
2. [REST API Endpoints](#rest-api-endpoints)
   - [Tasks API](#tasks-api)
   - [Workflows API](#workflows-api)
   - [Tool Executions API](#tool-executions-api)
   - [System API](#system-api)
3. [WebSocket Protocol](#websocket-protocol)
4. [Data Models](#data-models)
5. [Error Handling](#error-handling)
6. [Rate Limiting](#rate-limiting)

---

## Overview

### Design Principles

- **RESTful**: Standard HTTP verbs (GET, POST, PUT, DELETE)
- **JSON**: All request/response bodies in JSON format
- **Stateless**: No server-side session state (sessions in database)
- **Idempotent**: PUT and DELETE operations are idempotent
- **Paginated**: List endpoints support pagination
- **Real-time**: WebSocket for live updates

### HTTP Headers

**Required**:
```
Content-Type: application/json
```

**Optional**:
```
Accept: application/json
X-Request-ID: <uuid>  # For request tracing
```

### HTTP Status Codes

| Code | Meaning | Usage |
|------|---------|-------|
| 200 | OK | Successful GET, PUT |
| 201 | Created | Successful POST (resource created) |
| 202 | Accepted | Async operation started |
| 204 | No Content | Successful DELETE |
| 400 | Bad Request | Invalid input, validation error |
| 404 | Not Found | Resource doesn't exist |
| 409 | Conflict | Resource state conflict |
| 500 | Internal Server Error | Server error |

---

## REST API Endpoints

### Tasks API

#### POST `/api/v1/tasks` - Create Task

Create a new task for execution.

**Request Body**:
```json
{
  "title": "Read main.rs",
  "description": "Read the main.rs file contents",
  "task_type": "file_operation",
  "config": {
    "tool": "file_read",
    "args": {
      "path": "src/main.rs"
    }
  },
  "workspace_path": "/home/user/project",
  "metadata": {
    "priority": "high",
    "tags": ["filesystem"]
  }
}
```

**Response** (201 Created):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Read main.rs",
  "description": "Read the main.rs file contents",
  "task_type": "file_operation",
  "status": "pending",
  "config": {
    "tool": "file_read",
    "args": {
      "path": "src/main.rs"
    }
  },
  "metadata": {
    "priority": "high",
    "tags": ["filesystem"]
  },
  "workspace_path": "/home/user/project",
  "created_at": "2025-01-15T10:30:00Z",
  "updated_at": "2025-01-15T10:30:00Z",
  "started_at": null,
  "completed_at": null,
  "error": null
}
```

**Validation Rules**:
- `title`: Required, 1-200 characters
- `task_type`: Required, one of: `file_operation`, `git_operation`, `workflow`, `analysis`, `shell_operation`
- `config`: Required, valid JSON object
- `workspace_path`: Optional, absolute path

**cURL Example**:
```bash
curl -X POST http://localhost:8080/api/v1/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Read main.rs",
    "task_type": "file_operation",
    "config": {
      "tool": "file_read",
      "args": {"path": "src/main.rs"}
    }
  }'
```

---

#### GET `/api/v1/tasks/:id` - Get Task

Retrieve a task by ID.

**Path Parameters**:
- `id`: Task UUID

**Response** (200 OK):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Read main.rs",
  "status": "completed",
  "config": {...},
  "result": {
    "content": "fn main() { ... }",
    "lines": 150
  },
  "created_at": "2025-01-15T10:30:00Z",
  "completed_at": "2025-01-15T10:30:05Z"
}
```

**Error** (404 Not Found):
```json
{
  "error": "Task not found",
  "code": "TASK_NOT_FOUND",
  "details": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**cURL Example**:
```bash
curl http://localhost:8080/api/v1/tasks/550e8400-e29b-41d4-a716-446655440000
```

---

#### GET `/api/v1/tasks` - List Tasks

List tasks with optional filtering and pagination.

**Query Parameters**:
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `status` | string | - | Filter by status (comma-separated) |
| `task_type` | string | - | Filter by task type |
| `workspace_path` | string | - | Filter by workspace |
| `limit` | integer | 20 | Number of results (max 100) |
| `offset` | integer | 0 | Offset for pagination |
| `sort` | string | `created_at:desc` | Sort field and direction |

**Valid sort fields**: `created_at`, `updated_at`, `status`, `title`

**Response** (200 OK):
```json
{
  "tasks": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Read main.rs",
      "status": "completed",
      "created_at": "2025-01-15T10:30:00Z"
    },
    ...
  ],
  "total": 150,
  "limit": 20,
  "offset": 0,
  "has_more": true
}
```

**cURL Examples**:
```bash
# Get pending tasks
curl "http://localhost:8080/api/v1/tasks?status=pending&limit=10"

# Get tasks by type
curl "http://localhost:8080/api/v1/tasks?task_type=file_operation"

# Pagination
curl "http://localhost:8080/api/v1/tasks?limit=20&offset=40"

# Multiple statuses
curl "http://localhost:8080/api/v1/tasks?status=pending,running"
```

---

#### PUT `/api/v1/tasks/:id` - Update Task

Update task metadata (not status).

**Path Parameters**:
- `id`: Task UUID

**Request Body** (partial update):
```json
{
  "title": "Updated title",
  "description": "Updated description",
  "metadata": {
    "priority": "low"
  }
}
```

**Response** (200 OK):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Updated title",
  "description": "Updated description",
  "updated_at": "2025-01-15T10:35:00Z",
  ...
}
```

**Note**: Cannot update `status` directly - use `/execute` or `/cancel` endpoints.

**cURL Example**:
```bash
curl -X PUT http://localhost:8080/api/v1/tasks/550e8400-e29b-41d4-a716-446655440000 \
  -H "Content-Type: application/json" \
  -d '{"title": "Updated title"}'
```

---

#### DELETE `/api/v1/tasks/:id` - Delete Task

Delete a task and its execution history.

**Path Parameters**:
- `id`: Task UUID

**Response** (204 No Content)

**Error** (409 Conflict):
```json
{
  "error": "Cannot delete running task",
  "code": "TASK_RUNNING",
  "details": {
    "task_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "running"
  }
}
```

**cURL Example**:
```bash
curl -X DELETE http://localhost:8080/api/v1/tasks/550e8400-e29b-41d4-a716-446655440000
```

---

#### POST `/api/v1/tasks/:id/execute` - Execute Task

Start task execution.

**Path Parameters**:
- `id`: Task UUID

**Response** (202 Accepted):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "started_at": "2025-01-15T10:30:05Z"
}
```

**Error** (409 Conflict):
```json
{
  "error": "Task already running",
  "code": "TASK_ALREADY_RUNNING"
}
```

**cURL Example**:
```bash
curl -X POST http://localhost:8080/api/v1/tasks/550e8400-e29b-41d4-a716-446655440000/execute
```

---

#### POST `/api/v1/tasks/:id/cancel` - Cancel Task

Cancel a running task.

**Path Parameters**:
- `id`: Task UUID

**Response** (200 OK):
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "cancelled",
  "completed_at": "2025-01-15T10:30:10Z"
}
```

**cURL Example**:
```bash
curl -X POST http://localhost:8080/api/v1/tasks/550e8400-e29b-41d4-a716-446655440000/cancel
```

---

### Workflows API

#### POST `/api/v1/workflows` - Create Workflow

Create a multi-step workflow.

**Request Body**:
```json
{
  "name": "Multi-file refactoring",
  "description": "Refactor error handling",
  "pattern_type": "Plan-Execute",
  "config": {
    "llm_provider": "ollama",
    "model": "llama3",
    "max_retries": 3
  },
  "tasks": [
    {
      "title": "Read file1",
      "tool": "file_read",
      "args": {"path": "file1.rs"}
    },
    {
      "title": "Read file2",
      "tool": "file_read",
      "args": {"path": "file2.rs"}
    },
    {
      "title": "Apply changes",
      "tool": "file_write",
      "args": {"path": "file1.rs", "content": "..."},
      "depends_on": [0, 1]
    }
  ]
}
```

**Response** (201 Created):
```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "name": "Multi-file refactoring",
  "pattern_type": "Plan-Execute",
  "status": "pending",
  "tasks": [
    {"id": "task-1", "step": 1, "status": "pending"},
    {"id": "task-2", "step": 2, "status": "pending"},
    {"id": "task-3", "step": 3, "status": "pending"}
  ],
  "created_at": "2025-01-15T10:00:00Z"
}
```

---

#### GET `/api/v1/workflows/:id` - Get Workflow

Retrieve workflow with all tasks.

**Response** (200 OK):
```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "name": "Multi-file refactoring",
  "status": "running",
  "state": {
    "current_step": 2,
    "variables": {...}
  },
  "tasks": [
    {
      "id": "task-1",
      "step": 1,
      "title": "Read file1",
      "status": "completed",
      "result": {...}
    },
    {
      "id": "task-2",
      "step": 2,
      "title": "Read file2",
      "status": "running"
    }
  ],
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T10:15:00Z"
}
```

---

#### GET `/api/v1/workflows` - List Workflows

**Query Parameters**: Same as tasks (status, limit, offset)

**Response** (200 OK):
```json
{
  "workflows": [
    {
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "name": "Multi-file refactoring",
      "pattern_type": "Plan-Execute",
      "status": "running",
      "task_count": 3,
      "created_at": "2025-01-15T10:00:00Z"
    }
  ],
  "total": 25
}
```

---

#### POST `/api/v1/workflows/:id/execute` - Execute Workflow

Start workflow execution (executes tasks in order respecting dependencies).

**Response** (202 Accepted):
```json
{
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "running",
  "started_at": "2025-01-15T10:00:10Z"
}
```

---

#### DELETE `/api/v1/workflows/:id` - Delete Workflow

Delete workflow and all associated tasks.

**Response** (204 No Content)

---

### Tool Executions API

#### GET `/api/v1/tool-executions` - List Executions

List tool execution history with filtering.

**Query Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `task_id` | string | Filter by task |
| `tool_name` | string | Filter by tool |
| `status` | string | Filter by status (success/failure) |
| `since` | string | ISO 8601 datetime (e.g., `2025-01-15T00:00:00Z`) |
| `limit` | integer | Max 100 |
| `offset` | integer | Pagination offset |

**Response** (200 OK):
```json
{
  "executions": [
    {
      "id": "te-001",
      "task_id": "550e8400-e29b-41d4-a716-446655440000",
      "tool_name": "file_read",
      "args": {
        "path": "src/main.rs"
      },
      "result": {
        "content": "fn main() {...}",
        "sha256": "a3f2..."
      },
      "status": "success",
      "duration_ms": 8,
      "executed_at": "2025-01-15T10:30:02Z"
    }
  ],
  "total": 250
}
```

**cURL Example**:
```bash
# Get all file_read executions from last 24 hours
curl "http://localhost:8080/api/v1/tool-executions?tool_name=file_read&since=2025-01-14T10:00:00Z"

# Get failed executions
curl "http://localhost:8080/api/v1/tool-executions?status=failure"
```

---

#### GET `/api/v1/tool-executions/stats` - Execution Statistics

Get aggregated tool execution statistics.

**Query Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `since` | string | ISO 8601 datetime |
| `until` | string | ISO 8601 datetime |

**Response** (200 OK):
```json
{
  "total_executions": 1250,
  "success_rate": 0.98,
  "avg_duration_ms": 35,
  "by_tool": {
    "file_read": {
      "count": 450,
      "success_rate": 0.99,
      "avg_duration_ms": 12
    },
    "git_status": {
      "count": 200,
      "success_rate": 0.97,
      "avg_duration_ms": 45
    },
    "shell_exec": {
      "count": 150,
      "success_rate": 0.93,
      "avg_duration_ms": 120
    }
  },
  "by_status": {
    "success": 1225,
    "failure": 25
  },
  "time_range": {
    "since": "2025-01-08T10:00:00Z",
    "until": "2025-01-15T10:00:00Z"
  }
}
```

**cURL Example**:
```bash
# Get stats for last 7 days
curl "http://localhost:8080/api/v1/tool-executions/stats?since=2025-01-08T00:00:00Z"
```

---

### System API

#### GET `/health` - Health Check

Check if API server is healthy.

**Response** (200 OK):
```json
{
  "status": "healthy",
  "version": "0.2.0",
  "uptime_seconds": 3600,
  "database": "connected",
  "websocket": "active"
}
```

**cURL Example**:
```bash
curl http://localhost:8080/health
```

---

#### GET `/api/v1/system/stats` - System Statistics

Get overall system statistics.

**Response** (200 OK):
```json
{
  "tasks": {
    "total": 150,
    "by_status": {
      "pending": 5,
      "running": 2,
      "completed": 140,
      "failed": 3
    },
    "by_type": {
      "file_operation": 80,
      "git_operation": 40,
      "shell_operation": 30
    }
  },
  "workflows": {
    "total": 25,
    "active": 1
  },
  "tool_executions": {
    "total": 1250,
    "last_24h": 450
  },
  "sessions": {
    "active": 3
  }
}
```

---

## WebSocket Protocol

### Connection

**URL**: `ws://localhost:8080/ws`

**Handshake**:
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  console.log('Connected');
};
```

### Message Types

All messages are JSON objects with a `type` field.

#### Client → Server Messages

**Subscribe to Task**:
```json
{
  "type": "subscribe",
  "resource": "task",
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Subscribe to Workflow**:
```json
{
  "type": "subscribe",
  "resource": "workflow",
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
}
```

**Unsubscribe**:
```json
{
  "type": "unsubscribe",
  "resource": "task",
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Ping** (heartbeat):
```json
{
  "type": "ping"
}
```

#### Server → Client Messages

**Task Update**:
```json
{
  "type": "task_update",
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "progress": 0.45,
  "message": "Processing file 3 of 5",
  "timestamp": "2025-01-15T10:30:02Z"
}
```

**Task Complete**:
```json
{
  "type": "task_complete",
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "result": {
    "content": "...",
    "lines": 150
  },
  "duration_ms": 125,
  "timestamp": "2025-01-15T10:30:05Z"
}
```

**Task Failed**:
```json
{
  "type": "task_failed",
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "failed",
  "error": "File not found: src/missing.rs",
  "timestamp": "2025-01-15T10:30:05Z"
}
```

**Workflow Update**:
```json
{
  "type": "workflow_update",
  "workflow_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "current_step": 2,
  "total_steps": 5,
  "message": "Executing step 2: Read file2"
}
```

**Pong** (heartbeat response):
```json
{
  "type": "pong",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

### Error Messages

**Connection Error**:
```json
{
  "type": "error",
  "code": "CONNECTION_ERROR",
  "message": "Failed to authenticate session"
}
```

**Subscription Error**:
```json
{
  "type": "error",
  "code": "SUBSCRIPTION_ERROR",
  "message": "Resource not found",
  "details": {
    "resource": "task",
    "id": "invalid-id"
  }
}
```

### JavaScript Client Example

```javascript
class AcoWebSocketClient {
  constructor(url = 'ws://localhost:8080/ws') {
    this.url = url;
    this.ws = null;
    this.subscriptions = new Set();
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.startHeartbeat();
    };

    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      this.handleMessage(message);
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    this.ws.onclose = () => {
      console.log('WebSocket closed, reconnecting in 5s...');
      setTimeout(() => this.connect(), 5000);
    };
  }

  handleMessage(message) {
    switch (message.type) {
      case 'task_update':
        console.log(`Task ${message.task_id}: ${message.message}`);
        break;
      case 'task_complete':
        console.log(`Task ${message.task_id} completed`);
        break;
      case 'pong':
        // Heartbeat response
        break;
      default:
        console.warn('Unknown message type:', message.type);
    }
  }

  subscribe(resource, id) {
    const subscription = { resource, id };
    this.subscriptions.add(JSON.stringify(subscription));
    this.send({
      type: 'subscribe',
      resource,
      id
    });
  }

  send(message) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  startHeartbeat() {
    setInterval(() => {
      this.send({ type: 'ping' });
    }, 30000); // Every 30 seconds
  }
}

// Usage
const client = new AcoWebSocketClient();
client.connect();
client.subscribe('task', '550e8400-e29b-41d4-a716-446655440000');
```

---

## Data Models

### Task

```typescript
interface Task {
  id: string;  // UUID
  title: string;
  description?: string;
  task_type: 'file_operation' | 'git_operation' | 'workflow' | 'analysis' | 'shell_operation';
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
  config: Record<string, any>;  // Tool-specific configuration
  metadata?: Record<string, any>;  // User-defined metadata
  workspace_path?: string;
  created_at: string;  // ISO 8601
  updated_at: string;  // ISO 8601
  started_at?: string;  // ISO 8601
  completed_at?: string;  // ISO 8601
  error?: string;
}
```

### Workflow

```typescript
interface Workflow {
  id: string;  // UUID
  name: string;
  description?: string;
  pattern_type: 'ReAct' | 'Plan-Execute' | 'Router' | 'Reflection';
  config: Record<string, any>;
  state: Record<string, any>;  // Current workflow state
  status: 'pending' | 'running' | 'completed' | 'failed';
  created_at: string;
  updated_at: string;
  completed_at?: string;
}
```

### ToolExecution

```typescript
interface ToolExecution {
  id: string;  // UUID
  task_id?: string;  // Nullable
  tool_name: string;
  args: Record<string, any>;
  result?: Record<string, any>;
  status: 'success' | 'failure';
  duration_ms?: number;
  error?: string;
  executed_at: string;  // ISO 8601
}
```

---

## Error Handling

### Error Response Format

All errors follow consistent structure:

```json
{
  "error": "Human-readable error message",
  "code": "ERROR_CODE",
  "details": {
    "field": "Additional context",
    ...
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `VALIDATION_ERROR` | 400 | Invalid request input |
| `TASK_NOT_FOUND` | 404 | Task doesn't exist |
| `WORKFLOW_NOT_FOUND` | 404 | Workflow doesn't exist |
| `TASK_ALREADY_RUNNING` | 409 | Task is already running |
| `TASK_RUNNING` | 409 | Cannot delete running task |
| `DATABASE_ERROR` | 500 | Database operation failed |
| `INTERNAL_ERROR` | 500 | Unexpected server error |

### Example Validation Error

```json
{
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "details": {
    "title": "Field is required",
    "config": "Must be a valid JSON object"
  }
}
```

---

## Rate Limiting

**Current Implementation**: No rate limiting (localhost trust)

**Future** (v0.3.0):
- 100 requests per minute per IP
- 10 concurrent WebSocket connections per IP
- Header: `X-RateLimit-Remaining: 95`

---

## OpenAPI/Swagger

Full OpenAPI 3.0 specification available at:
- Swagger UI: `http://localhost:8080/swagger-ui`
- OpenAPI JSON: `http://localhost:8080/openapi.json`

---

**Document Version**: 1.0
**Last Updated**: 2025-01-15
