-- User-level database schema (~/.orca/user.db)
-- Contains: LLM configurations, global prompts, workflow templates, sessions

-- LLM Provider Configurations
CREATE TABLE IF NOT EXISTS llm_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL, -- openai, anthropic, ollama, etc.
    model TEXT NOT NULL,
    api_key TEXT, -- encrypted or env var reference
    api_base TEXT,
    temperature REAL DEFAULT 0.7,
    max_tokens INTEGER DEFAULT 4096,
    settings TEXT, -- JSON for additional settings
    is_default INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_llm_providers_provider_type ON llm_providers(provider_type);
CREATE INDEX idx_llm_providers_is_default ON llm_providers(is_default);

-- Global Prompt Templates
CREATE TABLE IF NOT EXISTS prompts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    template TEXT NOT NULL,
    category TEXT, -- system, task, workflow, custom
    variables TEXT, -- JSON array of variable names
    metadata TEXT, -- JSON for additional metadata
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_prompts_category ON prompts(category);
CREATE INDEX idx_prompts_name ON prompts(name);

-- Workflow Templates (reusable across projects)
CREATE TABLE IF NOT EXISTS workflow_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    pattern TEXT NOT NULL, -- react, plan_execute, reflection
    definition TEXT NOT NULL, -- JSON workflow definition
    tags TEXT, -- JSON array of tags
    is_public INTEGER DEFAULT 1,
    usage_count INTEGER DEFAULT 0,
    metadata TEXT, -- JSON for additional metadata
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_workflow_templates_pattern ON workflow_templates(pattern);
CREATE INDEX idx_workflow_templates_name ON workflow_templates(name);
CREATE INDEX idx_workflow_templates_is_public ON workflow_templates(is_public);

-- User Sessions (execution contexts)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    workspace_root TEXT NOT NULL,
    description TEXT,
    active INTEGER DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_activity_at INTEGER NOT NULL,
    metadata TEXT -- JSON for additional session data
);

CREATE INDEX idx_sessions_workspace_root ON sessions(workspace_root);
CREATE INDEX idx_sessions_last_activity_at ON sessions(last_activity_at);
CREATE INDEX idx_sessions_active ON sessions(active);
