# Orca - Product Requirements Document

## Executive Summary

**Orca** is a standalone, single-process AI orchestration platform that enables users to build and execute AI agent workflows with budget management, multi-LLM support, and cost tracking capabilities. Orca provides a simple, self-contained alternative to distributed orchestration systems, requiring no server infrastructure while maintaining full workflow management, state persistence, and execution observability.

## Product Vision

Enable teams to:
1. **Build Intelligent Workflows** - Compose AI agent workflows using the Pregel execution model
2. **Manage LLM Costs** - Track, control, and enforce spending limits across multiple LLM providers
3. **Multi-LLM Optimization** - Use different LLM providers for different workflow steps (e.g., fast planner, powerful worker)
4. **Standalone Deployment** - Run workflows without server dependency or infrastructure overhead

## Core Features

### 1. Workflow Execution Engine
- **Pregel-based execution model** - Bulk Synchronous Parallel (BSP) superstep-based execution
- **Checkpoint-based resilience** - Automatic state snapshots after each superstep
- **Parallel node execution** - Multiple workflow nodes execute concurrently within supersteps
- **Streaming support** - Token-by-token LLM response streaming for real-time feedback
- **Deterministic execution** - Reproducible, testable workflow runs

### 2. Budget Management System
- **Budget Types**:
  - **Credit-based** - Fixed amount budgets (e.g., $100 for this month)
  - **Recurring** - Automatic renewal at intervals (daily, weekly, monthly)
- **Budget Enforcement Modes**:
  - **Warn** - Alert when usage exceeds 80% threshold, continue execution
  - **Block** - Hard limit enforcement, reject operations exceeding budget
- **Usage Tracking**:
  - Real-time cost tracking per workflow
  - Current usage and total spent metrics
  - Remaining budget calculations
- **Active Budget Selection**:
  - Set one budget as active for cost tracking
  - Support for multiple budgets with switching capability

### 3. Multi-LLM Profile Management
- **Dual-LLM Configurations**:
  - **Planner LLM** - For reasoning, planning, and analysis (can use capable/expensive models)
  - **Worker LLM** - For execution, tool calls, and follow-up tasks (can use faster/cheaper models)
- **Profile Management**:
  - Create, retrieve, update, delete LLM profiles
  - Set one profile as active
  - Store provider and model specifications
  - Optional descriptions for profile documentation
- **Supported Providers**:
  - Anthropic (Claude models: 3-opus, 3-sonnet, 3-haiku)
  - OpenAI (GPT-4, GPT-3.5, o1, reasoning models)
  - Ollama (local open-source models)
  - llama.cpp, LM Studio (local models)
  - Other providers via provider integration

### 4. LLM Pricing & Cost Calculation
- **Pricing Database**:
  - Input token costs (per 1M tokens)
  - Output token costs (per 1M tokens)
  - Reasoning token costs (for o1, R1 models)
- **Cost Calculation Features**:
  - Per-request cost calculation
  - Support for thinking models with reasoning token tracking
  - Unknown model default (free pricing for unmapped models)
  - Pricing updates and management
- **Provider Pricing Support**:
  - Anthropic Claude models
  - OpenAI GPT and reasoning models
  - Extensible for additional providers

### 5. Standalone Architecture
- **Single-Process Deployment**:
  - No server dependency
  - No distributed coordination overhead
  - Direct in-process tool execution
- **Local SQLite Database**:
  - User-level database at `~/.orca/orca.db`
  - Project-level database at `.orca/project.db`
  - Automatic schema initialization
- **Configuration Management**:
  - Project-level config: `./.orca/orca.toml`
  - User-level config: `~/.orca/orca.toml`
  - Environment variable expansion support
- **Direct Tool Bridge**:
  - In-process tool execution
  - No WebSocket or RPC overhead
  - Full workspace access

### 6. State Management & Persistence
- **State Definition**:
  - Explicit schema with reducer types
  - Support for multiple state reduction strategies:
    - **Append** - Accumulate values (for message history)
    - **Overwrite** - Replace with latest value (for status fields)
    - **Merge** - Deep merge objects (for nested state)
    - **Sum** - Aggregate numeric values (for counters)
- **Checkpoint Storage**:
  - SQLite-backed checkpoint saving
  - Thread-level checkpoints for resumable execution
  - Automatic checkpoint creation after each superstep

### 7. Execution Observability
- **Event Logging**:
  - Execution events for workflow state transitions
  - Step-by-step execution tracking
  - Cost tracking events
- **Execution Tracking**:
  - Session management for execution context
  - Task lifecycle tracking
  - Workflow status monitoring
- **Budget Integration**:
  - Automatic cost tracking per workflow execution
  - Budget status before/after execution
  - Cost alerts when approaching limits

## Database Schema

### Orca Database Tables

#### Core Workflow Tables
- **tasks** - Individual task/step definitions and status
- **workflows** - Workflow definitions and metadata
- **workflow_tasks** - M2M relationship between workflows and tasks
- **sessions** - Execution session tracking

#### Budget Management Tables
- **budgets** - Budget definitions (credit/recurring type, enforcement mode)
- **budget_tracking** - Usage tracking and renewal dates

#### LLM Configuration Tables
- **llm_profiles** - Planner/worker LLM configurations
- **llm_pricing** - Cost per token for providers and models

#### Operational Tables
- **tool_executions** - Audit log of tool invocations
- **configurations** - Key-value configuration store

## User Workflows

### Workflow 1: Setup & Initial Configuration
1. User installs Orca
2. Creates `.orca/orca.toml` with default LLM provider
3. Sets environment variable for API key (e.g., `ANTHROPIC_API_KEY`)
4. Orca auto-initializes database and loads pricing data

### Workflow 2: Budget-Based Cost Control
1. User creates budget: `$500 credit budget for this project`
2. Sets enforcement mode: `Block` (hard limit)
3. Orca tracks costs against budget
4. When approaching 80%, status warning displayed
5. At 100%, execution blocked with budget exceeded error

### Workflow 3: Multi-LLM Optimization
1. User creates LLM profile: "FastPlanner"
   - Planner: Anthropic Claude-3-Opus (capable, expensive)
   - Worker: OpenAI GPT-3.5 (fast, cheap)
2. Workflow uses different models for different steps
3. Costs tracked separately per model
4. User can compare cost/quality tradeoffs

### Workflow 4: Execute Workflow with Budget Tracking
1. User runs workflow with active budget
2. Before execution: Check budget remaining
3. During execution: Track token usage and costs
4. After execution: Log cost against budget
5. Display budget status update

## Technical Specifications

### Architecture
- **Language**: Rust 1.75+
- **Async Runtime**: Tokio
- **Database**: SQLite (local), PostgreSQL (optional for orchestrator)
- **UI**: TUI (ratatui) for Aco client, CLI for Orca
- **Execution Model**: Pregel (BSP) - Bulk Synchronous Parallel

### Data Models

#### Budget Model
```rust
struct Budget {
    id: String,                          // UUID
    name: String,
    budget_type: BudgetType,            // Credit | Recurring
    credit_amount: Option<f64>,
    current_usage: f64,
    enforcement: BudgetEnforcement,     // Warn | Block
    active: bool,
}
```

#### LLM Profile Model
```rust
struct LlmProfile {
    id: String,                         // UUID
    name: String,
    planner_provider: String,           // "anthropic", "openai", etc.
    planner_model: String,              // "claude-3-opus", "gpt-4", etc.
    worker_provider: String,
    worker_model: String,
    active: bool,
}
```

#### LLM Pricing Model
```rust
struct LlmPricing {
    id: String,
    provider: String,
    model: String,
    cost_per_input_token: f64,          // Cost per 1M input tokens
    cost_per_output_token: f64,         // Cost per 1M output tokens
    cost_per_reasoning_token: Option<f64>,  // For o1, R1 models
}
```

### Configuration Schema
```toml
[llm]
provider = "anthropic"
model = "claude-3-sonnet"
api_key = "${ANTHROPIC_API_KEY}"
api_base = "https://api.anthropic.com/v1"
temperature = 0.7
max_tokens = 1000

[execution]
streaming = true
workspace_root = "/path/to/workspace"
max_concurrent_tasks = 3
task_timeout = 300
max_iterations = 5

[budget]
type = "credit"
amount = 500.0
enforcement = "block"

[workflow]
# Workflow-specific settings

[logging]
level = "info"
format = "json"
colored = false
timestamps = true
```

## Integration Points

### External LLM Providers
- Anthropic API (Claude models)
- OpenAI API (GPT models)
- Local: Ollama, llama.cpp, LM Studio
- OpenRouter (multi-provider routing)

### Tool Execution
- Shell commands
- File operations
- API calls
- Custom Rust functions
- Python script execution (via shell)

### State Persistence
- SQLite (primary for Orca)
- PostgreSQL (optional for distributed orchestrator)

## Success Metrics

1. **Functionality**:
   -  Budget creation and enforcement working
   -  LLM profile management functional
   -  Cost tracking accurate per request
   -  All integration tests passing (69+ tests)

2. **Usability**:
   - Simple config-driven setup
   - Clear budget status and cost information
   - Helpful error messages and warnings

3. **Reliability**:
   - Budget enforcement prevents overspending
   - Checkpoint recovery works correctly
   - Cost tracking accurate to 2 decimal places

4. **Performance**:
   - Workflow execution with minimal overhead
   - Cost calculation in <10ms
   - Budget status checks < 5ms

## Implementation Status

### Completed 
- Core workflow execution (Pregel BSP model)
- Budget management system (credit and recurring)
- LLM profile management (planner/worker configs)
- Pricing database and cost calculation
- Integration test suite (69 tests, all passing)
- Database schema and migrations
- Budget enforcement (warn and block modes)
- Multi-LLM support infrastructure

### In Progress =
- TUI configuration menus (setup wizard)
- Executor integration with budget tracking
- Advanced cost analytics and reporting

### Planned =Ë
- Cost forecasting and alerts
- Multi-account/project support
- Web dashboard (optional)
- Cost optimization recommendations
- Advanced usage analytics

## Testing & Quality

### Test Coverage
- **Budget System**: 10 integration tests
- **Budget Enforcement**: 9 integration tests
- **LLM Profiles**: 7 integration tests
- **Pricing Service**: 9 integration tests
- **Concurrency**: 7 integration tests
- **Other**: 27+ integration tests
- **Total**: 69+ integration tests, all passing

### Testing Strategy
- Integration tests with in-memory SQLite databases
- Repository pattern with dependency injection
- Service layer for business logic
- Async/await with Tokio test framework

## Deployment

### Installation
```bash
# Build from source
cargo build -p orca --release

# Or use installation script
./scripts/build-orca.sh --install
```

### First Run
1. Create `.orca/orca.toml` (or use default)
2. Set `ANTHROPIC_API_KEY` environment variable
3. Run Orca - database auto-initializes
4. Pricing data auto-loads on first run

### Configuration
- Edit `~/.orca/orca.toml` for user-level settings
- Edit `./.orca/orca.toml` for project-level settings
- CLI flags can override config values

## Support & Documentation

- **Architecture**: See `docs/architecture.md`
- **Build Guide**: See `docs/BUILD.md`
- **Quick Start**: See `docs/running.md`
- **API Reference**: See `docs/endpoints.md`
- **Environment**: See `docs/environment.md`

## Glossary

- **Superstep** - Synchronization boundary in Pregel execution
- **Checkpoint** - Saved state snapshot for recovery
- **Reducer** - Function that merges state changes from parallel nodes
- **Direct Tool Bridge** - In-process tool execution without RPC
- **Budget Enforcement** - Policy for controlling spending (warn vs block)
- **Planner LLM** - LLM used for reasoning and planning
- **Worker LLM** - LLM used for execution and task completion
