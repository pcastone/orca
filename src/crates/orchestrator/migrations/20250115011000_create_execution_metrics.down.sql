-- Drop execution metrics tables
-- Note: ALTER TABLE DROP COLUMN is not supported in SQLite
-- We need to recreate prompt_history without the new columns

-- Drop indexes first
DROP INDEX IF EXISTS idx_prompt_history_prompt_exec;
DROP INDEX IF EXISTS idx_prompt_history_iteration;
DROP INDEX IF EXISTS idx_execution_iterations_num;
DROP INDEX IF EXISTS idx_execution_iterations_exec;
DROP INDEX IF EXISTS idx_prompt_executions_status;
DROP INDEX IF EXISTS idx_prompt_executions_created;
DROP INDEX IF EXISTS idx_prompt_executions_session;
DROP INDEX IF EXISTS idx_prompt_executions_project;

-- Drop new tables
DROP TABLE IF EXISTS execution_iterations;
DROP TABLE IF EXISTS prompt_executions;

-- Note: The added columns (iteration_id, prompt_execution_id) on prompt_history
-- will remain as SQLite doesn't support DROP COLUMN in older versions.
-- This is acceptable as they will be NULL and not cause issues.
