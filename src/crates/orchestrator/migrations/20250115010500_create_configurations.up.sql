-- P7-012: Create configurations table
-- Key-value store for runtime and environment configurations

CREATE TABLE IF NOT EXISTS configurations (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    value_type TEXT NOT NULL DEFAULT 'string',
    description TEXT,
    is_secret INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (is_secret IN (0, 1)),
    CHECK (value_type IN ('string', 'integer', 'float', 'boolean', 'json'))
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_configurations_type ON configurations(value_type);
CREATE INDEX IF NOT EXISTS idx_configurations_secret ON configurations(is_secret);
CREATE INDEX IF NOT EXISTS idx_configurations_updated ON configurations(updated_at);
