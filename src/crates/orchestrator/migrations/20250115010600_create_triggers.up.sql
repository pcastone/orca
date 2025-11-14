-- P7-013: Create triggers for automatic timestamp updates
-- Automatically update updated_at columns when records are modified

-- Trigger for tasks table
CREATE TRIGGER IF NOT EXISTS trigger_tasks_updated_at
AFTER UPDATE ON tasks
FOR EACH ROW
BEGIN
    UPDATE tasks SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Trigger for workflows table
CREATE TRIGGER IF NOT EXISTS trigger_workflows_updated_at
AFTER UPDATE ON workflows
FOR EACH ROW
BEGIN
    UPDATE workflows SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Trigger for sessions table
CREATE TRIGGER IF NOT EXISTS trigger_sessions_updated_at
AFTER UPDATE ON sessions
FOR EACH ROW
BEGIN
    UPDATE sessions SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Trigger for configurations table
CREATE TRIGGER IF NOT EXISTS trigger_configurations_updated_at
AFTER UPDATE ON configurations
FOR EACH ROW
BEGIN
    UPDATE configurations SET updated_at = datetime('now')
    WHERE key = NEW.key;
END;
