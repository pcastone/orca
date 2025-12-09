-- Create execution metrics tables for tracking ReAct agent iterations
-- Three-level hierarchy: prompt_executions → execution_iterations → prompt_history (llm_calls)

-- Top-level prompt execution tracking (one per user prompt)
CREATE TABLE IF NOT EXISTS prompt_executions (
    id TEXT PRIMARY KEY NOT NULL,
    original_prompt TEXT NOT NULL,
    project_name TEXT,
    agent_type TEXT NOT NULL DEFAULT 'react',
    session_id TEXT,
    task_id TEXT,

    -- Aggregated metrics
    total_input_tokens INTEGER DEFAULT 0,
    total_output_tokens INTEGER DEFAULT 0,
    total_reasoning_tokens INTEGER DEFAULT 0,
    total_cost_usd REAL DEFAULT 0.0,
    total_duration_ms INTEGER DEFAULT 0,
    iteration_count INTEGER DEFAULT 0,
    llm_call_count INTEGER DEFAULT 0,
    tool_call_count INTEGER DEFAULT 0,

    -- Status
    status TEXT NOT NULL DEFAULT 'running',
    error_message TEXT,
    final_response TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,

    CHECK (agent_type IN ('react', 'plan_execute', 'reflection', 'direct')),
    CHECK (status IN ('running', 'completed', 'failed', 'cancelled')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

-- Per-iteration tracking (one ReAct cycle: agent → tools → agent)
CREATE TABLE IF NOT EXISTS execution_iterations (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL,
    iteration_num INTEGER NOT NULL,

    -- Iteration metrics
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,
    duration_ms INTEGER DEFAULT 0,

    -- What happened
    agent_action TEXT,
    tool_calls_json TEXT,
    tool_results_json TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    CHECK (agent_action IN ('tool_call', 'final_answer', 'error', NULL)),
    FOREIGN KEY (execution_id) REFERENCES prompt_executions(id) ON DELETE CASCADE
);

-- Add iteration_id to prompt_history if not exists
-- This links individual LLM calls to their iteration
ALTER TABLE prompt_history ADD COLUMN iteration_id TEXT REFERENCES execution_iterations(id) ON DELETE SET NULL;

-- Add prompt_execution_id to prompt_history for direct linking
ALTER TABLE prompt_history ADD COLUMN prompt_execution_id TEXT REFERENCES prompt_executions(id) ON DELETE SET NULL;

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_prompt_executions_project ON prompt_executions(project_name);
CREATE INDEX IF NOT EXISTS idx_prompt_executions_session ON prompt_executions(session_id);
CREATE INDEX IF NOT EXISTS idx_prompt_executions_created ON prompt_executions(created_at);
CREATE INDEX IF NOT EXISTS idx_prompt_executions_status ON prompt_executions(status);

CREATE INDEX IF NOT EXISTS idx_execution_iterations_exec ON execution_iterations(execution_id);
CREATE INDEX IF NOT EXISTS idx_execution_iterations_num ON execution_iterations(execution_id, iteration_num);

CREATE INDEX IF NOT EXISTS idx_prompt_history_iteration ON prompt_history(iteration_id);
CREATE INDEX IF NOT EXISTS idx_prompt_history_prompt_exec ON prompt_history(prompt_execution_id);
