# codebase-memory-rlm-rs

> **Skill name:** `rlm`  
> **MCP server:** `codebase-memory-rlm-rs`  
> **Binary:** `codebase-memory-rlm`  
> **Pattern:** [Recursive Language Model](https://arxiv.org/pdf/2512.24601) + graph-native code intelligence  
> **Inspired by:** [DeusData/codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp) — standalone Rust reimplementation with built-in RLM

以 Rust 重寫的 **單一 MCP 伺服器**：內建程式碼知識圖譜索引 + RLM map-reduce 編排，無需另外安裝上游 C 二進位檔。

---

## For humans / 給人類

### What this is / 這是什麼

| Component | Role |
|-----------|------|
| **Graph index** | SQLite 儲存 symbols、CALLS 邊、FTS5 全文搜尋 |
| **CBM-style tools** | `index_repository`, `search_graph`, `trace_path`, … |
| **RLM tools** | `rlm_workflow`, `rlm_filter`, `rlm_scan`, `rlm_chunk`, … |

### v0.1 scope / 目前範圍

| Feature | Status |
|---------|--------|
| Regex symbol extraction (Rust, Python, JS/TS, Go, …) | ✅ |
| FTS5 code search | ✅ |
| Heuristic CALLS graph | ✅ |
| Git change detection | ✅ |
| In-memory RLM sessions | ✅ |
| tree-sitter / 159 languages | ❌ roadmap |
| Embeddings / semantic search | ❌ roadmap |
| Full upstream CBM parity | ❌ roadmap |

Index cache: `~/.cache/codebase-memory-rlm-rs/<project>.db`

### Prerequisites / 前置需求

- [Rust](https://rustup.rs/) 1.75+
- `git` on PATH (for `detect_changes`)

### Install / 安裝

**Clone & build:**

```bash
git clone https://github.com/stevenke1981/codebase-memory-rlm-rs.git
cd codebase-memory-rlm-rs
cargo build --release
```

**Windows (PowerShell):**

```powershell
git clone https://github.com/stevenke1981/codebase-memory-rlm-rs.git
cd codebase-memory-rlm-rs
cargo build --release
.\install.ps1
```

**Linux / macOS:**

```bash
git clone https://github.com/stevenke1981/codebase-memory-rlm-rs.git
cd codebase-memory-rlm-rs
cargo build --release
./install.sh
```

Install scripts copy `SKILL.md` to agent skill directories and print MCP config hints.

**Optional — install binary to PATH:**

```bash
cargo install --path .
# → ~/.cargo/bin/codebase-memory-rlm
```

### MCP configuration / MCP 設定

只需 **一個** MCP server（與 Python 版 `codebase-memory-rlm-mcp` 不同，後者需搭配上游 CBM）。

**OpenCode** (`~/.config/opencode/opencode.json`):

```json
{
  "mcp": {
    "codebase-memory-rlm-rs": {
      "type": "local",
      "command": ["codebase-memory-rlm"],
      "enabled": true,
      "timeout": 120000
    }
  }
}
```

若未 `cargo install`，使用完整路徑：

```json
"command": ["D:\\codebase-memory-rlm-rs\\target\\release\\codebase-memory-rlm.exe"]
```

**Claude Code** (`~/.claude/settings.json` or project `.mcp.json`):

```json
{
  "mcpServers": {
    "codebase-memory-rlm-rs": {
      "command": "codebase-memory-rlm"
    }
  }
}
```

**Codex** (`~/.codex/config.toml`):

```toml
[mcp_servers.codebase-memory-rlm-rs]
command = "codebase-memory-rlm"
```

### Quick start / 快速開始

1. 在 agent 設定中啟用 MCP（見上方）
2. 對 agent 說：「Index this project」→ 呼叫 `index_repository`
3. 大型分析時使用 RLM 流程：`rlm_workflow` → `rlm_filter` → 平行 `rlm_read_symbol` → 合併

### MCP tools

#### Graph / CBM-style

| Tool | Description |
|------|-------------|
| `index_repository` | Index repo into knowledge graph |
| `index_status` | Check index status |
| `list_projects` | List indexed projects |
| `delete_project` | Delete project index |
| `search_graph` | Search symbols by query / pattern / label |
| `search_code` | FTS code search (`compact` or `files` mode) |
| `get_code_snippet` | Read source for one symbol |
| `trace_path` | Trace call paths |
| `get_architecture` | Architecture overview |
| `detect_changes` | Git-changed files |

#### RLM

| Tool | Description |
|------|-------------|
| `rlm_workflow` | Phase guidance (overview/filter/map/reduce) |
| `rlm_filter` | Graph search filter (alias of `search_graph`) |
| `rlm_read_symbol` | One symbol snippet (Map unit) |
| `rlm_scan` | Scan directory into in-memory session |
| `rlm_peek` | Query snippets in session |
| `rlm_chunk` | Paginated chunks for parallel Map |
| `rlm_session_list` | List active sessions |
| `rlm_session_delete` | Free session memory |

### Example workflow

```
1. index_repository(repo_path=".", project="my-app")
2. rlm_workflow(phase="filter")
3. search_graph(project="my-app", query="authentication")
4. Parallel: rlm_read_symbol(project="my-app", qualified_name="...")
5. trace_path(project="my-app", function_name="handleAuth")
```

### Compare with other repos

| Repo | Language | Needs upstream CBM? |
|------|----------|---------------------|
| [DeusData/codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp) | C | — (upstream) |
| [codebase-memory-rlm-mcp](https://github.com/stevenke1981/codebase-memory-rlm-mcp) | Python | Yes |
| **codebase-memory-rlm-rs** (this) | Rust | **No** |

---

## For AI agents / LLMs

```yaml
name: rlm
mcp_server: codebase-memory-rlm-rs
requires_mcp: [codebase-memory-rlm-rs]
entrypoint: SKILL.md
standalone: true
```

### Activation

Load when: large repo analysis, cross-file search, security audit, RLM explicit request, codebase graph discovery.

Skip when: 1–3 file localized edit.

### Tool reading order

1. `index_repository` if not indexed
2. `rlm_workflow(phase="overview")`
3. `index_status` → `rlm_filter` / `search_graph`
4. Parallel `rlm_read_symbol` / `trace_path` (Map)
5. Merge → `detect_changes` if needed (Reduce)
6. `rlm_scan` + `rlm_chunk` only for non-graph files (logs, CSV)

### Hard rules

- One `rlm_read_symbol` per symbol per worker
- `rlm_chunk` with `limit` ≤ 5 per call
- Never substitute bulk file reads for `rlm_filter`
- Prefer graph tools over `rg` when project is indexed

---

## Development

```bash
cargo test
cargo build --release
cargo run --release   # stdio MCP server
```

## License

MIT