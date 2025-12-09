-- Seed default Ollama LLM profile if no profiles exist
-- This ensures orca works out of the box with local Ollama

INSERT INTO llm_profiles (
    id,
    name,
    planner_provider,
    planner_model,
    worker_provider,
    worker_model,
    active,
    description,
    created_at,
    updated_at
)
SELECT
    'default-ollama',
    'Ollama Qwen',
    'ollama',
    'qwen3:4b',
    'ollama',
    'qwen3:4b',
    1,
    'Default Ollama profile with Qwen for local inference',
    strftime('%s', 'now'),
    strftime('%s', 'now')
WHERE NOT EXISTS (SELECT 1 FROM llm_profiles LIMIT 1);
