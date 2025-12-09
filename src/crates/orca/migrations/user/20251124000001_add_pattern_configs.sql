-- Add pattern_configs table for dynamic workflow configurations
-- Allows tasks to reference specific pattern configurations for dynamic ReAct

-- Pattern configurations table
CREATE TABLE IF NOT EXISTS pattern_configs (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    pattern_type TEXT NOT NULL,  -- react, plan_execute, reflection, lats, storm, etc.
    config TEXT NOT NULL DEFAULT '{}',  -- JSON: full PatternConfig
    tools TEXT NOT NULL DEFAULT '[]',   -- JSON array of tool names
    system_prompt TEXT,                 -- Optional system prompt override
    max_iterations INTEGER NOT NULL DEFAULT 10,
    is_default INTEGER NOT NULL DEFAULT 0,  -- SQLite uses INTEGER for bool
    usage_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    CHECK (pattern_type IN ('react', 'plan_execute', 'reflection', 'lats', 'storm', 'code_act', 'tot', 'cot', 'got'))
);

CREATE INDEX idx_pattern_configs_type ON pattern_configs(pattern_type);
CREATE INDEX idx_pattern_configs_name ON pattern_configs(name);
CREATE INDEX idx_pattern_configs_default ON pattern_configs(is_default);

-- Note: Tasks table is in project database, not user database
-- pattern_config_id foreign key should be added to project migrations

-- Insert default pattern configs
INSERT INTO pattern_configs (id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at)
VALUES
    -- Default ReAct for simple tasks
    ('default_react_simple', 'Quick Tasks', 'react',
     '{"temperature": 0.7}',
     '["read_file", "list_dir", "search"]',
     'You are a helpful assistant. Be concise and efficient.',
     3, 0, 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Default ReAct for general tasks
    ('default_react', 'General ReAct', 'react',
     '{"temperature": 0.7}',
     '["read_file", "write_file", "list_dir", "search", "bash"]',
     'You are a helpful coding assistant.',
     10, 1, 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Code generation with reflection
    ('default_reflection_code', 'Code Generation', 'reflection',
     '{"quality_threshold": 0.85, "max_refinements": 3}',
     '["read_file", "write_file", "run_tests", "compile", "lint"]',
     'You are an expert programmer. Write clean, well-tested code.',
     15, 0, 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Research tasks with plan-execute
    ('default_plan_execute', 'Research Tasks', 'plan_execute',
     '{"max_steps": 10, "enable_replanning": true}',
     '["web_search", "fetch_url", "read_file", "write_file", "summarize"]',
     'You are a research assistant. Create a plan before executing.',
     20, 0, 0, strftime('%s', 'now'), strftime('%s', 'now'));
