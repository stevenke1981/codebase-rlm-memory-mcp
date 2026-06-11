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

Index cache: `~/.cache/codebase-memory-mcp/<project>.db`（可用 `CBM_CACHE_DIR` 覆寫，與上游相同）

### Prerequisites / 前置需求

- [Rust](https://rustup.rs/) 1.75+
- `git` on PATH (for `detect_changes`)

### Install / 安裝

**Windows（推薦，一鍵編譯 + MCP 設定）：**

```powershell
git clone https://github.com/stevenke1981/codebase-memory-rlm-rs.git
cd codebase-memory-rlm-rs
.\install.ps1
```

`install.ps1` 會：
- `cargo build --release` 編譯二進位
- 安裝為 `codebase-memory-mcp.exe`（與上游 **同名**，skills 免改）
- 備份上游二進位為 `*.upstream.bak`
- 更新 OpenCode `opencode.json` 與 Codex `config.toml`
- 安裝 `rlm` skill

若不想覆蓋上游 C 版：`.\install.ps1 -NoReplace`

**手動編譯：**

```bash
cargo build --release
# → target/release/codebase-memory-rlm.exe
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

使用與上游相同的 MCP 名稱 **`codebase-memory-mcp`**，現有 skills / AGENTS.md 規則無需修改。

**OpenCode** (`~/.config/opencode/opencode.json`) — `install.ps1` 會自動寫入：

```json
{
  "mcp": {
    "codebase-memory-mcp": {
      "type": "local",
      "command": [
        "pwsh", "-NoProfile", "-Command",
        "& \"$env:USERPROFILE\\.config\\opencode-codebase-memory-mcp\\bin\\codebase-memory-mcp.exe\""
      ],
      "enabled": true,
      "timeout": 120000
    }
  }
}
```

**Codex** (`~/.codex/config.toml`) — `install.ps1` 會自動寫入：

```toml
[mcp_servers.codebase-memory-mcp]
type = "stdio"
command = "C:/Users/YOU/AppData/Local/Programs/codebase-memory-mcp/codebase-memory-mcp.exe"
```

**Claude Code** — 同上，server 名稱用 `codebase-memory-mcp`。

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
| `query_graph` | Read-only SELECT on graph tables |
| `get_graph_schema` | Node labels and edge types |
| `manage_adr` | Stub (v0.1) |
| `ingest_traces` | Stub (v0.1) |

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