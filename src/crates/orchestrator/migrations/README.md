# Database Migrations

This directory contains SQLx migrations for the acolib orchestrator database. All migrations use SQLite syntax.

## Migration Files

### Core Tables

| Migration | File | Description | Phase |
|-----------|------|-------------|-------|
| 001 | `20250115_001_create_tasks.sql` | Create tasks table for core task management | P7-007 |
| 002 | `20250115_002_create_workflows.sql` | Create workflows table for workflow definitions | P7-008 |
| 003 | `20250115_003_create_workflow_tasks.sql` | Create workflow_tasks junction table (M2M relationship) | P7-009 |
| 004 | `20250115_004_create_tool_executions.sql` | Create tool_executions audit log table | P7-010 |
| 005 | `20250115_005_create_sessions.sql` | Create sessions table for WebSocket connections | P7-011 |
| 006 | `20250115_006_create_configurations.sql` | Create configurations key-value store table | P7-012 |
| 007 | `20250115_007_create_triggers.sql` | Create triggers for automatic timestamp updates | P7-013 |

## Schema Overview

### tasks
Core task management table.

**Columns:**
- `id` (TEXT, PRIMARY KEY) - Unique task identifier (UUID)
- `title` (TEXT, NOT NULL) - Task title
- `description` (TEXT, NULL) - Optional task description
- `task_type` (TEXT, NOT NULL) - Type of task (e.g., "workflow", "tool", "script")
- `status` (TEXT, NOT NULL) - Task status (pending, running, completed, failed, cancelled)
- `config` (TEXT, NOT NULL) - JSON configuration object
- `metadata` (TEXT, NULL) - Optional JSON metadata
- `workspace_path` (TEXT, NULL) - Path to workspace
- `created_at` (TEXT, NOT NULL) - Creation timestamp (default: current time)
- `updated_at` (TEXT, NOT NULL) - Last update timestamp (auto-maintained by trigger)
- `started_at` (TEXT, NULL) - When task execution started
- `completed_at` (TEXT, NULL) - When task execution completed
- `error` (TEXT, NULL) - Error message if failed

**Indexes:**
- `idx_tasks_status` - Query by status
- `idx_tasks_type` - Query by task type
- `idx_tasks_created` - Query by creation date
- `idx_tasks_workspace` - Query by workspace

**Constraints:**
- PRIMARY KEY on id
- CHECK constraint on status values

### workflows
Workflow definitions and metadata.

**Columns:**
- `id` (TEXT, PRIMARY KEY) - Unique workflow identifier (UUID)
- `name` (TEXT, NOT NULL) - Workflow name
- `description` (TEXT, NULL) - Optional description
- `definition` (TEXT, NOT NULL) - JSON workflow definition
- `status` (TEXT, NOT NULL) - Workflow status (draft, active, archived, paused)
- `created_at` (TEXT, NOT NULL) - Creation timestamp
- `updated_at` (TEXT, NOT NULL) - Last update timestamp (auto-maintained by trigger)

**Indexes:**
- `idx_workflows_status` - Query by status
- `idx_workflows_created` - Query by creation date
- `idx_workflows_name` - Query by name

**Constraints:**
- PRIMARY KEY on id
- CHECK constraint on status values

### workflow_tasks
Junction table linking tasks to workflows (many-to-many relationship).

**Columns:**
- `workflow_id` (TEXT, NOT NULL) - Foreign key to workflows table
- `task_id` (TEXT, NOT NULL) - Foreign key to tasks table
- `sequence` (INTEGER, NOT NULL) - Execution order in workflow (default: 0)
- `created_at` (TEXT, NOT NULL) - Creation timestamp

**Indexes:**
- `idx_workflow_tasks_workflow` - Query by workflow
- `idx_workflow_tasks_task` - Query by task
- `idx_workflow_tasks_sequence` - Query by sequence

**Constraints:**
- PRIMARY KEY on (workflow_id, task_id)
- FOREIGN KEY constraints with CASCADE deletion

### tool_executions
Audit log for tool invocations (execution history).

**Columns:**
- `id` (TEXT, PRIMARY KEY) - Unique execution identifier
- `task_id` (TEXT, NOT NULL) - Foreign key to tasks table
- `tool_name` (TEXT, NOT NULL) - Name of tool invoked
- `arguments` (TEXT, NOT NULL) - JSON tool arguments
- `output` (TEXT, NULL) - JSON tool output
- `status` (TEXT, NOT NULL) - Execution status (pending, running, completed, failed, timeout)
- `error` (TEXT, NULL) - Error message if failed
- `created_at` (TEXT, NOT NULL) - Execution start time
- `completed_at` (TEXT, NULL) - Execution completion time
- `duration_ms` (INTEGER, NULL) - Execution duration in milliseconds

**Indexes:**
- `idx_tool_executions_task` - Query by task
- `idx_tool_executions_tool` - Query by tool name
- `idx_tool_executions_status` - Query by status
- `idx_tool_executions_created` - Query by creation date
- `idx_tool_executions_task_fk` - Foreign key lookup

**Constraints:**
- PRIMARY KEY on id
- CHECK constraint on status values

### sessions
WebSocket and connection session tracking.

**Columns:**
- `id` (TEXT, PRIMARY KEY) - Session identifier
- `client_id` (TEXT, NOT NULL) - Client identifier
- `user_id` (TEXT, NULL) - Associated user (if authenticated)
- `connection_type` (TEXT, NOT NULL) - Type of connection (websocket, http, grpc)
- `is_active` (INTEGER, NOT NULL) - Active status (1=true, 0=false)
- `metadata` (TEXT, NULL) - JSON metadata about connection
- `created_at` (TEXT, NOT NULL) - Connection creation time
- `updated_at` (TEXT, NOT NULL) - Last update time (auto-maintained by trigger)
- `last_heartbeat` (TEXT, NOT NULL) - Last heartbeat timestamp

**Indexes:**
- `idx_sessions_client` - Query by client ID
- `idx_sessions_user` - Query by user ID
- `idx_sessions_active` - Query active sessions
- `idx_sessions_heartbeat` - Query by heartbeat for cleanup
- `idx_sessions_created` - Query by creation date

**Constraints:**
- PRIMARY KEY on id
- CHECK constraint on is_active values
- CHECK constraint on connection_type values

### configurations
Key-value configuration store for runtime settings.

**Columns:**
- `key` (TEXT, PRIMARY KEY) - Configuration key
- `value` (TEXT, NOT NULL) - Configuration value
- `value_type` (TEXT, NOT NULL) - Type (string, integer, float, boolean, json)
- `description` (TEXT, NULL) - Human-readable description
- `is_secret` (INTEGER, NOT NULL) - Whether to treat as secret (1=true, 0=false)
- `created_at` (TEXT, NOT NULL) - Creation timestamp
- `updated_at` (TEXT, NOT NULL) - Last update timestamp (auto-maintained by trigger)

**Indexes:**
- `idx_configurations_type` - Query by type
- `idx_configurations_secret` - Query secrets vs. public configs
- `idx_configurations_updated` - Query by update date

**Constraints:**
- PRIMARY KEY on key
- CHECK constraint on is_secret values
- CHECK constraint on value_type values

## Running Migrations

### Prerequisites
- sqlx-cli installed: `cargo install sqlx-cli --no-default-features --features sqlite`
- SQLite database file exists or will be created
- DATABASE_URL environment variable set

### Run All Pending Migrations
```bash
cd crates/orchestrator
sqlx migrate run
```

### Run Specific Migrations (Dry Run)
```bash
sqlx migrate run --dry-run
```

### Revert Last Migration
```bash
sqlx migrate revert
```

### Revert All Migrations
```bash
# Run revert N times where N = number of migrations
for i in {1..7}; do sqlx migrate revert; done
```

### Check Migration Status
```bash
sqlx migrate status
```

### Create New Migration
```bash
sqlx migrate add -r <migration_description>
# Creates two files: UP and DOWN
```

## Environment Setup

### Development
```bash
# Create local SQLite database
export DATABASE_URL="sqlite:orchestrator.db"

# Or in .env file
DATABASE_URL=sqlite:orchestrator.db
```

### Testing
```bash
export DATABASE_URL="sqlite::memory:"
```

### Production
```bash
export DATABASE_URL="sqlite:/var/lib/acolib/orchestrator.db"
```

## Troubleshooting

### Migrations Won't Run
1. Check DATABASE_URL is set: `echo $DATABASE_URL`
2. Verify database file exists: `ls -la orchestrator.db`
3. Check migration syntax: `sqlx migrate list`
4. View error details: `sqlx migrate run --verbose`

### Database Locked
- Ensure no other processes have the database open
- SQLite creates .db-wal and .db-shm files during active connections
- Restart any running instances

### Foreign Key Constraints Failing
- Ensure migrations run in correct order
- Check that parent table records exist before inserting child records
- Enable foreign key constraints: `PRAGMA foreign_keys = ON;`

### Trigger Not Updating Timestamps
- Verify trigger syntax with `SELECT name FROM sqlite_master WHERE type='trigger';`
- Ensure trigger references correct table names
- Test manually: `UPDATE tasks SET title='test' WHERE id='...';`

## Best Practices

1. **Naming**: Always use TIMESTAMP_NUMBER_description format
2. **Reversibility**: Always include valid DOWN migrations
3. **Idempotency**: Use `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS`
4. **Indexes**: Create indexes for common queries and foreign keys
5. **Constraints**: Use CHECK constraints for enum-like values
6. **Comments**: Document complex migrations
7. **Testing**: Always test UP and DOWN migrations locally first

## Migration File Format

SQLx migrations should follow this format:

```sql
-- UP
-- Migration logic here
CREATE TABLE ...
CREATE INDEX ...

-- DOWN
-- Cleanup logic here (commented out)
-- DROP TABLE ...
-- DROP INDEX ...
```

Note: The actual DOWN section should be commented out in the file. sqlx handles UP/DOWN automatically.

## Related Documentation

- See `orchestrator/src/db/connection.rs` for database connection implementation
- See `orchestrator/src/db/models/` for Rust struct definitions
- See `orchestrator/src/db/repositories/` for data access layer
