-- Create bugs table for bug tracking
-- Tracks issues, errors, and problems encountered during workflow execution

CREATE TABLE IF NOT EXISTS bugs (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    severity TEXT NOT NULL DEFAULT 'medium',
    status TEXT NOT NULL DEFAULT 'open',
    task_id TEXT,
    workflow_id TEXT,
    execution_id TEXT,
    error_message TEXT,
    stack_trace TEXT,
    reproduction_steps TEXT,
    expected_behavior TEXT,
    actual_behavior TEXT,
    environment TEXT,
    assignee TEXT,
    reporter TEXT,
    labels TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT,
    CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    CHECK (status IN ('open', 'in_progress', 'resolved', 'closed', 'wont_fix')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE SET NULL
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_bugs_status ON bugs(status);
CREATE INDEX IF NOT EXISTS idx_bugs_severity ON bugs(severity);
CREATE INDEX IF NOT EXISTS idx_bugs_task ON bugs(task_id);
CREATE INDEX IF NOT EXISTS idx_bugs_workflow ON bugs(workflow_id);
CREATE INDEX IF NOT EXISTS idx_bugs_created ON bugs(created_at);
CREATE INDEX IF NOT EXISTS idx_bugs_assignee ON bugs(assignee);
