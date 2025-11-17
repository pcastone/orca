-- Budget system tables
-- Budgets, LLM Profiles, and Pricing

-- Budgets table
CREATE TABLE IF NOT EXISTS budgets (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    type TEXT NOT NULL,
    renewal_interval_unit TEXT,
    renewal_interval_value INTEGER,
    last_renewal_date INTEGER,
    next_renewal_date INTEGER,
    credit_amount REAL,
    credit_cap REAL,
    current_usage REAL NOT NULL DEFAULT 0.0,
    total_spent REAL NOT NULL DEFAULT 0.0,
    enforcement TEXT NOT NULL DEFAULT 'warn',
    active BOOLEAN NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_budgets_name ON budgets(name);
CREATE INDEX idx_budgets_active ON budgets(active);
CREATE INDEX idx_budgets_created_at ON budgets(created_at);

-- LLM Profiles table
CREATE TABLE IF NOT EXISTS llm_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    planner_provider TEXT NOT NULL,
    planner_model TEXT NOT NULL,
    worker_provider TEXT NOT NULL,
    worker_model TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_llm_profiles_name ON llm_profiles(name);
CREATE INDEX idx_llm_profiles_active ON llm_profiles(active);
CREATE INDEX idx_llm_profiles_created_at ON llm_profiles(created_at);

-- LLM Pricing table
CREATE TABLE IF NOT EXISTS llm_pricing (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    cost_per_input_token REAL NOT NULL DEFAULT 0.0,
    cost_per_output_token REAL NOT NULL DEFAULT 0.0,
    cost_per_reasoning_token REAL DEFAULT 0.0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(provider, model)
);

CREATE INDEX idx_llm_pricing_provider ON llm_pricing(provider);
CREATE INDEX idx_llm_pricing_model ON llm_pricing(model);
CREATE INDEX idx_llm_pricing_provider_model ON llm_pricing(provider, model);
