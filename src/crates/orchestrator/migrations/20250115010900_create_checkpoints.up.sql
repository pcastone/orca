-- Create checkpoints table for workflow state persistence
-- Stores execution checkpoints for recovery and debugging

CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    node_id TEXT,
    superstep INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL,
    parent_checkpoint_id TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_checkpoint_id) REFERENCES checkpoints(id) ON DELETE SET NULL
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_checkpoints_execution ON checkpoints(execution_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_workflow ON checkpoints(workflow_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_node ON checkpoints(node_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_created ON checkpoints(created_at);
CREATE INDEX IF NOT EXISTS idx_checkpoints_superstep ON checkpoints(superstep);
