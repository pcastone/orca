-- P7-011: Create sessions table
-- Tracks WebSocket connection sessions for real-time communication

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    client_id TEXT NOT NULL,
    user_id TEXT,
    connection_type TEXT NOT NULL DEFAULT 'websocket',
    is_active INTEGER NOT NULL DEFAULT 1,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_heartbeat TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (is_active IN (0, 1)),
    CHECK (connection_type IN ('websocket', 'http', 'grpc'))
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_sessions_client ON sessions(client_id);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(is_active);
CREATE INDEX IF NOT EXISTS idx_sessions_heartbeat ON sessions(last_heartbeat);
CREATE INDEX IF NOT EXISTS idx_sessions_created ON sessions(created_at);
