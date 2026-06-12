# Lessons Learned

---
## Lesson #1 — 2026-06-12
**Trigger:** 建立知識圖譜 (index_repository) 後，需要驗證索引狀態
**Rule:** 執行 `index_repository` 後，立即用 `index_status` 和 `get_architecture` 雙重驗證索引完整性
**Source:** 建立 codebase-rlm-memory-mcp 知識圖譜

---
## Lesson #2 — 2026-06-12
**Trigger:** 已索引專案的新 session 開始時
**Rule:** 系統會自動執行 `cbrlm hook-session-start` 並將知識圖譜架構摘要注入 system prompt（`<cbrlm_context>` 區塊）。Agent 不需手動 call `get_architecture`，可直接從該區塊取得專案結構。
**Source:** 自動載入知識圖譜到 system prompt

---
## Lesson #3 — 2026-06-12
**Trigger:** Session 開始時 system prompt 出現 `<cbrlm_context>` 區塊
**Rule:** 區塊內含 Knowledge Graph Summary（檔案數、符號數、關聯數、top files、symbol distribution）。使用 `search_graph` / `trace_path` / `rlm_read_symbol` 取得更詳細的圖譜資料，不需用 Grep/Read 重新探索 codebase。
**Source:** 自動載入知識圖譜到 system prompt
