-- Project-level database schema (<project>/.orca/project.db)
-- Contains: Project workflows, tasks, bugs, rules, permissions, AST cache

-- Project-specific Workflows (can reference user templates)
CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, running, completed, failed, cancelled, paused
    pattern TEXT NOT NULL, -- react, plan_execute, reflection
    template_id TEXT, -- references workflow_templates.id from user DB
    definition TEXT, -- JSON workflow definition (if customized from template)
    routing_strategy TEXT DEFAULT 'sequential', -- sequential, parallel, conditional
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER,
    metadata TEXT -- JSON for additional workflow data
);

CREATE INDEX idx_workflows_status ON workflows(status);
CREATE INDEX idx_workflows_name ON workflows(name);
CREATE INDEX idx_workflows_created_at ON workflows(created_at);
CREATE INDEX idx_workflows_template_id ON workflows(template_id);

-- Project Tasks
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, running, completed, failed, cancelled
    priority INTEGER DEFAULT 0,
    result TEXT,
    error TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER,
    metadata TEXT -- JSON for additional task data
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_priority ON tasks(priority);
CREATE INDEX idx_tasks_created_at ON tasks(created_at);

-- Workflow-Task Junction Table
CREATE TABLE IF NOT EXISTS workflow_tasks (
    workflow_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (workflow_id, task_id),
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX idx_workflow_tasks_workflow_id ON workflow_tasks(workflow_id);
CREATE INDEX idx_workflow_tasks_task_id ON workflow_tasks(task_id);

-- Bug Tracking
CREATE TABLE IF NOT EXISTS bugs (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open', -- open, in_progress, fixed, wontfix, duplicate
    priority INTEGER DEFAULT 3, -- 1=critical, 2=high, 3=medium, 4=low, 5=trivial
    severity TEXT, -- critical, major, minor, trivial
    assignee TEXT,
    reporter TEXT,
    labels TEXT, -- JSON array of labels
    related_files TEXT, -- JSON array of file paths
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    resolved_at INTEGER,
    metadata TEXT -- JSON for additional bug data
);

CREATE INDEX idx_bugs_status ON bugs(status);
CREATE INDEX idx_bugs_priority ON bugs(priority);
CREATE INDEX idx_bugs_assignee ON bugs(assignee);
CREATE INDEX idx_bugs_created_at ON bugs(created_at);

-- Project Rules (code style, security, workflow)
CREATE TABLE IF NOT EXISTS project_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    rule_type TEXT NOT NULL, -- style, security, workflow, custom
    description TEXT,
    config TEXT NOT NULL, -- JSON rule configuration
    severity TEXT DEFAULT 'warning', -- error, warning, info
    enabled INTEGER DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_project_rules_rule_type ON project_rules(rule_type);
CREATE INDEX idx_project_rules_enabled ON project_rules(enabled);

-- Tool Permissions (security controls)
CREATE TABLE IF NOT EXISTS tool_permissions (
    id TEXT PRIMARY KEY,
    tool_name TEXT NOT NULL UNIQUE,
    permission_level TEXT NOT NULL, -- allowed, restricted, requires_approval, denied
    path_restrictions TEXT, -- JSON array of allowed paths (e.g., ["/project/*"])
    arg_whitelist TEXT, -- JSON array of allowed argument patterns
    arg_blacklist TEXT, -- JSON array of forbidden argument patterns
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_tool_permissions_tool_name ON tool_permissions(tool_name);
CREATE INDEX idx_tool_permissions_permission_level ON tool_permissions(permission_level);

-- Tool Execution Audit Log
CREATE TABLE IF NOT EXISTS tool_executions (
    id TEXT PRIMARY KEY,
    tool_name TEXT NOT NULL,
    arguments TEXT NOT NULL, -- JSON arguments
    result TEXT, -- JSON result or error
    status TEXT NOT NULL, -- success, failure, denied, approved, rejected
    duration_ms INTEGER,
    user_approval INTEGER DEFAULT 0, -- 1 if user approved execution
    task_id TEXT, -- Optional: link to task that triggered execution
    created_at INTEGER NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE INDEX idx_tool_executions_tool_name ON tool_executions(tool_name);
CREATE INDEX idx_tool_executions_status ON tool_executions(status);
CREATE INDEX idx_tool_executions_created_at ON tool_executions(created_at);
CREATE INDEX idx_tool_executions_task_id ON tool_executions(task_id);

-- AST Cache (parsed Abstract Syntax Trees)
CREATE TABLE IF NOT EXISTS ast_cache (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,
    language TEXT NOT NULL, -- rust, python, javascript, typescript, etc.
    content_hash TEXT NOT NULL, -- SHA-256 hash of file content
    ast_data TEXT NOT NULL, -- JSON serialized AST
    symbols TEXT, -- JSON array of symbols (functions, classes, etc.)
    imports TEXT, -- JSON array of import statements
    file_size INTEGER,
    parse_duration_ms INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    accessed_at INTEGER NOT NULL
);

CREATE INDEX idx_ast_cache_file_path ON ast_cache(file_path);
CREATE INDEX idx_ast_cache_language ON ast_cache(language);
CREATE INDEX idx_ast_cache_content_hash ON ast_cache(content_hash);
CREATE INDEX idx_ast_cache_accessed_at ON ast_cache(accessed_at);

-- Task-Bug Junction Table (link tasks to bugs they fix)
CREATE TABLE IF NOT EXISTS task_bugs (
    task_id TEXT NOT NULL,
    bug_id TEXT NOT NULL,
    relationship TEXT DEFAULT 'fixes', -- fixes, relates_to, caused_by
    created_at INTEGER NOT NULL,
    PRIMARY KEY (task_id, bug_id),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (bug_id) REFERENCES bugs(id) ON DELETE CASCADE
);

CREATE INDEX idx_task_bugs_task_id ON task_bugs(task_id);
CREATE INDEX idx_task_bugs_bug_id ON task_bugs(bug_id);
