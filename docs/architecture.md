# Architecture Documentation

High-level overview of acolib system design, components, and data flow.

## Table of Contents

- [System Overview](#system-overview)
- [Component Architecture](#component-architecture)
- [Data Flow](#data-flow)
- [Execution Model](#execution-model)
- [State Management](#state-management)
- [Design Decisions](#design-decisions)
- [Technology Stack](#technology-stack)

## System Overview

acolib is a Rust-based platform for building and executing stateful AI agent workflows. It implements the Pregel execution model adapted for LLM-driven applications.

### Core Principles

1. **Explicit State Management** - All state changes explicit and trackable
2. **Checkpoint-Based Resilience** - Automatic persistence at each step
3. **Streaming-First** - Real-time event emission during execution
4. **Composability** - Reusable components at all levels
5. **Type Safety** - Rust's type system prevents many classes of errors

### Architecture Layers

```
┌─────────────────────────────────────────────────┐
│  User Interfaces (Web UI, TUI, CLI)             │
├─────────────────────────────────────────────────┤
│  API Layer (REST, WebSocket)                     │
├─────────────────────────────────────────────────┤
│  Orchestration Layer (Task/Workflow Management)  │
├─────────────────────────────────────────────────┤
│  Execution Engine (Pregel, Streaming)            │
├─────────────────────────────────────────────────┤
│  State Management (Reducers, Messages)           │
├─────────────────────────────────────────────────┤
│  Persistence Layer (Checkpoints, Database)       │
├─────────────────────────────────────────────────┤
│  Integration Layer (LLM, Tools, External APIs)   │
└─────────────────────────────────────────────────┘
```

## Component Architecture

### 1. langgraph-core

**Core graph execution engine.** Implements the Pregel model with streaming support.

```
langgraph-core/
├── builder.rs         # StateGraph builder
├── graph.rs           # Compiled graph execution
├── pregel/            # Pregel execution model
│   ├── executor.rs    # Superstep-based executor
│   ├── channel.rs     # Message channels
│   └── barrier.rs     # Synchronization points
├── state.rs           # State definition and reducers
├── messages.rs        # Message types and handling
├── tool.rs            # Tool registry and execution
├── interrupt.rs       # Human-in-the-loop
├── stream.rs          # Streaming infrastructure
└── send.rs            # Dynamic parallelism
```

**Key Responsibilities:**
- Graph construction and compilation
- Node and edge management
- Pregel-based execution
- State reduction and merging
- Streaming event emission
- Checkpoint coordination

### 2. langgraph-checkpoint

**Persistence abstraction layer.**

```
langgraph-checkpoint/
├── traits.rs          # CheckpointSaver trait
├── channel/           # Channel types
│   ├── last_value.rs  # Last write wins
│   ├── append.rs      # Append mode
│   ├── merge.rs       # Deep merge
│   └── sum.rs         # Accumulation
└── saver/
    ├── memory.rs      # In-memory (testing)
    ├── sqlite.rs      # SQLite backend
    └── postgres.rs    # PostgreSQL backend
```

**Key Responsibilities:**
- Trait definitions for checkpoint saving
- Channel type implementations
- State serialization/deserialization
- Checkpoint metadata management

### 3. langgraph-prebuilt

**Pre-built agent patterns.**

```
langgraph-prebuilt/
├── agents/
│   ├── react.rs           # ReAct pattern
│   ├── plan_execute.rs    # Plan-Execute pattern
│   └── reflection.rs      # Reflection pattern
├── builders/
│   ├── agent_builder.rs   # Common builder
│   └── router_builder.rs  # Routing builder
└── models/
    └── agent_config.rs    # Agent configuration
```

**Key Responsibilities:**
- ReAct agent: Think → Act → Observe
- Plan-Execute agent: Plan → Execute → Replan
- Reflection agent: Generate → Critique → Refine
- Builder patterns for easy customization

### 4. orchestrator

**Task and workflow coordination.**

```
orchestrator/
├── task.rs            # Task definition
├── workflow.rs        # Workflow definition
├── executor.rs        # Execution orchestration
├── manager.rs         # Task/workflow lifecycle
└── store.rs           # In-memory storage
```

**Key Responsibilities:**
- Task lifecycle management
- Workflow scheduling and execution
- Dependency resolution
- Resource allocation

### 5. aco

**Web UI, TUI, and CLI binaries.**

```
aco/
├── web/               # Web UI (React/Angular)
├── tui/               # Terminal UI (ratatui)
├── cli/               # Command-line interface
└── server.rs          # API server
```

**Key Responsibilities:**
- Web dashboard
- Terminal interface
- Command-line tools
- REST API implementation

### 6. llm

**LLM provider integration.**

```
llm/
├── remote/            # Cloud-hosted LLM APIs
│   ├── claude.rs      # Anthropic Claude models (Claude 3 Opus, Sonnet, Haiku)
│   ├── openai.rs      # OpenAI models (GPT-4, GPT-3.5, o1)
│   ├── gemini.rs      # Google Gemini models (Gemini Pro, Gemini Pro Vision)
│   ├── grok.rs        # xAI Grok models
│   ├── deepseek.rs    # Deepseek models including R1 (thinking model)
│   └── openrouter.rs  # Unified API for multiple providers
├── local/             # Local LLM servers
│   ├── ollama.rs      # Ollama local LLM runner
│   ├── llama_cpp.rs   # llama.cpp server integration
│   └── lmstudio.rs    # LM Studio local interface
├── config.rs          # Provider configuration
├── error.rs           # Provider errors
└── provider_utils.rs  # Shared utilities
```

**Key Responsibilities:**
- Remote provider integration (6 providers)
  - Claude (Anthropic)
  - OpenAI (GPT-4, o1)
  - Gemini (Google)
  - Grok (xAI)
  - Deepseek (including R1 thinking model)
  - OpenRouter (unified API)
- Local provider integration (3 providers)
  - Ollama
  - llama.cpp
  - LM Studio
- Thinking model support (o1, R1 series) with reasoning extraction
- Token counting and usage tracking
- Streaming and non-streaming responses
- Error handling with retries and backoff
- Unified ChatModel trait interface

### 7. tooling

**Utilities and helpers.**

```
tooling/
├── config.rs          # Configuration management
├── logging.rs         # Logging setup
├── utils.rs           # General utilities
└── errors.rs          # Error types
```

**Key Responsibilities:**
- Configuration loading and validation
- Logging initialization
- Common utilities
- Error definitions

### 8. utils

**Common utilities shared across crates.**

## Data Flow

### Execution Flow

```
1. User Request
   ├─ Task Creation → Database
   ├─ Workflow Definition → Database
   └─ Execution Start Request

2. Graph Compilation
   ├─ Validate structure (start → nodes → end)
   ├─ Create execution context
   └─ Initialize channels

3. Pregel Execution (Superstep Loop)
   ├─ Super step S:
   │  ├─ Activate eligible nodes
   │  ├─ Execute in parallel
   │  ├─ Collect outputs
   │  ├─ Reduce state changes
   │  ├─ Create checkpoint
   │  └─ Emit stream events
   └─ Repeat until all nodes inactive

4. Stream Emissions
   ├─ Values: Full state snapshots
   ├─ Updates: Incremental changes
   ├─ Checkpoints: Persistence events
   ├─ Tokens: LLM token streaming
   ├─ Debug: Execution trace
   └─ Custom: User-defined events

5. Result Storage
   ├─ Final state → Database
   ├─ Messages → Database
   ├─ Checkpoints → Checkpoint Store
   └─ Metrics → Analytics
```

### Message Flow (Conversational)

```
User Input
    ↓
Add to messages (append reducer)
    ↓
LLM Process (prompt + context)
    ↓
Parse Response
    ↓
Tool Call? → Execute Tool → Tool Result → Add to messages
    ↓
Add LLM Response to messages (deduplication)
    ↓
Should Continue? → No → Done
                → Yes → Repeat
```

### State Reduction

```
Multiple parallel nodes emit state changes
    ↓
State Reducer (by field)
    ↓
┌─ AppendReducer: [...old] + [...new]      (messages)
├─ OverwriteReducer: new                    (status)
├─ MergeReducer: deep_merge(old, new)      (state obj)
└─ SumReducer: old + new                    (counters)
    ↓
Updated State
```

## Execution Model

### Pregel Model (Superstep-Based)

acolib implements Google's Pregel BSP (Bulk Synchronous Parallel) model:

```
Initialization:
  - All nodes start as "active"
  - State initialized with input

Loop while active nodes exist:
  Superstep S:
    1. Each active node receives messages
    2. Nodes execute in parallel
    3. Nodes emit messages to next nodes
    4. Barrier synchronization
    5. Checkpoint created
    6. Stream events emitted
    7. Mark nodes for next superstep
    8. Check for halting condition

Termination:
  - All nodes inactive
  - No pending messages
  - Final state = result
```

### Node Activation

```
Node becomes ACTIVE when:
  - Has incoming messages
  - Execution just started (start node)
  - Previous node produced state change

Node becomes INACTIVE when:
  - Executed and no output change
  - Or explicitly halted
```

### Parallelism Boundaries

```
Within Superstep:
  ✓ Parallel node execution
  ✓ Parallel tool calls
  ✓ Parallel message processing

Between Supersteps:
  ✗ Barrier synchronization
  ✗ State reduction must complete
  ✗ Checkpoint must persist
```

## State Management

### State Definition

```rust
// Implicit state (HashMap<String, Value>)
let mut state = State::default();
state["messages"] = Value::List(...);

// Reducers define merge strategy
// Each field has a reducer type
```

### Reducer Types

| Reducer | Operation | Use Case |
|---------|-----------|----------|
| `AppendReducer` | `[...a] + [...b]` | Message history |
| `OverwriteReducer` | `b` | Current status |
| `MergeReducer` | `deep_merge(a, b)` | Nested objects |
| `SumReducer` | `a + b` | Counters |
| `Custom` | User-defined | Domain-specific |

### Message System

```
Message Types:
  - SystemMessage: Instructions to LLM
  - HumanMessage: User input
  - AssistantMessage: LLM response
  - ToolMessage: Tool result
  - ToolCallMessage: Tool invocation

Processing:
  1. Accumulate all messages
  2. Deduplicate via add_messages reducer
  3. Trim context window if needed
  4. Pass to LLM
```

### Checkpoint Creation

```
After each superstep:
  1. Serialize current state
  2. Record timestamp
  3. Record node that produced it
  4. Store in checkpoint store
  5. Emit checkpoint event

Benefits:
  - Resume from any checkpoint
  - Audit trail
  - Debugging aid
  - Human approval points
```

## Design Decisions

### Why Pregel Model?

- **Explicit Synchronization** - Clear execution boundaries
- **Checkpoint Friendly** - Natural points to persist state
- **Streaming Support** - Events emitted at superstep end
- **Testability** - Deterministic execution
- **Distributed Ready** - Design scales to multi-machine

### Why Async-First?

- **Token Streaming** - Real-time LLM output
- **Parallel Execution** - Within supersteps
- **I/O Efficiency** - Non-blocking network calls
- **UI Responsiveness** - Non-blocking server

### Why Type Safety?

- **Compile-time Checks** - Catch errors early
- **Refactoring Safety** - Compiler validates changes
- **Documentation** - Types are explicit contracts
- **Performance** - Optimization opportunities

### Why Traits?

- **Extensibility** - Custom checkpoint savers
- **Testing** - Mock implementations
- **Provider Agnostic** - Swap LLM providers
- **Decoupling** - Components don't depend on implementations

## Technology Stack

### Core Runtime
- **async runtime**: tokio 1.35+
- **Executor**: tokio task spawning
- **Synchronization**: tokio channels and mutexes

### Serialization
- **JSON**: serde_json
- **YAML**: serde_yaml
- **Binary**: bincode

### Web
- **HTTP**: reqwest 0.12 (client), actix-web or axum (server)
- **WebSocket**: tokio-tungstenite
- **Web UI**: React/Angular (JavaScript frontend)
- **TUI**: ratatui

### Database
- **PostgreSQL**: sqlx async driver
- **SQLite**: rusqlite or sqlx
- **Migrations**: sqlx migrations

### LLM Integration
- **OpenAI**: reqwest + serde
- **Anthropic**: reqwest + serde
- **Tokens**: tiktoken-rs

### Testing & Quality
- **Testing**: tokio::test, criterion benchmarks
- **Property Testing**: proptest
- **Linting**: clippy
- **Formatting**: rustfmt

### Configuration
- **Config Files**: serde_yaml
- **Environment**: dotenv + envy
- **Logging**: tracing + tracing-subscriber

## Performance Characteristics

### Execution Overhead
- **Node execution**: < 1ms overhead per node
- **State reduction**: O(n) where n = state fields
- **Checkpointing**: ~10ms for typical state
- **Streaming**: < 1ms per event

### Scalability
- **Nodes per workflow**: 100+ nodes tested
- **Parallel nodes**: Limited by machine cores
- **Message throughput**: 10K+ messages/sec
- **State size**: 100MB+ supported

### Memory Usage
- **Per workflow**: ~1MB base + state size
- **Per execution**: 5-10MB typical
- **Per checkpoint**: ~1KB metadata + state

### Network
- **API latency**: 1-5ms typical
- **WebSocket**: Real-time streaming
- **Connection pooling**: 10 connections default

## Future Architecture Evolution

### v0.3.0
- Multiple-machine execution support
- Distributed checkpoint store
- Advanced DAG optimization
- GPU-accelerated nodes

### v0.4.0
- Multi-agent coordination
- Workflow composition patterns
- Plugin system
- Custom node languages

### v1.0.0
- Production hardening
- Enterprise features
- Advanced analytics
- Open-source community

---

**For implementation details**, see code in `crates/` directories.
**For API reference**, see [docs/api/](api/).
**For developer setup**, see [CONTRIBUTING.md](../CONTRIBUTING.md).
