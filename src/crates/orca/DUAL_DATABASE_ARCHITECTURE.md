# Orca Dual-Database Architecture

## Overview

Orca uses a dual-database architecture to separate user-level configuration from project-specific data. This design provides better data isolation, security, and multi-project workflows.

## Database Structure

### User Database (`~/.orca/user.db`)

**Location**: `~/.orca/user.db` (user's home directory)

**Purpose**: Stores user-level configuration that persists across all projects

**Tables**:
- `llm_providers` - LLM provider configurations (API keys, endpoints, models)
- `prompts` - Reusable prompt templates
- `workflow_templates` - Reusable workflow definitions

**Characteristics**:
- Global to the user
- Contains sensitive data (API keys)
- Shared across all projects
- Backed up with user's home directory

### Project Database (`<project>/.orca/project.db`)

**Location**: `<project>/.orca/project.db` (within each project directory)

**Purpose**: Stores project-specific data and execution state

**Tables**:
- `workflows` - Project workflow instances
- `tasks` - Task execution records
- `bugs` - Bug tracking records
- `project_rules` - Code style, security, and workflow rules
- `tool_permissions` - Tool execution permissions and restrictions
- `tool_executions` - Tool execution audit log
- `ast_cache` - Parsed Abstract Syntax Tree cache for performance
- `workflow_tasks` - Workflow-task relationships
- `task_bugs` - Task-bug relationships

**Characteristics**:
- Project-specific
- Version controlled (`.orca/` can be committed to git)
- Team-shareable
- Isolated between projects

## Architecture Components

### DatabaseManager

Central manager for both databases:

```rust
use orca::DatabaseManager;

// Initialize with workspace root
let manager = DatabaseManager::new(".").await?;

// Access user database (always available)
let user_db = manager.user_db();

// Access project database (may be None if not in a project)
let project_db = manager.project_db();

// Check if project database exists
if manager.has_project() {
    // Project-specific operations
}
```

### Repositories

Each entity has a dedicated repository for database operations:

**User-Level Repositories** (use `user_db()`):
- `LlmProviderRepository` - Manage LLM provider configurations
- `PromptRepository` - Manage prompt templates
- `WorkflowTemplateRepository` - Manage workflow templates

**Project-Level Repositories** (use `project_db()`):
- `BugRepository` - Bug tracking CRUD
- `ProjectRuleRepository` - Project rules management
- `ToolPermissionRepository` - Tool permission enforcement
- `AstCacheRepository` - AST cache management

### Security Layer

**Tool Permission Enforcement**:

```rust
use orca::tools::ToolPermissionEnforcer;

// Create enforcer with DatabaseManager
let enforcer = ToolPermissionEnforcer::new(db_manager);

// Check permissions before execution
let decision = enforcer.check_permission("file_write", &args).await?;

match decision {
    ExecutionDecision::Allow => { /* proceed */ }
    ExecutionDecision::Deny(reason) => { /* block */ }
    ExecutionDecision::RequiresApproval(reason) => { /* request approval */ }
}

// Log execution for audit trail
enforcer.log_execution(tool_name, args, result, duration_ms, approved, task_id).await?;
```

## CLI Commands

### User-Level Commands

These work from any directory (require `~/.orca/user.db`):

```bash
# Initialize user database
orca init

# Health check
orca health

# Version info
orca version
```

### Project-Level Commands

These require a project database (`<project>/.orca/project.db`):

```bash
# Bug tracking
orca bug create "Fix login error" -d "Users cannot login" -p 1
orca bug list
orca bug show <id>
orca bug update-status <id> in_progress
orca bug close <id>

# Project rules
orca rule create "No console.log" -t style -c '{"pattern":"console\\.log"}' -s error
orca rule list
orca rule list-type security
orca rule show <id>
orca rule enable <id>
orca rule disable <id>

# Tasks
orca task create "Implement feature X"
orca task list
orca task run <id>

# Workflows
orca workflow create "CI Pipeline"
orca workflow list
orca workflow run <id>
```

## Data Flow

### Initialization Flow

1. User runs `orca init`
2. Creates `~/.orca/` directory
3. Creates `~/.orca/user.db` with user schema
4. Creates `~/.orca/orca.toml` configuration

### Project Setup Flow

1. User runs `orca init` in project directory
2. Creates `.orca/` directory in project root
3. Creates `.orca/project.db` with project schema
4. Optionally creates `.orca/orca.toml` for project overrides

### Command Execution Flow

1. CLI parses command
2. DatabaseManager initializes:
   - Always connects to `~/.orca/user.db`
   - Detects and connects to `.orca/project.db` if present
3. Command handler receives DatabaseManager
4. Handler accesses appropriate database via `user_db()` or `project_db()`
5. Repository performs database operations
6. Results displayed to user

## Migration Strategy

### Schema Migrations

Migrations are organized by database type:

```
crates/orca/migrations/
├── user/
│   └── 20250115000000_user_schema.sql
└── project/
    └── 20250115000001_project_schema.sql
```

DatabaseManager automatically runs migrations on initialization.

### Data Migration (if needed)

For migrating from old single-database architecture:

```rust
// Pseudocode for migration tool
async fn migrate_old_to_new(old_db: &Path) -> Result<()> {
    let old_conn = connect_to_old(old_db)?;
    let new_manager = DatabaseManager::new(".").await?;

    // Migrate user-level data
    let llm_configs = old_conn.query("SELECT * FROM llm_configs")?;
    for config in llm_configs {
        LlmProviderRepository::new(new_manager.user_db())
            .save(&migrate_config(config)).await?;
    }

    // Migrate project-level data
    let bugs = old_conn.query("SELECT * FROM bugs")?;
    for bug in bugs {
        BugRepository::new(new_manager.project_db()?)
            .save(&migrate_bug(bug)).await?;
    }

    Ok(())
}
```

## Testing Strategy

### Repository Tests

Each repository has comprehensive integration tests:

```rust
#[tokio::test]
async fn test_bug_repository_crud() {
    let (db, _temp) = create_test_db().await;
    run_project_migrations(&db).await;

    let repo = BugRepository::new(db.clone());

    // Test CRUD operations
    // ...
}
```

Tests use temporary SQLite databases for isolation.

### CLI Tests

CLI commands tested with temporary databases:

```rust
#[tokio::test]
async fn test_bug_create_command() {
    setup_test_environment().await;

    let output = run_cli(&["bug", "create", "Test bug"]).await?;

    assert!(output.contains("✓ Bug created successfully"));
}
```

## Security Considerations

### API Key Storage

- API keys stored in `~/.orca/user.db` only
- User database not version-controlled
- File permissions: `0600` (user read/write only)

### Tool Permissions

- Tool execution controlled by `tool_permissions` table
- Four permission levels:
  - `allowed` - Execute without approval
  - `restricted` - Execute with path/argument restrictions
  - `requires_approval` - Require user approval each time
  - `denied` - Block execution entirely

### Audit Logging

All tool executions logged to `tool_executions`:
- Tool name
- Arguments (sanitized)
- Result (success/failure)
- Duration
- User approval status
- Associated task ID

## Best Practices

### For Users

1. **Initialize user database first**: Run `orca init` in your home directory
2. **Per-project initialization**: Run `orca init` in each project directory
3. **Version control**: Commit `.orca/project.db` for team collaboration
4. **Never commit**: Keep `~/.orca/` out of version control

### For Developers

1. **Always use DatabaseManager**: Access databases through the manager
2. **Check project existence**: Use `has_project()` before accessing `project_db()`
3. **Use repositories**: Never access database directly
4. **Handle missing project DB**: Provide helpful error messages

### For Teams

1. **Share project rules**: Commit `.orca/project.db` to ensure consistent rules
2. **Document custom rules**: Add comments to rule configurations
3. **Review tool permissions**: Regularly audit `tool_permissions` table
4. **Monitor executions**: Check `tool_executions` for anomalies

## Performance Considerations

### AST Cache

The `ast_cache` table improves performance:

```rust
use orca::tools::AstCacheService;

let service = AstCacheService::new(db_manager);

// Check cache
if let Some(cached) = service.get("src/main.rs").await? {
    // Use cached AST
    use_cached_ast(&cached.ast_data);
} else {
    // Parse and cache
    let ast = parse_file("src/main.rs")?;
    service.store("src/main.rs", "rust", &ast).await?;
}

// Get cache statistics
let stats = service.get_stats();
println!("Hit rate: {:.2}%", stats.hit_rate());
```

### Connection Pooling

Both databases use connection pooling:
- User DB: 5 max connections (low contention)
- Project DB: 5 max connections (adjustable)

### Database Size

Typical sizes:
- User DB: < 1 MB (small, infrequently updated)
- Project DB: 1-10 MB (grows with project activity)

## Troubleshooting

### "No project database" Error

**Problem**: Command requires project database but none found

**Solution**:
```bash
cd /path/to/project
orca init  # Creates .orca/project.db
```

### "Database locked" Error

**Problem**: Multiple processes accessing database

**Solution**:
- Close other orca instances
- Check for zombie processes: `ps aux | grep orca`
- Wait for locks to release (SQLite default timeout: 5s)

### Migration Failures

**Problem**: Database schema out of date

**Solution**:
```bash
# Backup existing database
cp ~/.orca/user.db ~/.orca/user.db.backup

# Re-initialize (will run migrations)
orca init --force
```

## Future Enhancements

- Remote database support (PostgreSQL, MySQL)
- Database encryption at rest
- Multi-user collaboration features
- Conflict resolution for concurrent edits
- Database replication/sync between team members

## References

- [SQLite Documentation](https://www.sqlite.org/docs.html)
- [sqlx Documentation](https://docs.rs/sqlx/)
- [Orca CLI Guide](./README.md)
- [Security Best Practices](./docs/security.md)
