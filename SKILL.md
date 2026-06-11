---
name: rlm
description: >
  Recursive Language Model for large codebases via CBRLM (codebase-rlm-memory-mcp).
  Built-in graph index + RLM map-reduce. Use rlm_filter, rlm_read_symbol, trace_path;
  rlm_scan/rlm_chunk for logs and huge files. Triggers: analyze codebase, scan all files,
  large repository, RLM, find usage across project, security audit at scale.
license: MIT
compatibility: opencode, codex, claude-code
metadata:
  mcp-server: codebase-rlm-memory-mcp
  abbrev: cbrlm
  standalone: true
  paper: https://arxiv.org/pdf/2512.24601
---

# RLM + CBRLM (codebase-rlm-memory-mcp)

**Context is external.** Use MCP tools — never bulk-read the repo into main context.

MCP server: **`codebase-rlm-memory-mcp`** · binary: **`cbrlm`** · abbrev: **CBRLM**

## Prerequisites

1. `codebase-rlm-memory-mcp` MCP server enabled (`cbrlm` binary)
2. Project indexed via `index_repository`
3. (Optional) Hooks installed via `install.ps1` / `install.sh` — SessionStart reminder + PreToolUse graph augment on Grep/Glob

**Project naming:** shares `~/.cache/codebase-memory-mcp` with upstream CBM, but CBRLM indexes use **`cbrlm+` prefix** (e.g. upstream `D-animejs-skills` → CBRLM `cbrlm+D-animejs-skills`). Pass either form; `cbrlm+` is added automatically.

## RLM loop (graph-native)

### Phase 0 — Index

```
index_repository(repo_path=".")
index_status(project="cbrlm+my-app")   # or upstream alias "my-app"
```

### Phase 1 — Filter

```
rlm_workflow(phase="filter")
search_graph(project, query="auth middleware", label="Function")
search_code(project, pattern="UserID", mode="files")
```

For logs/CSV: `rlm_scan(path)` → `rlm_peek(session_id, query)`

### Phase 2 — Map (parallel)

```
rlm_read_symbol(project, qualified_name="api.routes.createUser")
trace_path(project, function_name="handleAuth", direction="both")
rlm_chunk(session_id, offset=0, limit=3)
```

### Phase 3 — Reduce

```
rlm_workflow(phase="reduce")
detect_changes(project)
get_architecture(project)
```

## Tool map

| Task | MCP tool |
|------|----------|
| Index repo | `index_repository` |
| Check index | `index_status` |
| Filter symbols | `rlm_filter` / `search_graph` |
| Read one symbol | `rlm_read_symbol` |
| Trace calls | `trace_path` |
| Scan logs/CSV | `rlm_scan` / `rlm_peek` / `rlm_chunk` |
| Workflow help | `rlm_workflow` |

## Hooks (if installed)

| Event | Matcher | Effect |
|-------|---------|--------|
| SessionStart | startup/resume/clear/compact | Remind: graph tools first, `cbrlm+` names, `index_repository` if needed |
| PreToolUse | Grep\|Glob | Non-blocking `search_graph` augment via `cbrlm hook-augment` |

Hooks never block tool calls. Read is intentionally excluded from PreToolUse (preserve read-before-edit).

## Rules

1. Never load 10+ files into root context
2. `rlm_read_symbol` = one qualified_name per call
3. Prefer graph tools over `rg` when project is indexed
4. Use `rlm_scan`/`rlm_chunk` only for non-code blobs
5. Reduce to structured JSON before final answer