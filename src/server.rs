use std::path::Path;
use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

use crate::git_util::detect_changed_files;
use crate::index::{default_project_name, Indexer};
use crate::paths::{resolve_project_name, upstream_alias, upstream_project_key};
use crate::rlm::RlmSessionStore;
use crate::store::{delete_project, list_projects, project_exists, Store};

#[derive(Clone)]
pub struct CbrlmServer {
    pub rlm: Arc<RlmSessionStore>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl CbrlmServer {
    pub fn new() -> Self {
        Self {
            rlm: Arc::new(RlmSessionStore::new()),
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for CbrlmServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IndexRepositoryArgs {
    pub repo_path: String,
    pub project: Option<String>,
    pub mode: Option<String>,
    pub target_projects: Option<Vec<String>>,
    pub persistence: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProjectArgs {
    pub project: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchGraphArgs {
    pub project: String,
    pub query: Option<String>,
    pub name_pattern: Option<String>,
    pub label: Option<String>,
    pub qn_pattern: Option<String>,
    pub file_pattern: Option<String>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchCodeArgs {
    pub project: String,
    pub pattern: String,
    pub file_pattern: Option<String>,
    pub path_filter: Option<String>,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SnippetArgs {
    pub project: String,
    pub qualified_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TraceArgs {
    pub project: String,
    pub function_name: String,
    #[serde(default = "default_direction")]
    pub direction: String,
    #[serde(default = "default_depth")]
    pub depth: i64,
    pub mode: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QueryGraphArgs {
    pub project: String,
    pub query: String,
    pub max_rows: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ManageAdrArgs {
    pub project: String,
    pub mode: Option<String>,
    pub content: Option<String>,
    pub sections: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IngestTracesArgs {
    pub project: String,
    pub traces: Vec<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RlmScanArgs {
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RlmPeekArgs {
    pub session_id: String,
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RlmChunkArgs {
    pub session_id: String,
    pub file_pattern: Option<String>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_chunk_limit")]
    pub limit: usize,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RlmSessionIdArgs {
    pub session_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkflowArgs {
    #[serde(default = "default_phase")]
    pub phase: String,
}

fn default_limit() -> usize {
    20
}
fn default_chunk_limit() -> usize {
    5
}
fn default_depth() -> i64 {
    3
}
fn default_direction() -> String {
    "both".into()
}
fn default_mode() -> String {
    "compact".into()
}
fn default_phase() -> String {
    "overview".into()
}

fn json_ok(value: serde_json::Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into())
}

fn json_err(msg: impl std::fmt::Display) -> String {
    json_ok(serde_json::json!({ "error": msg.to_string() }))
}

fn resolve_and_open(project: &str) -> Result<(String, Store), String> {
    let resolved = resolve_project_name(project);
    let store = Store::open(&resolved).map_err(|e| e.to_string())?;
    Ok((resolved, store))
}

#[tool_router]
impl CbrlmServer {
    #[tool(description = "Index a repository into the knowledge graph.")]
    fn index_repository(&self, Parameters(args): Parameters<IndexRepositoryArgs>) -> String {
        if args.mode.as_deref() == Some("cross-repo-intelligence") {
            return json_ok(serde_json::json!({
                "warning": "cross-repo-intelligence not supported in v0.1",
                "hint": "Index each repo separately with mode=full|moderate|fast"
            }));
        }
        let repo = Path::new(&args.repo_path);
        let project = match args.project.as_deref() {
            Some(p) => resolve_project_name(p),
            None => default_project_name(repo),
        };
        let upstream = upstream_project_key(repo);
        let Ok(store) = Store::open(&project).map_err(|e| e.to_string()) else {
            return json_err("failed to open store");
        };
        let mode = args.mode.as_deref().unwrap_or("full");
        match Indexer::index_repo(&store, repo, mode) {
            Ok(count) => match store.summary() {
                Ok(summary) => json_ok(serde_json::json!({
                    "project": project,
                    "upstream_project": upstream,
                    "upstream_alias": upstream_alias(&project),
                    "mode": args.mode.unwrap_or_else(|| "full".into()),
                    "persistence": args.persistence.unwrap_or(false),
                    "symbols_indexed": count,
                    "summary": summary,
                    "engine": "codebase-rlm-memory-mcp",
                    "abbrev": "cbrlm",
                    "note": "CBRLM index uses cbrlm+ prefixed project names; upstream CBM uses upstream_project"
                })),
                Err(e) => json_err(e),
            },
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Check index status for a project.")]
    fn index_status(&self, Parameters(args): Parameters<ProjectArgs>) -> String {
        let project = resolve_project_name(&args.project);
        if !project_exists(&project) {
            return json_ok(serde_json::json!({
                "project": project,
                "requested": args.project,
                "indexed": false,
                "hint": "CBRLM projects use cbrlm+ prefix; pass upstream name or cbrlm+name"
            }));
        }
        match Store::open(&project) {
            Ok(store) => match store.summary() {
                Ok(summary) => json_ok(serde_json::json!({
                    "project": project,
                    "upstream_alias": upstream_alias(&project),
                    "indexed": true,
                    "summary": summary
                })),
                Err(e) => json_err(e),
            },
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "List indexed projects.")]
    fn list_projects(&self) -> String {
        match list_projects() {
            Ok(projects) => json_ok(serde_json::json!({ "projects": projects })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Delete a project index.")]
    fn delete_project(&self, Parameters(args): Parameters<ProjectArgs>) -> String {
        let project = resolve_project_name(&args.project);
        match delete_project(&project) {
            Ok(()) => json_ok(serde_json::json!({ "deleted": project })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Search the code knowledge graph.")]
    fn search_graph(&self, Parameters(args): Parameters<SearchGraphArgs>) -> String {
        let Ok((_project, store)) = resolve_and_open(&args.project) else {
            return json_err("failed to open store");
        };
        match store.search_graph(
            args.query.as_deref(),
            args.name_pattern.as_deref(),
            args.label.as_deref(),
            args.limit,
        ) {
            Ok(hits) => json_ok(serde_json::json!({ "results": hits })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Graph-augmented code search. Modes: compact, files.")]
    fn search_code(&self, Parameters(args): Parameters<SearchCodeArgs>) -> String {
        let Ok((_project, store)) = resolve_and_open(&args.project) else {
            return json_err("failed to open store");
        };
        if args.mode == "files" {
            match store.search_code_files(&args.pattern, args.limit) {
                Ok(files) => return json_ok(serde_json::json!({ "files": files })),
                Err(e) => return json_err(e),
            }
        }
        match store.search_code_matches(&args.pattern, args.limit) {
            Ok(matches) => {
                let rows: Vec<_> = matches
                    .into_iter()
                    .map(|(file, line, content)| {
                        serde_json::json!({ "file": file, "line": line, "content": content })
                    })
                    .collect();
                json_ok(serde_json::json!({ "matches": rows }))
            }
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Read source code for a symbol.")]
    fn get_code_snippet(&self, Parameters(args): Parameters<SnippetArgs>) -> String {
        let Ok((_project, store)) = resolve_and_open(&args.project) else {
            return json_err("failed to open store");
        };
        match store.get_snippet(&args.qualified_name) {
            Ok(snippet) => json_ok(serde_json::json!({
                "qualified_name": args.qualified_name,
                "snippet": snippet,
            })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Trace call paths.")]
    fn trace_path(&self, Parameters(args): Parameters<TraceArgs>) -> String {
        let mode = args.mode.clone().unwrap_or_else(|| "calls".into());
        let warning = if mode != "calls" {
            Some(format!("mode '{mode}' not supported in v0.1; fell back to calls"))
        } else {
            None
        };
        let Ok((_project, store)) = resolve_and_open(&args.project) else {
            return json_err("failed to open store");
        };
        match store.trace(&args.function_name, args.depth, &args.direction) {
            Ok(hops) => {
                let mut body = serde_json::json!({ "mode": "calls", "hops": hops });
                if let Some(w) = warning {
                    body["warning"] = serde_json::Value::String(w);
                    body["requested_mode"] = serde_json::Value::String(mode);
                }
                json_ok(body)
            }
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Execute a read-only graph query (SELECT on symbols/edges/files).")]
    fn query_graph(&self, Parameters(args): Parameters<QueryGraphArgs>) -> String {
        let Ok((_project, store)) = resolve_and_open(&args.project) else {
            return json_err("failed to open store");
        };
        let max_rows = args.max_rows.unwrap_or(100).min(1000);
        match store.query_graph_sql(&args.query, max_rows) {
            Ok(rows) => json_ok(serde_json::json!({ "rows": rows, "count": rows.len() })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Get the schema of the knowledge graph.")]
    fn get_graph_schema(&self, Parameters(args): Parameters<ProjectArgs>) -> String {
        let Ok((_project, store)) = resolve_and_open(&args.project) else {
            return json_err("failed to open store");
        };
        json_ok(store.graph_schema())
    }

    #[tool(description = "Create or update Architecture Decision Records.")]
    fn manage_adr(&self, Parameters(args): Parameters<ManageAdrArgs>) -> String {
        json_ok(serde_json::json!({
            "supported": false,
            "project": args.project,
            "message": "manage_adr not implemented in codebase-rlm-memory-mcp (cbrlm) v0.1",
            "workaround": "Store ADRs in repo .codebase-memory/adr.md manually"
        }))
    }

    #[tool(description = "Ingest runtime traces to enhance the knowledge graph.")]
    fn ingest_traces(&self, Parameters(args): Parameters<IngestTracesArgs>) -> String {
        json_ok(serde_json::json!({
            "supported": false,
            "project": args.project,
            "traces_received": args.traces.len(),
            "message": "ingest_traces not implemented in codebase-rlm-memory-mcp (cbrlm) v0.1"
        }))
    }

    #[tool(description = "Architecture overview.")]
    fn get_architecture(&self, Parameters(args): Parameters<ProjectArgs>) -> String {
        let Ok((_project, store)) = resolve_and_open(&args.project) else {
            return json_err("failed to open store");
        };
        match (store.list_symbol_labels(), store.top_packages(10), store.summary()) {
            (Ok(labels), Ok(top), Ok(summary)) => json_ok(serde_json::json!({
                "summary": summary,
                "labels": labels,
                "top_files": top,
            })),
            _ => json_err("architecture query failed"),
        }
    }

    #[tool(description = "Detect git-changed files.")]
    fn detect_changes(&self, Parameters(args): Parameters<ProjectArgs>) -> String {
        let Ok((_project, store)) = resolve_and_open(&args.project) else {
            return json_err("failed to open store");
        };
        let Ok(Some(repo)) = store.get_meta("repo_path") else {
            return json_err("project not indexed");
        };
        match detect_changed_files(Path::new(&repo)) {
            Ok(files) => json_ok(serde_json::json!({ "changed_files": files })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "RLM workflow guidance.")]
    fn rlm_workflow(&self, Parameters(args): Parameters<WorkflowArgs>) -> String {
        let guide = match args.phase.as_str() {
            "filter" => "index_status → search_graph/search_code(files). No bulk reads.",
            "map" => "Parallel get_code_snippet per symbol or rlm_chunk per chunk.",
            "reduce" => "Merge JSON; trace_path for gaps; detect_changes for impact.",
            _ => "RLM: Filter → Parallel Map → Reduce. Context is external.",
        };
        json_ok(serde_json::json!({ "phase": args.phase, "guide": guide }))
    }

    #[tool(description = "RLM filter via graph search.")]
    fn rlm_filter(&self, Parameters(args): Parameters<SearchGraphArgs>) -> String {
        self.search_graph(Parameters(args))
    }

    #[tool(description = "RLM map unit — one symbol.")]
    fn rlm_read_symbol(&self, Parameters(args): Parameters<SnippetArgs>) -> String {
        self.get_code_snippet(Parameters(args))
    }

    #[tool(description = "Scan directory into RLM session.")]
    fn rlm_scan(&self, Parameters(args): Parameters<RlmScanArgs>) -> String {
        match self.rlm.create(Path::new(&args.path)) {
            Ok(summary) => json_ok(summary),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Peek in RLM session.")]
    fn rlm_peek(&self, Parameters(args): Parameters<RlmPeekArgs>) -> String {
        match self.rlm.with_session(&args.session_id, |s| s.peek(&args.query, args.limit)) {
            Ok(matches) => json_ok(serde_json::json!({
                "session_id": args.session_id,
                "matches": matches,
            })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Paginated RLM chunks.")]
    fn rlm_chunk(&self, Parameters(args): Parameters<RlmChunkArgs>) -> String {
        match self.rlm.with_session_mut(&args.session_id, |s| {
            s.chunks(args.file_pattern.as_deref(), args.offset, args.limit)
        }) {
            Ok((total, chunks)) => json_ok(serde_json::json!({
                "session_id": args.session_id,
                "total": total,
                "offset": args.offset,
                "limit": args.limit,
                "chunks": chunks,
            })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "List RLM sessions.")]
    fn rlm_session_list(&self) -> String {
        json_ok(serde_json::json!({ "sessions": self.rlm.list() }))
    }

    #[tool(description = "Delete RLM session.")]
    fn rlm_session_delete(&self, Parameters(args): Parameters<RlmSessionIdArgs>) -> String {
        let deleted = self.rlm.delete(&args.session_id);
        json_ok(serde_json::json!({ "session_id": args.session_id, "deleted": deleted }))
    }
}

#[tool_handler]
impl ServerHandler for CbrlmServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "CBRLM (codebase-rlm-memory-mcp): Rust code intelligence + RLM. Shares ~/.cache/codebase-memory-mcp with upstream CBM but uses cbrlm+ project names (e.g. cbrlm+D-animejs-skills). Index first, then search_graph / rlm_filter.",
            )
    }
}