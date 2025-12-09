-- Execution Metrics Tables for tracking ReAct agent execution
-- Three-level hierarchy: prompt_executions -> execution_iterations -> llm_calls

-- Table 1: Prompt Executions (top-level aggregation)
-- Tracks the complete execution of a single prompt through the agent
CREATE TABLE IF NOT EXISTS prompt_executions (
    id TEXT PRIMARY KEY NOT NULL,
    original_prompt TEXT NOT NULL,
    project_name TEXT,
    agent_type TEXT NOT NULL DEFAULT 'react',
    session_id TEXT REFERENCES sessions(id),
    task_id TEXT,

    -- Aggregated token metrics
    total_input_tokens INTEGER DEFAULT 0,
    total_output_tokens INTEGER DEFAULT 0,
    total_reasoning_tokens INTEGER DEFAULT 0,

    -- Cost and timing
    total_cost_usd REAL DEFAULT 0.0,
    total_duration_ms INTEGER DEFAULT 0,

    -- Iteration/call counts
    iteration_count INTEGER DEFAULT 0,
    llm_call_count INTEGER DEFAULT 0,
    tool_call_count INTEGER DEFAULT 0,

    -- Status
    status TEXT NOT NULL DEFAULT 'running',
    final_response TEXT,
    error_message TEXT,

    -- Timestamps
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME
);

-- Table 2: Execution Iterations (per agent cycle)
-- Each iteration represents one agent -> tools -> agent cycle
CREATE TABLE IF NOT EXISTS execution_iterations (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL REFERENCES prompt_executions(id) ON DELETE CASCADE,
    iteration_num INTEGER NOT NULL,

    -- Per-iteration token metrics
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,

    -- Timing
    duration_ms INTEGER DEFAULT 0,

    -- Tool execution info
    tool_calls TEXT, -- JSON array of tool calls
    tool_results TEXT, -- JSON array of results

    -- Agent state
    thought TEXT,
    action TEXT,
    observation TEXT,

    -- Status
    status TEXT NOT NULL DEFAULT 'running',

    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME
);

-- Table 3: LLM Calls (individual API calls)
-- Tracks each individual LLM API call for detailed cost/token tracking
CREATE TABLE IF NOT EXISTS llm_calls (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL REFERENCES prompt_executions(id) ON DELETE CASCADE,
    iteration_id TEXT REFERENCES execution_iterations(id) ON DELETE SET NULL,

    -- Provider info
    provider TEXT NOT NULL,
    model TEXT NOT NULL,

    -- Token metrics
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,

    -- Cost and timing
    cost_usd REAL DEFAULT 0.0,
    latency_ms INTEGER DEFAULT 0,

    -- Request/response (optional, can be large)
    request_messages TEXT, -- JSON array of messages
    response_content TEXT,

    -- Timestamps
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_prompt_executions_project ON prompt_executions(project_name);
CREATE INDEX IF NOT EXISTS idx_prompt_executions_session ON prompt_executions(session_id);
CREATE INDEX IF NOT EXISTS idx_prompt_executions_task ON prompt_executions(task_id);
CREATE INDEX IF NOT EXISTS idx_prompt_executions_created ON prompt_executions(created_at);
CREATE INDEX IF NOT EXISTS idx_prompt_executions_status ON prompt_executions(status);

CREATE INDEX IF NOT EXISTS idx_execution_iterations_execution ON execution_iterations(execution_id);
CREATE INDEX IF NOT EXISTS idx_execution_iterations_num ON execution_iterations(execution_id, iteration_num);

CREATE INDEX IF NOT EXISTS idx_llm_calls_execution ON llm_calls(execution_id);
CREATE INDEX IF NOT EXISTS idx_llm_calls_iteration ON llm_calls(iteration_id);
CREATE INDEX IF NOT EXISTS idx_llm_calls_provider ON llm_calls(provider);
CREATE INDEX IF NOT EXISTS idx_llm_calls_created ON llm_calls(created_at);
