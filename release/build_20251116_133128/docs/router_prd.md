# YAML–Configured Agentic Router Framework (Option B)
**Version:** 1.1  
**Owner:** Solutions Architecture / Agentic Orchestrator  
**Status:** Draft PRD  
**Last Updated:** 2025-11-05  

---

## 1. Overview
This document defines the design and requirements for the **Router / Supervisor Pattern**, a core orchestration component of `rLangGraph`.  
It dynamically dispatches reasoning or tooling patterns (ReAct, LATS, CodeAct, Reflection, etc.) using YAML configuration.  

The Router operates as a **Coordinator and Policy Gateway** between agentic reasoning layers and the **Tool Runtime SDK** (see `tools_runtime_sdk.md`), which executes validated system-level actions (filesystem, git, AST, network, shell).

---

## 2. Goals

| Goal | Description |
|-|-|
| **Declarative routing** | Rules, prompts, and policies defined in YAML; no hard-coded logic |
| **Runtime tool integration** | Router invokes Tool Runtime SDK operations through pattern graphs |
| **Multi-pattern execution** | Sequential or context-driven dispatch to multiple sub-graphs |
| **Safety & policy enforcement** | SDK policy gates (network, git, shell, AST) guard each invocation |
| **Auditability & replay** | Every ToolRequest/Response logged under Router session |
| **Extensibility** | New patterns and tools can be registered without recompile |

---

## 3. Relationship to Tool Runtime SDK
- Router acts as a **client of the SDK** — it formats ToolRequests and submits them to the SDK’s Runtime Host.  
- The SDK provides canonical contracts (`file_read`, `git_diff`, `shell_exec`, `ast_query`, etc.) and enforces policy gates.  
- Each sub-pattern (ReAct, CodeAct, etc.) calls SDK tools through the Router’s execution context.  
- Errors, metrics, and violations bubble back as structured `ToolResponse` and `ErrorMessage` events.  
- The Router records a tool timeline for auditing (see SDK section 8: Lifecycle & Auditing).

---

## 4. Architecture Layers

```mermaid
graph TD
U[User Message]
R[Router Pattern (YAML)]
subgraph rLangGraph Runtime
  P1[ReAct]
  P2[LATS]
  P3[CodeAct]
  P4[Reflection]
  P5[ToolValidation]
end
SDK[Tool Runtime SDK Host (daemon/CLI)]
OS[(Filesystem/Git/Network/Shell)]

U --> R --> P1 & P2 & P3 --> SDK --> OS
R --> Log[Audit & Metrics]
```

---

## 5. Execution Flow
1. Load merged YAML config (`$include`, `${ENV}`).  
2. Extract Router pattern (`find_pattern_by_id`).  
3. Deserialize to `SupervisorConfig`.  
4. Router evaluates rules → selects patterns → invokes their compiled graphs.  
5. Each pattern uses Tool Runtime SDK calls for I/O, Git, AST, etc.  
6. Responses are merged back into Router state.  
7. Termination conditions checked (`contains`, `max_steps`, etc.).  
8. Final state and ToolResponses emitted for logging.

---

## 6. Component Responsibilities

| Component | Responsibility |
|-|-|
| **Router (YAML)** | Rule evaluation, pattern selection, termination |
| **rLangGraph Pattern Graph** | Reasoning, planning, LLM interaction |
| **Tool Runtime SDK** | Safe execution of system operations |
| **Coordinator (optional)** | Budgeting, rate limiting, policy override |
| **Audit Logger** | Persists ToolRequests/Responses for traceability |

---

## 7. Router YAML Schema (Simplified)
See `config/patterns/router.yaml`.

```yaml
type: supervisor
id: router_v1
registry:
  allow: [tv_for_logs, lats_diagnose, codeact_static, reflection_basic, react_default]
settings:
  route_policy:
    rules:
      - name: logs
        when: { any: [{ text: "/\b(log|stacktrace)\b/i" }] }
        prefer: [tv_for_logs, lats_diagnose]
    default: [react_default]
  termination: { any: [{ max_steps: 8 }] }
  guards: { enforce_registry: true }
```

---

## 8. SDK Integration Points

| Router Stage | SDK Tool Called | Purpose |
|-|-|-|
| `tv_for_logs` | `file_read`, `grep` | Load log files securely |
| `lats_diagnose` | `ast_query`, `shell_exec(cargo test)` | Analyze root cause |
| `codeact_static` | `ast_edit`, `file_patch`, `git_diff` | Generate and validate code patch |
| `reflection_basic` | `validate`, `lint.must.pass` | Check correctness and standards |
| `react_default` | (no SDK call) | General reasoning path |

All SDK calls return `ToolResponse` objects with policy-enforced results.

---

## 9. Error and Policy Model

The Router inherits SDK canonical error codes:

| Code | Meaning |
|-|-|
| `E_FILE_IO` | Filesystem error |
| `E_AST_PARSE` | AST generation failed |
| `E_GIT` | Git operation failed |
| `E_POLICY` | Policy violation (blocking) |
| `E_SHELL` | Command execution failed |
| `E_TIMEOUT` | Tool timeout |
| `E_INTERNAL` | Unhandled runtime error |

Router policy handlers may retry, abort, or downgrade errors to warnings based on severity.

---

## 10. Security and Compliance
- All tool calls must pass SDK allow-list checks (domains, shell regex, sandbox paths).  
- Git mutations require validation rules from `validators:` section.  
- Sensitive data redacted before logging (`E_POLICY` if not).  
- AST operations validated against language rules (`ast_validate`).  
- All ToolRequests include `session_id` and `timestamp`.  

---

## 11. Performance Targets
| Metric | Target |
|-|-|
| Routing decision | ≤ 2 ms |
| Tool invocation overhead | ≤ 5 ms beyond SDK execution |
| Concurrent sessions | ≥ 100 |
| SDK policy evaluation | ≤ 1 ms per call |
| Audit log write | asynchronous (< 50 ms) |

---

## 12. Future Extensions
| Area | Enhancement |
|-|-|
| **Workflow Hybrid** | Allow Router rules to launch Workflows (Option A) |
| **Budget Coordinator** | Integrate cost limits via SDK metrics |
| **LLM Scoring** | Confidence-based rule selection |
| **Dynamic tool discovery** | Query SDK registry at runtime |
| **Observability** | Prometheus metrics per Tool call |
| **Checkpoint Persistence** | Replace in-memory saver with SQL/Redis backends |

---

## 13. Acceptance Criteria

| ID | Requirement | Verification |
|-|-|-|
| R1 | Router loads YAML rules and registry | Unit test with base.yaml |
| R2 | Pattern dispatch via regex/context | Integration test CLI |
| R3 | All Tool Runtime SDK calls validated | Policy simulator |
| R4 | Audit log of ToolRequests | Log inspection |
| R5 | Termination on `contains` or `max_steps` | Simulation |
| R6 | Performance targets met | Bench results |
| R7 | Compliant with SDK error model | Error table match |

---

## 14. References
- `yaml_pattern_pack_router_plus_cli.zip`  
- `tools_runtime_sdk.md` (v1 Library Spec)  
- `config/patterns/router.yaml`  
- `src/router.rs`, `src/pattern_lookup.rs`, `src/config_loader.rs`  
- `src/bin/router_main.rs`  

---

**End of Document**
