
# Tool Runtime SDK — Library Specification (v1)

**Purpose.** A neutral, implementation‑agnostic SDK for discovering, invoking, and auditing developer tools
(filesystem, git, AST, shell, HTTP, grep, metrics) under consistent contracts and rules.

**Intended use.** Embed this SDK in any **Runtime Host** (daemon, service, CLI) and call it from any **Client**
(UI, agent). An optional **Coordinator** can plan steps and query metrics/budgets. No product‑specific naming.

---

## 1. Roles & Concepts

- **Client** — Issues tool calls (e.g., UI, agent).
- **Runtime Host** — Loads the SDK and executes tools.
- **Coordinator** — Optional planner/enforcer for budgets and policy.
- **Session** — Authenticated context for a set of tool calls.
- **Registry** — Declarative config for tools, rules, and policies.

---

## 2. Messages & Envelopes (JSON)

Transport is out‑of‑scope (WebSocket/HTTP/Unix socket all fine).

**ToolRequest**
```json
{
  "type": "ToolRequest",
  "tool": "file_read",
  "args": {"path": "src/main.rs"},
  "request_id": "uuid",
  "session_id": "sess-123",
  "timestamp": 1730822400
}
```

**ToolResponse**
```json
{
  "type": "ToolResponse",
  "ok": true,
  "tool": "file_read",
  "request_id": "uuid",
  "duration_ms": 34,
  "data": {"content": "fn main() {}", "sha256": "…"},
  "errors": []
}
```

**EventMessage**
```json
{
  "type": "EventMessage",
  "event": "TaskProgress",
  "request_id": "uuid",
  "progress": {"pct": 65, "message": "running tests"}
}
```

**ErrorMessage**
```json
{
  "type": "ErrorMessage",
  "code": "E_FILE_IO",
  "message": "File not found: src/lib.rs",
  "request_id": "uuid"
}
```

**Heartbeat**
```json
{
  "type": "Heartbeat",
  "session_id": "sess-123",
  "timestamp": 1730822430
}
```

---

## 3. Errors & Policy Gates

**Canonical error codes:** `E_FILE_IO`, `E_AST_PARSE`, `E_AST_EDIT`, `E_VALIDATION_FAIL`, `E_GIT`, `E_HTTP`, `E_SHELL`, `E_POLICY`, `E_TIMEOUT`, `E_INTERNAL`.

**Policy examples (enforced by SDK):**
- Network egress must match domain allowlist.
- Shell commands must match allowlist regex; redact sensitive env.
- Paths must resolve under the workspace root (unless override).
- Mutating git operations require a clean tree and validation pass.

---

## 4. Registry Schema (YAML)

Load per‑session or at startup.

```yaml
version: 1

network:
  allowed_domains: ["doc.rust-lang.org", "crates.io"]

shell_allow:
  - "^cargo(\s|$)"
  - "^npm(\s|$)"
  - "^rg(\s|$)"

git:
  allow_push: true
  require_clean_tree_for_commit: true

ast:
  language: rust
  format_on_save: true
  validate_on_edit: true

validators:
  - rule: sec.network.allowlist
    enforcement: blocking
  - rule: sec.shell.allowlist
    enforcement: blocking
  - rule: sec.paths.sandbox
    enforcement: blocking
  - rule: code.ast.parsed
    enforcement: blocking
  - rule: build.must.pass
    enforcement: blocking
  - rule: lint.must.pass
    enforcement: blocking
  - rule: fmt.must.pass
    enforcement: blocking
  - rule: secrets.scan
    enforcement: blocking
  - rule: code.todo.blockers
    enforcement: blocking
```

---

## 5. Tool Catalog (Contracts)

### 5.1 Filesystem

**fs_list**
```json
{"glob": "src/**/*.rs", "max_results": 5000, "include_hidden": false}
```
→ `{"files": ["src/lib.rs", "src/auth/mod.rs"]}`

**fs_move**
```json
{"src": "path/from", "dst": "path/to", "overwrite": false}
```
→ `{"moved": true}`

**fs_copy**
```json
{"src": "path/from", "dst": "path/to", "overwrite": false, "preserve_mode": true}
```
→ `{"copied": true}`

**fs_delete**
```json
{"path": "path/to/delete", "recursive": false, "force": false}
```
→ `{"deleted": true}`

**file_read**
```json
{"path": "src/lib.rs", "max_bytes": 1048576}
```
→ `{"content": "...", "sha256": "…"}`

**file_write**
```json
{
  "path": "src/auth.rs",
  "content": "…",
  "attribution": {"task_id": 42},
  "create_dirs": true,
  "mode_octal": "0644"
}
```
→ `{"written": true, "bytes": 1024}`

**file_patch**
```json
{"path": "src/lib.rs", "unified_diff": "..."}
```
→ `{"patched": true, "hunks_applied": 3}`

**grep**
```json
{"pattern":"panic!", "glob":"src/**", "case_sensitive":false, "max_results":1000}
```
→ `{"matches":[{"file":"src/x.rs","line":12,"col":8,"snippet":"…"}]}`

### 5.2 Processes & Shell

**proc_list**
```json
{"filter": "cargo|node", "limit": 50}
```
→ `{"processes":[{"pid":1234,"cmd":"cargo build","cpu":0.3,"mem_mb":120.4}]}`

**shell_exec**
```json
{"cmd":"cargo test -q","cwd":".","timeout_ms":600000,"env":{"RUSTFLAGS":""},"stdin":null,"allow_network":false}
```
→ `{"code":0,"stdout":"…","stderr":""}`

### 5.3 Network

**curl**
```json
{"method":"GET","url":"https://doc.rust-lang.org/std/","headers":{"User-Agent":"tool-runtime-sdk/1.0"},"body":null,"timeout_ms":15000,"max_bytes":5242880}
```
→ `{"status":200,"headers":{"content-type":"text/html"},"body_b64":"…","truncated":false}`

### 5.4 Git (full)

**git_status**
```json
{"porcelain": true}
```
→ `{"branch":"feat/x","ahead":1,"behind":0,"changes":[{"path":"src/lib.rs","status":"M"}]}`

**git_diff**
```json
{"rev":"HEAD","paths":["src/**"]}
```
→ `{"patch":"diff --git …"}`

**git_add**
```json
{"paths":["src/**"]}
```
→ `{"added":12}`

**git_commit**
```json
{"message":"feat(auth): add token parser","signoff":true,"allow_empty":false}
```
→ `{"commit":"abcd1234"}`

**git_checkout**
```json
{"target":"feature/branch","create":false,"track":true}
```
→ `{"checked_out":true}`

**git_merge**
```json
{"branch":"main","no_ff":true}
```
→ `{"merged":true,"conflicts":[]}`

**git_rebase**
```json
{"onto":"origin/main","autosquash":true}
```
→ `{"rebased":true}`

**git_push**
```json
{"remote":"origin","refspec":"HEAD:refs/heads/feature","force":false}
```
→ `{"pushed":true}`

**git_pull**
```json
{"remote":"origin","rebase":true}
```
→ `{"pulled":true}`

### 5.5 AST

**ast_generate**
```json
{"paths":["src/**/*.rs"],"language":"rust","incremental":true,"max_files":5000}
```
→ `{"parsed_files":123,"errors":[]}`

**ast_query**
```json
{"where":{"type":"fn","name_like":"auth*","file_glob":"src/**"},"limit":200}
```
→ `{"nodes":[{"id":"n1","type":"fn","name":"auth_login","file":"src/auth.rs","span":{"start":120,"end":240}}]}`

**ast_edit**
```json
{"operation":"rename_symbol","target_node_id":"n1","new_name":"auth_signin","format_on_save":true,"validate_build":true}
```
→ `{"edited":true,"files_touched":["src/auth.rs"]}`

**ast_validate**
```json
{"rules":[{"id":"exported_fn_docs","enforcement":"blocking","where":{"type":"fn","exported":true},"must_have":["docstring"]}]}
```
→ `{"violations":[{"rule":"exported_fn_docs","node_id":"n42","message":"Missing rustdoc"}]}`

**ast_external**
```json
{"provider":"rust-analyzer","request":{"method":"textDocument/documentSymbol","params":{"uri":"file:///…"}}}
```
→ `{"provider":"rust-analyzer","response":{"…":"…"}}`

---

## 6. Tooling Summary Table (SDK)

| **Category** | **Tool Name** | **Purpose / Function** | **Key Args** | **Output / Result** | **Notes / Policies** |
|---------------|---------------|------------------------|---------------|----------------------|-----------------------|
| **Filesystem** | `fs_list` | List files by pattern | `glob`, `max_results`, `include_hidden` | `files[]` | Sandboxed to workspace |
|  | `fs_move` | Move a file/dir | `src`, `dst`, `overwrite` | `moved` | Write locks |
|  | `fs_copy` | Copy file/dir | `src`, `dst`, `overwrite`, `preserve_mode` | `copied` | Preserve perms optional |
|  | `fs_delete` | Delete file/dir | `path`, `recursive`, `force` | `deleted` | Safe defaults |
|  | `file_read` | Read file | `path`, `max_bytes` | `content`, `sha256` | Size‑limited |
|  | `file_write` | Write file | `path`, `content`, `attribution`, `create_dirs` | `written`, `bytes` | Triggers AST refresh |
|  | `file_patch` | Apply patch | `path`, `unified_diff` | `patched`, `hunks_applied` | |
|  | `grep` | Search text | `pattern`, `glob`, `case_sensitive`, `max_results` | `matches[]` | Skips binary/vendor by default |
| **Proc & Shell** | `proc_list` | List processes | `filter`, `limit` | `processes[]` | Redacts secrets |
|  | `shell_exec` | Run command | `cmd`, `cwd`, `timeout_ms`, `env` | `code`, `stdout`, `stderr` | Allowlisted |
| **Network** | `curl` | HTTP(S) client | `method`, `url`, `headers`, `body`, `timeout_ms` | `status`, `headers`, `body_b64` | Domain allowlist |
| **Git** | `git_status` | Working tree status | `porcelain` | `branch`, `changes[]` | Read‑only |
|  | `git_diff` | Diff vs rev | `rev`, `paths` | `patch` | |
|  | `git_add` | Stage files | `paths` | `added` | |
|  | `git_commit` | Commit changes | `message`, `signoff`, `allow_empty` | `commit` | Validation gate |
|  | `git_checkout` | Switch branch | `target`, `create`, `track` | `checked_out` | Dirty tree blocked |
|  | `git_merge` | Merge branch | `branch`, `no_ff` | `merged`, `conflicts[]` | |
|  | `git_rebase` | Rebase | `onto`, `autosquash` | `rebased` | Clean tree required |
|  | `git_push` | Push | `remote`, `refspec`, `force` | `pushed` | Policy‑gated |
|  | `git_pull` | Pull | `remote`, `rebase` | `pulled` | Policy‑gated |
| **AST** | `ast_generate` | Build AST | `paths`, `language`, `incremental` | `parsed_files`, `errors[]` | Updates indices |
|  | `ast_query` | Query AST | `where`, `limit` | `nodes[]` | |
|  | `ast_edit` | Structural edit | `operation`, `target_node_id`, `new_name` | `edited`, `files_touched[]` | Auto‑format/validate |
|  | `ast_validate` | Rule checks | `rules[]` | `violations[]` | Blocking/warning |
|  | `ast_external` | External AST | `provider`, `request` | `response` | LSP/Semgrep bridge |
| **Validation** | `validate` | Run validators | `rule_id`, `enforcement`, `glob` | `violations[]` | Enqueues fixes |
| **Metrics** | `emit_metrics` | Push metrics | `interval`, `session_id` | `status` | Rollups |

---

## 7. Core Rules Catalog (SDK)

> Enforced via `validate`, `ast_validate`, git guards, and policy checks. Levels: **blocking**, **warning**, **suggestion**.

| Rule ID | Category | Level | Trigger | Description | Primary Tool |
|---|---|---|---|---|---|
| sec.network.allowlist | Security / Network | blocking | curl | host in allowlist; redirects ≤ 3 | curl |
| sec.shell.allowlist | Security / Shell | blocking | shell_exec | cmd matches allowlist; redact env | shell_exec |
| sec.paths.sandbox | Security / FS | blocking | FS ops | path under workspace root | fs_* |
| sec.git.clean_tree | Git | blocking | commit/push/rebase/merge | no unstaged changes; validation pass | git_* |
| code.ast.parsed | AST | blocking | after write/patch | files parse without errors | ast_generate |
| build.must.pass | Build | blocking | on commit | build succeeds | shell_exec |
| lint.must.pass | Lint | blocking | on commit | linter passes | shell_exec |
| fmt.must.pass | Formatting | blocking | on commit | formatter check passes | shell_exec |
| code.todo.blockers | Hygiene | blocking | on commit | disallow TODO/FIXME in protected branches | grep |
| test.required.changed | Testing | blocking | on commit | code change requires test change or override | git_diff, grep |
| secrets.scan | Security | blocking | on commit | diff secrets scan | shell_exec |
| code.fn.docs.exported | Docs | blocking | on commit | exported fns must have docs | ast_validate |
| code.no.panic.in.lib | Safety | warning | on commit | no panic! in library code | ast_validate |
| code.no.unwrap | Safety | warning | on commit | flag .unwrap() in non-tests | ast_validate |
| code.mod.import.rules | Architecture | blocking | on commit | enforce import boundaries | ast_validate |
| commit.message.conventional | Git | warning | commit | Conventional Commits | git_commit |
| branch.naming.convention | Git | warning | checkout | branch naming patterns | git_checkout |

**Rule Pack example**
```yaml
validators:
  - rule: sec.network.allowlist
    enforcement: blocking
  - rule: sec.shell.allowlist
    enforcement: blocking
  - rule: sec.paths.sandbox
    enforcement: blocking
  - rule: sec.git.clean_tree
    enforcement: blocking
  - rule: code.ast.parsed
    enforcement: blocking
  - rule: build.must.pass
    enforcement: blocking
  - rule: lint.must.pass
    enforcement: blocking
  - rule: fmt.must.pass
    enforcement: blocking
  - rule: secrets.scan
    enforcement: blocking
```

---

## 8. Lifecycle & Auditing

1. **Plan (optional)** — Coordinator proposes tool calls (outside SDK scope).  
2. **Pre‑checks** — Policy/budget checks, file locks.  
3. **Execute** — SDK runs tool; logs timeline `{tool, args_hash, start_ts, end_ts}`.  
4. **Sync** — AST reparse on code change; update indices.  
5. **Validate** — Run rules; emit violations.  
6. **Emit** — Metrics via `EventMessage` or metrics channel.

**Locks:** Per‑file write locks; AST writes serialize.  
**Metrics:** Request count, duration, files touched, errors, bytes, etc.

---

## 9. Defaults & Limits

- FS 10s; Git 30s; Shell 10m; Curl 15s; AST gen 5m; AST edit 30s; Grep 10s.  
- Max stdout capture 5 MiB; Max HTTP body 5 MiB.  
- `proc_list` returns ≤50 rows unless specified.

---

## 10. MCP Compatibility (Optional)

The SDK envelopes map 1:1 to MCP message types; sessions and security apply identically.

---

## 11. Versioning

- Semantic: **MAJOR.MINOR.PATCH**.  
- MAJOR for breaking changes; MINOR for new tools; PATCH for docs/typos.
