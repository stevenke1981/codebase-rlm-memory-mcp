---
name: rlm
description: >
  Recursive Language Model for large codebases via Rust MCP server codebase-memory-rlm-rs.
  Built-in graph index + RLM map-reduce. Use rlm_filter, rlm_read_symbol, trace_path;
  rlm_scan/rlm_chunk for logs and huge files. Triggers: analyze codebase, scan all files,
  large repository, RLM, find usage across project, security audit at scale.
license: MIT
compatibility: opencode, codex, claude-code
metadata:
  mcp-server: codebase-memory-rlm-rs
  standalone: true
  paper: https://arxiv.org/pdf/2512.24601
---

# RLM + codebase-memory-rlm-rs

**Context is external.** Use MCP tools — never bulk-read the repo into main context.

Single MCP server: graph index and RLM are built-in (no separate codebase-memory-mcp required).

## Prerequisites

1. `codebase-memory-rlm-rs` MCP server enabled (`codebase-memory-rlm` binary)
2. Project indexed via `index_repository`

Pass `project` on every graph tool call (defaults to repo folder name).

## RLM loop (graph-native)

### Phase 0 — Index

```
index_repository(repo_path=".", project="my-app")
index_status(project="my-app")
```

### Phase 1 — Filter

```
rlm_workflow(phase="filter")
search_graph(project, query="auth middleware", label="Function")
search_code(project, pattern="UserID", mode="files")
```

For logs/CSV (not in graph): `rlm_scan(path)` → `rlm_peek(session_id, query)`

### Phase 2 — Map (parallel)

One tool call per worker — never combine symbols:

```
rlm_read_symbol(project, qualified_name="api.routes.createUser")
trace_path(project, function_name="handleAuth", direction="both")
rlm_chunk(session_id, offset=0, limit=3)       # huge files only
```

Spawn 3–10 parallel sub-agents; each handles 1 symbol or 1 chunk.

### Phase 3 — Reduce

Merge worker JSON. Fill gaps with `trace_path` or `detect_changes`.

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
| Filter file paths | `search_code(mode="files")` |
| Read one symbol | `rlm_read_symbol` / `get_code_snippet` |
| Trace calls/impact | `trace_path` |
| Architecture | `get_architecture` |
| Git impact | `detect_changes` |
| Scan logs/CSV | `rlm_scan` |
| Peek in session | `rlm_peek` |
| Chunk huge file | `rlm_chunk` |
| Workflow help | `rlm_workflow` |

## Rules

1. Never load 10+ files into root context
2. `rlm_read_symbol` = one qualified_name per call
3. Prefer graph tools over `rg` when project is indexed
4. Use `rlm_scan`/`rlm_chunk` only for non-code or unindexed blobs
5. Always reduce to structured JSON before final answer