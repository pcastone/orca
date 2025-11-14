-- P7-013: Drop triggers (revert)

DROP TRIGGER IF EXISTS trigger_tasks_updated_at;
DROP TRIGGER IF EXISTS trigger_workflows_updated_at;
DROP TRIGGER IF EXISTS trigger_sessions_updated_at;
DROP TRIGGER IF EXISTS trigger_configurations_updated_at;
