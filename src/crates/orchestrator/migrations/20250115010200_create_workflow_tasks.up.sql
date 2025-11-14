-- P7-009: Create workflow_tasks junction table
-- Links tasks to workflows (many-to-many relationship)

CREATE TABLE IF NOT EXISTS workflow_tasks (
    workflow_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    sequence INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (workflow_id, task_id),
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_workflow_tasks_workflow ON workflow_tasks(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_tasks_task ON workflow_tasks(task_id);
CREATE INDEX IF NOT EXISTS idx_workflow_tasks_sequence ON workflow_tasks(workflow_id, sequence);
