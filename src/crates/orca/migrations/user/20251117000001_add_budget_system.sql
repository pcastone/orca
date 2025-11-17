-- Budget management system
CREATE TABLE IF NOT EXISTS budgets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    type TEXT NOT NULL CHECK (type IN ('recurring', 'credit')),

    -- Recurring fields (used when type='recurring')
    renewal_interval_unit TEXT CHECK (renewal_interval_unit IN ('days', 'weeks', 'months')),
    renewal_interval_value INTEGER,
    last_renewal_date INTEGER,
    next_renewal_date INTEGER,

    -- Credit fields (used when type='credit')
    credit_amount REAL,
    credit_cap REAL,

    -- Tracking
    current_usage REAL DEFAULT 0.0,
    total_spent REAL DEFAULT 0.0,
    enforcement TEXT DEFAULT 'warn' CHECK (enforcement IN ('block', 'warn')),
    active BOOLEAN DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_budgets_active ON budgets(active);
CREATE INDEX IF NOT EXISTS idx_budgets_created ON budgets(created_at);

-- LLM provider pricing database (for auto cost calculation)
CREATE TABLE IF NOT EXISTS llm_pricing (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    cost_per_input_token REAL NOT NULL,
    cost_per_output_token REAL NOT NULL,
    cost_per_reasoning_token REAL,
    updated_at INTEGER NOT NULL,
    UNIQUE(provider, model)
);

CREATE INDEX IF NOT EXISTS idx_llm_pricing_provider_model ON llm_pricing(provider, model);

-- LLM profiles for multi-LLM workflows
CREATE TABLE IF NOT EXISTS llm_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    planner_provider TEXT NOT NULL,
    planner_model TEXT NOT NULL,
    worker_provider TEXT NOT NULL,
    worker_model TEXT NOT NULL,
    active BOOLEAN DEFAULT 0,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_llm_profiles_active ON llm_profiles(active);
CREATE INDEX IF NOT EXISTS idx_llm_profiles_name ON llm_profiles(name);

-- Usage/cost tracking logs
CREATE TABLE IF NOT EXISTS usage_logs (
    id TEXT PRIMARY KEY,
    budget_id TEXT,
    llm_provider TEXT NOT NULL,
    llm_model TEXT NOT NULL,
    request_type TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    reasoning_tokens INTEGER,
    cost_usd REAL NOT NULL,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (budget_id) REFERENCES budgets(id)
);

CREATE INDEX IF NOT EXISTS idx_usage_logs_budget ON usage_logs(budget_id);
CREATE INDEX IF NOT EXISTS idx_usage_logs_timestamp ON usage_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_usage_logs_provider_model ON usage_logs(llm_provider, llm_model);
