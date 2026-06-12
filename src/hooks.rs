//! Non-blocking agent hooks aligned with upstream CBM (SessionStart + PreToolUse).
//!
//! Cardinal rule: hooks NEVER block tool calls. Every error path exits 0 with no stdout.

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::paths::default_project_name;
use crate::store::{project_exists, SearchGraphFilter, Store};

const STDIN_CAP: usize = 256 * 1024;
const MIN_TOKEN: usize = 4;
const MAX_TOKEN: usize = 96;
const RESULT_LIMIT: usize = 5;
const MAX_WALKUP: usize = 8;
const DEADLINE_MS: u64 = 300;

pub const SESSION_REMINDER: &str = "\
CRITICAL - Code Discovery Protocol (CBRLM / codebase-rlm-memory-mcp):
1. ALWAYS use cbrlm MCP tools FIRST for code exploration:
   - search_graph / rlm_filter to find functions, classes, routes
   - trace_path for call chains and data flow
   - rlm_read_symbol / get_code_snippet for exact symbol source
   - rlm_scan / rlm_peek / rlm_chunk for logs and huge non-code files
   - rlm_workflow for map-reduce phases
2. Project names use cbrlm+ prefix (e.g. cbrlm+D-my-project); shares ~/.cache/codebase-memory-mcp with upstream CBM.
3. Use Grep/Glob/Read freely for text, configs, and non-code files; always Read a file before editing it.
4. If the project is not indexed yet, run index_repository FIRST.";

/// Codex/Gemini one-liner (no single quotes, no embedded newlines).
pub const CODEX_SESSION_REMINDER_CMD: &str = "\
echo \"Code discovery: prefer codebase-rlm-memory-mcp (search_graph, trace_path, rlm_filter, rlm_read_symbol) over grep/file-read; projects use cbrlm+ prefix; run index_repository first if not indexed.\"";

pub const CODEX_HOOK_BEGIN: &str = "# >>> codebase-rlm-memory-mcp SessionStart >>>";
pub const CODEX_HOOK_END: &str = "# <<< codebase-rlm-memory-mcp SessionStart <<<";

pub const HOOK_GATE_SCRIPT: &str = "cbrlm-code-discovery-gate";
pub const HOOK_SESSION_SCRIPT: &str = "cbrlm-session-reminder";
pub const HOOK_MATCHER: &str = "Grep|Glob";
pub const HOOK_TIMEOUT_SEC: i64 = 5;

/// Print SessionStart reminder with auto-detected project architecture to stdout.
/// Detects the current working directory, resolves the project name,
/// and if indexed, outputs architecture summary + top files + symbol distribution.
pub fn hook_session_start() -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => {
            print!("{SESSION_REMINDER}");
            return 0;
        }
    };

    let project = crate::paths::default_project_name(&cwd);

    if !project_exists(&project) {
        // Project not indexed — output reminder + suggestion
        print!(
            "{}\n\n\
             ⚠ Project \"{}\" is not indexed yet.\n\
             Run `index_repository` via MCP to build the knowledge graph:\n\
               index_repository(repo_path=\"{}\")\n\
             This will enable graph-powered code discovery.",
            SESSION_REMINDER,
            project,
            cwd.to_string_lossy().replace('\\', "/")
        );
        return 0;
    }

    let store = match Store::open(&project) {
        Ok(s) => s,
        Err(_) => {
            print!("{SESSION_REMINDER}");
            return 0;
        }
    };

    let summary = match store.summary() {
        Ok(s) => s,
        Err(_) => {
            print!("{SESSION_REMINDER}");
            return 0;
        }
    };

    let labels = store.list_symbol_labels().unwrap_or_default();
    let top_files = store.top_packages(8).unwrap_or_default();

    let labels_str: String = labels
        .iter()
        .map(|(l, c)| format!("  {l}: {c}"))
        .collect::<Vec<_>>()
        .join("\n");

    let files_str: String = top_files
        .iter()
        .map(|(f, c)| format!("  {f}  ({c} symbols)"))
        .collect::<Vec<_>>()
        .join("\n");

    let output = format!(
        "<cbrlm_architecture project=\"{project}\">
📊 Knowledge Graph Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Repository: {repo}
  Files indexed: {files}
  Symbols indexed: {symbols}
  Relationships: {edges}
  Indexed at: {indexed_at}

📂 Top files by symbol count:
{files_str}

🏷️  Symbol distribution:
{labels_str}

💡 Usage:
  • search_graph(project=\"{project}\", query=...)  - Find symbols
  • trace_path(project=\"{project}\", ...)          - Trace call chains  
  • rlm_read_symbol(project=\"{project}\", qn=...)  - Read symbol source
  • rlm_filter(project=\"{project}\", ...)          - Filter by label/name
</cbrlm_architecture>

{SESSION_REMINDER}",
        project = summary.project,
        repo = summary.repo_path,
        files = summary.files_indexed,
        symbols = summary.symbols_indexed,
        edges = summary.edges_indexed,
        indexed_at = summary.indexed_at,
        files_str = files_str,
        labels_str = labels_str,
    );

    print!("{output}");
    0
}

/// PreToolUse augmenter: stdin hook JSON → optional additionalContext on stdout.
pub fn hook_augment() -> i32 {
    let deadline = Instant::now() + Duration::from_millis(DEADLINE_MS);

    let input = match read_stdin() {
        Some(s) => s,
        None => return 0,
    };

    let root: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return 0,
    };

    let tool = root.get("tool_name").and_then(|v| v.as_str());
    if tool != Some("Grep") && tool != Some("Glob") {
        return 0;
    }

    let pattern = root
        .get("tool_input")
        .and_then(|v| v.get("pattern"))
        .and_then(|v| v.as_str());

    let token = match pattern.and_then(extract_token) {
        Some(t) => t,
        None => return 0,
    };

    let cwd = root
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok());

    let start = match cwd {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => return 0,
    };

    if let Some(ctx) = resolve_and_query(&start, &token, deadline) {
        emit_augment(&ctx);
    }
    0
}

fn read_stdin() -> Option<String> {
    let mut buf = String::new();
    let mut handle = io::stdin().lock();
    let mut chunk = [0u8; 4096];
    let mut total = 0usize;
    loop {
        if total >= STDIN_CAP {
            break;
        }
        let n = handle.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        let take = n.min(STDIN_CAP - total);
        buf.push_str(&String::from_utf8_lossy(&chunk[..take]));
        total += take;
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Extract the longest identifier-like token from a grep/glob pattern.
pub fn extract_token(pattern: &str) -> Option<String> {
    let mut best_start = 0usize;
    let mut best_len = 0usize;
    let bytes = pattern.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    i += 1;
                } else {
                    break;
                }
            }
            let len = i - start;
            if len > best_len {
                best_len = len;
                best_start = start;
            }
        } else {
            i += 1;
        }
    }
    if best_len < MIN_TOKEN {
        return None;
    }
    let len = best_len.min(MAX_TOKEN);
    Some(pattern[best_start..best_start + len].to_string())
}

enum QueryOutcome {
    Hits(String),
    NoHits,
    NotIndexed,
    Error,
}

fn resolve_and_query(start: &Path, token: &str, deadline: Instant) -> Option<String> {
    let mut dir = start.to_path_buf();
    for _ in 0..MAX_WALKUP {
        if Instant::now() >= deadline {
            return None;
        }
        match query_project(&dir, token) {
            QueryOutcome::Hits(ctx) => return Some(ctx),
            QueryOutcome::NoHits => return None,
            QueryOutcome::NotIndexed | QueryOutcome::Error => {
                if !dir.pop() {
                    break;
                }
            }
        }
    }
    None
}

fn query_project(dir: &Path, token: &str) -> QueryOutcome {
    let project = default_project_name(dir);
    if !project_exists(&project) {
        return QueryOutcome::NotIndexed;
    }
    let store = match Store::open(&project) {
        Ok(s) => s,
        Err(_) => return QueryOutcome::Error,
    };
    let pattern = format!("%{token}%");
    let hits = match store.search_graph(SearchGraphFilter {
        name_pattern: Some(&pattern),
        limit: RESULT_LIMIT,
        ..SearchGraphFilter::default()
    }) {
        Ok(page) => page.results,
        Err(_) => return QueryOutcome::Error,
    };
    if hits.is_empty() {
        return QueryOutcome::NoHits;
    }
    QueryOutcome::Hits(format_context(&hits, token))
}

fn format_context(hits: &[crate::models::SearchHit], token: &str) -> String {
    let mut text = format!(
        "[cbrlm] {} graph symbol(s) match \"{}\" \
         (structured context; your search results below are unaffected):",
        hits.len(),
        token
    );
    for hit in hits {
        let disp = if !hit.qualified_name.is_empty() {
            &hit.qualified_name
        } else {
            &hit.name
        };
        let label = if hit.label.is_empty() {
            String::new()
        } else {
            format!("  {}", hit.label)
        };
        text.push_str(&format!("\n- {disp}  {}{label}", hit.file_path));
        if text.len() > 3900 {
            break;
        }
    }
    text
}

fn emit_augment(text: &str) {
    let out = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": text
        }
    });
    if let Ok(s) = serde_json::to_string(&out) {
        print!("{s}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_token_skips_short_patterns() {
        assert_eq!(extract_token("ab"), None);
        assert_eq!(extract_token("*.ts"), None);
    }

    #[test]
    fn extract_token_picks_longest_identifier() {
        assert_eq!(
            extract_token("foo.*handleAuth_bar"),
            Some("handleAuth_bar".into())
        );
        assert_eq!(extract_token("UserService"), Some("UserService".into()));
    }

    #[test]
    fn extract_token_respects_max_len() {
        let long = "a".repeat(120);
        let got = extract_token(&long).unwrap();
        assert_eq!(got.len(), MAX_TOKEN);
    }
}
