# codebase-rlm-memory-mcp (CBRLM)

> **Full name:** codebase-rlm-memory-mcp  
> **Abbrev:** CBRLM (`cbrlm`)  
> **MCP server:** `codebase-rlm-memory-mcp`  
> **Binary:** `cbrlm`  
> **Repo:** https://github.com/stevenke1981/codebase-rlm-memory-mcp  
> **Pattern:** [Recursive Language Model](https://arxiv.org/pdf/2512.24601) + graph-native code intelligence  
> **Inspired by:** [DeusData/codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp)

Rust 單一 MCP 伺服器：內建程式碼知識圖譜（CBM 相容工具）+ RLM map-reduce 編排。無需另外安裝上游 C 二進位。

---

## For humans / 給人類

### What this is / 這是什麼

| 元件 | 角色 |
|------|------|
| **Graph index** | SQLite 儲存 symbols、CALLS 邊、FTS5 全文搜尋 |
| **CBM-style tools** | `index_repository`、`search_graph`、`trace_path` 等（與上游工具名相容） |
| **RLM tools** | `rlm_workflow`、`rlm_filter`、`rlm_scan`、`rlm_chunk` 等（Map-Reduce 編排） |

### v0.1 scope / 目前範圍

| Feature | Status |
|---------|--------|
| Regex symbol extraction | ✅ |
| FTS5 code search | ✅ |
| Heuristic CALLS graph | ✅ |
| Git change detection | ✅ |
| In-memory RLM sessions | ✅ |
| tree-sitter / 159 languages | ❌ roadmap |
| Embeddings / semantic search | ❌ roadmap |
| Full upstream CBM parity | ❌ roadmap |

### 與上游 CBM 共存（專案名稱不同）

共用快取目錄，但 **專案名稱加前綴**，避免覆寫上游索引：

| 引擎 | 專案名稱範例 | DB 檔案 |
|------|-------------|---------|
| 上游 CBM (C) | `D-animejs-skills` | `D-animejs-skills.db` |
| **CBRLM** (Rust) | `cbrlm+D-animejs-skills` | `cbrlm+D-animejs-skills.db` |

快取路徑：`~/.cache/codebase-memory-mcp/`（`CBM_CACHE_DIR` 可覆寫）

### Prerequisites / 前置需求

- [Rust](https://rustup.rs/) 1.75+
- `git` on PATH（`detect_changes` 用）

### Install / 安裝

**Windows（推薦）：**

```powershell
git clone https://github.com/stevenke1981/codebase-rlm-memory-mcp.git
cd codebase-rlm-memory-mcp
.\install.ps1
```

`install.ps1` 會：
1. `cargo build --release` → `cbrlm.exe`
2. 安裝到 `~/.config/opencode-cbrlm/bin/cbrlm.exe`
3. 寫入 OpenCode / Codex MCP 設定
4. 安裝 `rlm` skill

**手動編譯：**

```bash
cargo build --release
# Windows: target/release/cbrlm.exe
# Linux/macOS: target/release/cbrlm
```

**安裝到 PATH（可選）：**

```bash
cargo install --path .
# → ~/.cargo/bin/cbrlm
```

### MCP configuration / MCP 設定

MCP server 名稱：**`codebase-rlm-memory-mcp`**（與上游 `codebase-memory-mcp` 分開，可同時啟用）

**OpenCode** (`~/.config/opencode/opencode.json`):

```json
{
  "mcp": {
    "codebase-rlm-memory-mcp": {
      "type": "local",
      "command": [
        "pwsh", "-NoProfile", "-Command",
        "& \"$env:USERPROFILE\\.config\\opencode-cbrlm\\bin\\cbrlm.exe\""
      ],
      "enabled": true,
      "timeout": 120000,
      "environment": { "CBRLM_PROJECT_PREFIX": "cbrlm+" }
    }
  }
}
```

**Codex** (`~/.codex/config.toml`):

```toml
[mcp_servers.codebase-rlm-memory-mcp]
type = "stdio"
command = "C:/Users/YOU/.config/opencode-cbrlm/bin/cbrlm.exe"

[mcp_servers.codebase-rlm-memory-mcp.env]
CBRLM_PROJECT_PREFIX = "cbrlm+"
```

**Claude Code** (`~/.claude/settings.json` or project `.mcp.json`):

```json
{
  "mcpServers": {
    "codebase-rlm-memory-mcp": {
      "command": "C:\\Users\\YOU\\.config\\opencode-cbrlm\\bin\\cbrlm.exe",
      "env": { "CBRLM_PROJECT_PREFIX": "cbrlm+" }
    }
  }
}
```

### Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `CBM_CACHE_DIR` | `~/.cache/codebase-memory-mcp` | 圖譜 DB 儲存目錄（與上游共用） |
| `CBRLM_PROJECT_PREFIX` | `cbrlm+` | CBRLM 專案名稱前綴 |

### MCP tools

#### Graph / CBM-compatible

| Tool | Description |
|------|-------------|
| `index_repository` | 索引 repo 到知識圖譜 |
| `index_status` | 檢查索引狀態 |
| `list_projects` | 列出 CBRLM 專案（`cbrlm+` 前綴） |
| `delete_project` | 刪除專案索引 |
| `search_graph` | 搜尋 symbols（query / pattern / label） |
| `search_code` | FTS 程式碼搜尋（`compact` / `files`） |
| `get_code_snippet` | 讀取單一 symbol 原始碼 |
| `trace_path` | 追蹤呼叫路徑 |
| `get_architecture` | 架構總覽 |
| `detect_changes` | Git 變更檔案 |
| `query_graph` | 唯讀 SELECT 查詢 |
| `get_graph_schema` | 圖譜 schema |
| `manage_adr` | Stub (v0.1) |
| `ingest_traces` | Stub (v0.1) |

#### RLM

| Tool | Description |
|------|-------------|
| `rlm_workflow` | 階段指引（overview / filter / map / reduce） |
| `rlm_filter` | 圖譜篩選（`search_graph` 別名） |
| `rlm_read_symbol` | Map 單元：一次讀一個 symbol |
| `rlm_scan` | 掃描目錄到記憶體 session |
| `rlm_peek` | 在 session 內搜尋片段 |
| `rlm_chunk` | 分頁 chunk（平行 Map 用） |
| `rlm_session_list` | 列出活躍 session |
| `rlm_session_delete` | 釋放 session 記憶體 |

### Quick start / 快速開始

1. 在 agent 設定啟用 MCP（見上方）
2. 重啟 agent
3. 對 agent 說：「Index this project」
4. 大型分析走 RLM：`rlm_workflow` → `rlm_filter` → 平行 `rlm_read_symbol` → 合併

**範例流程：**

```
1. index_repository(repo_path=".")
   → project: cbrlm+D-your-repo

2. rlm_workflow(phase="filter")
   search_graph(project="cbrlm+...", query="authentication")

3. 平行 Map：
   rlm_read_symbol(project, qualified_name="auth::login")
   trace_path(project, function_name="login")

4. Reduce：
   detect_changes(project)
   get_architecture(project)
```

### Compare with related repos

| Repo | Language | Needs upstream CBM? |
|------|----------|---------------------|
| [DeusData/codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp) | C | — |
| [codebase-memory-rlm-mcp](https://github.com/stevenke1981/codebase-memory-rlm-mcp) | Python | Yes |
| **codebase-rlm-memory-mcp** (this) | Rust | **No** |

---

## For AI agents / LLMs

```yaml
name: rlm
mcp_server: codebase-rlm-memory-mcp
abbrev: cbrlm
binary: cbrlm
requires_mcp: [codebase-rlm-memory-mcp]
entrypoint: SKILL.md
standalone: true
paper: https://arxiv.org/pdf/2512.24601
```

### Activation

**Load when:** large repo analysis, cross-file search, security audit, RLM explicit request, codebase graph discovery.

**Skip when:** 1–3 file localized edit.

### Tool reading order

1. `index_repository` if not indexed
2. `rlm_workflow(phase="overview")`
3. `index_status` → `rlm_filter` / `search_graph`
4. Parallel `rlm_read_symbol` / `trace_path` (Map)
5. Merge → `detect_changes` if needed (Reduce)
6. `rlm_scan` + `rlm_chunk` only for non-graph files (logs, CSV)

### Project naming

- Auto-derived: `cbrlm+<upstream_key>` (e.g. `cbrlm+D-animejs-skills`)
- Passing upstream name `D-animejs-skills` auto-resolves to `cbrlm+D-animejs-skills`
- Legacy `rs+` names auto-migrate to `cbrlm+`
- Cache dir shared with upstream: `~/.cache/codebase-memory-mcp/`

### Hard rules

- One `rlm_read_symbol` per symbol per worker
- `rlm_chunk` with `limit` ≤ 5 per call
- Never substitute bulk file reads for `rlm_filter`
- Prefer graph tools over `rg` when project is indexed
- Context is external — never load 10+ files into root context

### RLM loop

```
Filter  → search_graph / rlm_filter / search_code(files)
Map     → parallel rlm_read_symbol (1 symbol each) or rlm_chunk
Reduce  → merge JSON, trace_path for gaps, detect_changes for impact
```

Non-code blobs: `rlm_scan(path)` → `rlm_peek` or `rlm_chunk`

---

## Development

```bash
cargo test
cargo build --release
cargo run --release   # stdio MCP server
```

## License

MIT