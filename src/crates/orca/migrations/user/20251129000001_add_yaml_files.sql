-- YAML file tracking for change detection and sync status
-- Stored in user database (~/.orca/user.db)
-- Tracks checksums to detect file changes and avoid unnecessary reloads

CREATE TABLE IF NOT EXISTS yaml_files (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,
    file_type TEXT NOT NULL,          -- 'workflow', 'template', 'prompt', 'pattern', 'tool'
    content_hash TEXT NOT NULL,        -- SHA-256 hash of file content
    target_table TEXT NOT NULL,        -- 'workflow_templates', 'pattern_configs', 'prompts'
    target_id TEXT,                    -- ID of record in target table (after sync)
    file_size INTEGER,
    last_synced_at INTEGER NOT NULL,
    sync_status TEXT DEFAULT 'synced', -- 'synced', 'pending', 'error'
    sync_error TEXT,                   -- Error message if sync failed
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Indexes for common lookups
CREATE INDEX IF NOT EXISTS idx_yaml_files_file_path ON yaml_files(file_path);
CREATE INDEX IF NOT EXISTS idx_yaml_files_file_type ON yaml_files(file_type);
CREATE INDEX IF NOT EXISTS idx_yaml_files_content_hash ON yaml_files(content_hash);
CREATE INDEX IF NOT EXISTS idx_yaml_files_sync_status ON yaml_files(sync_status);
CREATE INDEX IF NOT EXISTS idx_yaml_files_target_table ON yaml_files(target_table);
