//! Database seeding functions for orca installation
//!
//! Handles inserting default data into orca's SQLite databases.

use crate::schema::*;
use anyhow::Result;
use sqlx::{Pool, Sqlite, SqlitePool};
use std::path::Path;
use tracing::{debug, info};
use uuid::Uuid;

/// Database seeder for orca
pub struct DatabaseSeeder {
    force: bool,
}

impl DatabaseSeeder {
    pub fn new(force: bool) -> Self {
        Self { force }
    }

    /// Seed the user database with default data
    pub async fn seed_user_db(&self, db_path: &Path, seed: &OrcaDatabaseSeed) -> Result<()> {
        info!("Seeding user database at {:?}", db_path);

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create database file if it doesn't exist
        if !db_path.exists() {
            std::fs::File::create(db_path)?;
        }

        let db_url = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&db_url).await?;

        // Run migrations first (create tables)
        self.run_user_migrations(&pool).await?;

        // Seed data
        self.seed_llm_providers(&pool, &seed.llm_providers).await?;
        self.seed_llm_pricing(&pool, &seed.llm_pricing).await?;
        self.seed_budgets(&pool, &seed.budgets).await?;
        self.seed_llm_profiles(&pool, &seed.llm_profiles).await?;
        self.seed_prompts(&pool, &seed.prompts).await?;
        self.seed_workflow_templates(&pool, &seed.workflow_templates)
            .await?;

        info!("User database seeding complete");
        Ok(())
    }

    /// Seed the project database with pattern configs
    pub async fn seed_project_db(&self, db_path: &Path, seed: &OrcaDatabaseSeed) -> Result<()> {
        info!("Seeding project database at {:?}", db_path);

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create database file if it doesn't exist
        if !db_path.exists() {
            std::fs::File::create(db_path)?;
        }

        let db_url = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&db_url).await?;

        // Run migrations first
        self.run_project_migrations(&pool).await?;

        // Seed pattern configs
        self.seed_pattern_configs(&pool, &seed.pattern_configs)
            .await?;

        info!("Project database seeding complete");
        Ok(())
    }

    async fn run_user_migrations(&self, pool: &Pool<Sqlite>) -> Result<()> {
        debug!("Running user database migrations");

        // Create tables if they don't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS llm_providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                provider_type TEXT,
                model TEXT NOT NULL,
                api_key TEXT,
                api_base TEXT,
                temperature REAL DEFAULT 0.7,
                max_tokens INTEGER DEFAULT 4096,
                settings TEXT,
                is_default INTEGER DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS llm_pricing (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                cost_per_input_token REAL,
                cost_per_output_token REAL,
                cost_per_reasoning_token REAL,
                updated_at INTEGER,
                UNIQUE(provider, model)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS budgets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                type TEXT NOT NULL,
                renewal_interval_unit TEXT,
                renewal_interval_value INTEGER,
                last_renewal_date INTEGER,
                next_renewal_date INTEGER,
                credit_amount REAL,
                credit_cap REAL,
                current_usage REAL DEFAULT 0.0,
                total_spent REAL DEFAULT 0.0,
                enforcement TEXT DEFAULT 'warn',
                active BOOLEAN DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS llm_profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                planner_provider TEXT NOT NULL,
                planner_model TEXT NOT NULL,
                worker_provider TEXT NOT NULL,
                worker_model TEXT NOT NULL,
                active BOOLEAN DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                template TEXT NOT NULL,
                category TEXT,
                variables TEXT,
                metadata TEXT,
                created_at INTEGER,
                updated_at INTEGER
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                pattern TEXT NOT NULL,
                definition TEXT NOT NULL,
                tags TEXT,
                is_public INTEGER DEFAULT 0,
                usage_count INTEGER DEFAULT 0,
                metadata TEXT,
                created_at INTEGER,
                updated_at INTEGER
            )
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn run_project_migrations(&self, pool: &Pool<Sqlite>) -> Result<()> {
        debug!("Running project database migrations");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pattern_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                pattern_type TEXT NOT NULL,
                config TEXT,
                tools TEXT,
                system_prompt TEXT,
                max_iterations INTEGER DEFAULT 10,
                is_default INTEGER DEFAULT 0,
                usage_count INTEGER DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER
            )
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn seed_llm_providers(&self, pool: &Pool<Sqlite>, providers: &[LlmProviderSeed]) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        for provider in providers {
            // Check if provider with same name already exists
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM llm_providers WHERE name = ?"
            )
                .bind(&provider.name)
                .fetch_optional(pool)
                .await?;

            let exists = existing.is_some();

            if exists && !self.force {
                debug!("LLM provider already exists, skipping: {}", provider.name);
                continue;
            }

            let id = existing.map(|(id,)| id).unwrap_or_else(|| Uuid::new_v4().to_string());
            let settings_json = provider
                .settings
                .as_ref()
                .map(|s| serde_json::to_string(s).unwrap_or_default());

            if self.force && exists {
                // Update existing provider
                sqlx::query(r#"
                    UPDATE llm_providers SET
                        provider_type = ?, model = ?, api_key = ?, api_base = ?,
                        temperature = ?, max_tokens = ?, settings = ?, is_default = ?, updated_at = ?
                    WHERE id = ?
                "#)
                    .bind(&provider.provider_type)
                    .bind(&provider.model)
                    .bind(&provider.api_key)
                    .bind(&provider.api_base)
                    .bind(provider.temperature)
                    .bind(provider.max_tokens as i64)
                    .bind(&settings_json)
                    .bind(provider.is_default as i32)
                    .bind(now)
                    .bind(&id)
                    .execute(pool)
                    .await?;
            } else {
                // Insert new provider
                sqlx::query(r#"
                    INSERT INTO llm_providers
                    (id, name, provider_type, model, api_key, api_base, temperature, max_tokens, settings, is_default, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#)
                    .bind(&id)
                    .bind(&provider.name)
                    .bind(&provider.provider_type)
                    .bind(&provider.model)
                    .bind(&provider.api_key)
                    .bind(&provider.api_base)
                    .bind(provider.temperature)
                    .bind(provider.max_tokens as i64)
                    .bind(&settings_json)
                    .bind(provider.is_default as i32)
                    .bind(now)
                    .bind(now)
                    .execute(pool)
                    .await?;
            }

            debug!("Seeded LLM provider: {}", provider.name);
        }

        Ok(())
    }

    async fn seed_llm_pricing(&self, pool: &Pool<Sqlite>, pricing: &[LlmPricingSeed]) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        for price in pricing {
            let id = Uuid::new_v4().to_string();

            let query = if self.force {
                r#"
                INSERT OR REPLACE INTO llm_pricing
                (id, provider, model, cost_per_input_token, cost_per_output_token, cost_per_reasoning_token, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#
            } else {
                r#"
                INSERT OR IGNORE INTO llm_pricing
                (id, provider, model, cost_per_input_token, cost_per_output_token, cost_per_reasoning_token, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#
            };

            sqlx::query(query)
                .bind(&id)
                .bind(&price.provider)
                .bind(&price.model)
                .bind(price.cost_per_input_token)
                .bind(price.cost_per_output_token)
                .bind(price.cost_per_reasoning_token)
                .bind(now)
                .execute(pool)
                .await?;

            debug!("Seeded LLM pricing: {}:{}", price.provider, price.model);
        }

        Ok(())
    }

    async fn seed_budgets(&self, pool: &Pool<Sqlite>, budgets: &[BudgetSeed]) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        for budget in budgets {
            let id = Uuid::new_v4().to_string();

            let query = if self.force {
                r#"
                INSERT OR REPLACE INTO budgets
                (id, name, type, renewal_interval_unit, renewal_interval_value, credit_amount, credit_cap, enforcement, active, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            } else {
                r#"
                INSERT OR IGNORE INTO budgets
                (id, name, type, renewal_interval_unit, renewal_interval_value, credit_amount, credit_cap, enforcement, active, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            };

            sqlx::query(query)
                .bind(&id)
                .bind(&budget.name)
                .bind(&budget.budget_type)
                .bind(&budget.renewal_interval_unit)
                .bind(budget.renewal_interval_value.map(|v| v as i64))
                .bind(budget.credit_amount)
                .bind(budget.credit_cap)
                .bind(&budget.enforcement)
                .bind(budget.active as i32)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;

            debug!("Seeded budget: {}", budget.name);
        }

        Ok(())
    }

    async fn seed_llm_profiles(&self, pool: &Pool<Sqlite>, profiles: &[LlmProfileSeed]) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        for profile in profiles {
            // Check if profile with same name already exists
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM llm_profiles WHERE name = ?"
            )
                .bind(&profile.name)
                .fetch_optional(pool)
                .await?;

            let exists = existing.is_some();

            if exists && !self.force {
                debug!("LLM profile already exists, skipping: {}", profile.name);
                continue;
            }

            let id = existing.map(|(id,)| id).unwrap_or_else(|| Uuid::new_v4().to_string());

            if self.force && exists {
                // Update existing profile
                sqlx::query(r#"
                    UPDATE llm_profiles SET
                        description = ?, planner_provider = ?, planner_model = ?,
                        worker_provider = ?, worker_model = ?, active = ?, updated_at = ?
                    WHERE id = ?
                "#)
                    .bind(&profile.description)
                    .bind(&profile.planner_provider)
                    .bind(&profile.planner_model)
                    .bind(&profile.worker_provider)
                    .bind(&profile.worker_model)
                    .bind(profile.active as i32)
                    .bind(now)
                    .bind(&id)
                    .execute(pool)
                    .await?;
            } else {
                // Insert new profile
                sqlx::query(r#"
                    INSERT INTO llm_profiles
                    (id, name, description, planner_provider, planner_model, worker_provider, worker_model, active, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#)
                    .bind(&id)
                    .bind(&profile.name)
                    .bind(&profile.description)
                    .bind(&profile.planner_provider)
                    .bind(&profile.planner_model)
                    .bind(&profile.worker_provider)
                    .bind(&profile.worker_model)
                    .bind(profile.active as i32)
                    .bind(now)
                    .bind(now)
                    .execute(pool)
                    .await?;
            }

            debug!("Seeded LLM profile: {}", profile.name);
        }

        Ok(())
    }

    async fn seed_prompts(&self, pool: &Pool<Sqlite>, prompts: &[PromptSeed]) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        for prompt in prompts {
            let id = Uuid::new_v4().to_string();
            let variables_json = serde_json::to_string(&prompt.variables)?;
            let metadata_json = prompt
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_default());

            let query = if self.force {
                r#"
                INSERT OR REPLACE INTO prompts
                (id, name, description, template, category, variables, metadata, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            } else {
                r#"
                INSERT OR IGNORE INTO prompts
                (id, name, description, template, category, variables, metadata, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            };

            sqlx::query(query)
                .bind(&id)
                .bind(&prompt.name)
                .bind(&prompt.description)
                .bind(&prompt.template)
                .bind(&prompt.category)
                .bind(&variables_json)
                .bind(&metadata_json)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;

            debug!("Seeded prompt: {}", prompt.name);
        }

        Ok(())
    }

    async fn seed_workflow_templates(
        &self,
        pool: &Pool<Sqlite>,
        templates: &[WorkflowTemplateSeed],
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        for template in templates {
            let id = Uuid::new_v4().to_string();
            let tags_json = serde_json::to_string(&template.tags)?;

            let query = if self.force {
                r#"
                INSERT OR REPLACE INTO workflow_templates
                (id, name, description, pattern, definition, tags, is_public, usage_count, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?)
                "#
            } else {
                r#"
                INSERT OR IGNORE INTO workflow_templates
                (id, name, description, pattern, definition, tags, is_public, usage_count, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?)
                "#
            };

            sqlx::query(query)
                .bind(&id)
                .bind(&template.name)
                .bind(&template.description)
                .bind(&template.pattern)
                .bind(&template.definition)
                .bind(&tags_json)
                .bind(template.is_public as i32)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;

            debug!("Seeded workflow template: {}", template.name);
        }

        Ok(())
    }

    async fn seed_pattern_configs(
        &self,
        pool: &Pool<Sqlite>,
        configs: &[PatternConfigSeed],
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        for config in configs {
            let id = Uuid::new_v4().to_string();
            let tools_json = serde_json::to_string(&config.tools)?;
            let config_json = config
                .config
                .as_ref()
                .map(|c| serde_json::to_string(c).unwrap_or_default());

            let query = if self.force {
                r#"
                INSERT OR REPLACE INTO pattern_configs
                (id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)
                "#
            } else {
                r#"
                INSERT OR IGNORE INTO pattern_configs
                (id, name, pattern_type, config, tools, system_prompt, max_iterations, is_default, usage_count, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)
                "#
            };

            sqlx::query(query)
                .bind(&id)
                .bind(&config.name)
                .bind(&config.pattern_type)
                .bind(&config_json)
                .bind(&tools_json)
                .bind(&config.system_prompt)
                .bind(config.max_iterations as i64)
                .bind(config.is_default as i32)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;

            debug!("Seeded pattern config: {}", config.name);
        }

        Ok(())
    }
}
