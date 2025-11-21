-- Create prompt_history table for tracking LLM interactions
-- Records all prompts sent to LLMs and their responses for debugging and analysis

CREATE TABLE IF NOT EXISTS prompt_history (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT,
    workflow_id TEXT,
    execution_id TEXT,
    session_id TEXT,
    node_id TEXT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_type TEXT NOT NULL DEFAULT 'chat',
    system_prompt TEXT,
    user_prompt TEXT NOT NULL,
    assistant_response TEXT,
    messages TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    total_tokens INTEGER,
    cost_usd REAL,
    latency_ms INTEGER,
    temperature REAL,
    max_tokens INTEGER,
    top_p REAL,
    stop_sequences TEXT,
    tools_available TEXT,
    tool_calls TEXT,
    status TEXT NOT NULL DEFAULT 'completed',
    error_message TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (prompt_type IN ('chat', 'completion', 'embedding', 'tool_use')),
    CHECK (status IN ('pending', 'streaming', 'completed', 'failed', 'cancelled')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE SET NULL
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_prompt_history_task ON prompt_history(task_id);
CREATE INDEX IF NOT EXISTS idx_prompt_history_workflow ON prompt_history(workflow_id);
CREATE INDEX IF NOT EXISTS idx_prompt_history_execution ON prompt_history(execution_id);
CREATE INDEX IF NOT EXISTS idx_prompt_history_session ON prompt_history(session_id);
CREATE INDEX IF NOT EXISTS idx_prompt_history_provider ON prompt_history(provider);
CREATE INDEX IF NOT EXISTS idx_prompt_history_model ON prompt_history(model);
CREATE INDEX IF NOT EXISTS idx_prompt_history_created ON prompt_history(created_at);
CREATE INDEX IF NOT EXISTS idx_prompt_history_status ON prompt_history(status);
