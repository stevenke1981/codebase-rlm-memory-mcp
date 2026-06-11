# codebase-rlm-memory-mcp (CBRLM)

> **Full name:** codebase-rlm-memory-mcp  
> **Abbrev:** CBRLM  
> **MCP server:** `codebase-rlm-memory-mcp`  
> **Binary:** `cbrlm`  
> **Pattern:** [Recursive Language Model](https://arxiv.org/pdf/2512.24601) + graph-native code intelligence  
> **Inspired by:** [DeusData/codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp)

Rust 單一 MCP 伺服器：內建程式碼知識圖譜 + RLM map-reduce 編排。

---

## For humans / 給人類

### 專案命名（與上游 CBM 共存）

| 引擎 | 專案名稱範例 | DB 檔案 |
|------|-------------|---------|
| 上游 CBM (C) | `D-animejs-skills` | `D-animejs-skills.db` |
| **CBRLM** (Rust) | `cbrlm+D-animejs-skills` | `cbrlm+D-animejs-skills.db` |

共用快取目錄：`~/.cache/codebase-memory-mcp/`（`CBM_CACHE_DIR` 可覆寫）  
CBRLM 前綴：`cbrlm+`（`CBRLM_PROJECT_PREFIX` 可覆寫）

### Install / 安裝

```powershell
git clone https://github.com/stevenke1981/codebase-rlm-memory-mcp.git
cd codebase-rlm-memory-mcp
.\install.ps1
```

`install.ps1` 會編譯 `cbrlm.exe`、寫入 MCP 設定、安裝 `rlm` skill。

手動編譯：

```bash
cargo build --release
# → target/release/cbrlm
```

### MCP configuration

**OpenCode** (`~/.config/opencode/opencode.json`):

```json
{
  "mcp": {
    "codebase-rlm-memory-mcp": {
      "type": "local",
      "command": ["pwsh", "-NoProfile", "-Command", "& \"$env:USERPROFILE\\.config\\opencode-cbrlm\\bin\\cbrlm.exe\""],
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

### MCP tools

#### Graph (CBM-compatible)

`index_repository` · `index_status` · `list_projects` · `delete_project` · `search_graph` · `search_code` · `get_code_snippet` · `trace_path` · `get_architecture` · `detect_changes` · `query_graph` · `get_graph_schema`

#### RLM

`rlm_workflow` · `rlm_filter` · `rlm_read_symbol` · `rlm_scan` · `rlm_peek` · `rlm_chunk` · `rlm_session_list` · `rlm_session_delete`

### Quick start

```
1. index_repository(repo_path=".")
2. rlm_workflow(phase="filter")
3. search_graph(project="cbrlm+...", query="auth")
4. Parallel rlm_read_symbol per symbol
5. trace_path + detect_changes (reduce)
```

---

## For AI agents / LLMs

```yaml
name: rlm
mcp_server: codebase-rlm-memory-mcp
abbrev: cbrlm
requires_mcp: [codebase-rlm-memory-mcp]
entrypoint: SKILL.md
```

- Project auto-name: `cbrlm+<upstream_key>`
- One `rlm_read_symbol` per symbol per worker
- `rlm_chunk` limit ≤ 5 per call

---

## Development

```bash
cargo test
cargo build --release
cargo run --release   # stdio MCP via cbrlm
```

## License

MIT