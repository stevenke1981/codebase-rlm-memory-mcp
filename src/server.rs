use std::path::Path;
use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};

use crate::git_util::detect_changed_files;
use crate::index::{default_project_name, Indexer};
use crate::rlm::RlmSessionStore;
use crate::store::{delete_project, list_projects, project_exists, Store};

#[derive(Clone)]
pub struct CbmRlmServer {
    pub rlm: Arc<RlmSessionStore>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl CbmRlmServer {
    pub fn new() -> Self {
        Self {
            rlm: Arc::new(RlmSessionStore::new()),
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for CbmRlmServer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct IndexRepositoryArgs {
    pub repo_path: String,
    pub project: Option<String>,
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
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchCodeArgs {
    pub project: String,
    pub pattern: String,
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

fn open_store(project: &str) -> Result<Store, String> {
    Store::open(project).map_err(|e| e.to_string())
}

#[tool_router]
impl CbmRlmServer {
    #[tool(description = "Index a repository into the knowledge graph.")]
    fn index_repository(&self, Parameters(args): Parameters<IndexRepositoryArgs>) -> String {
        let repo = Path::new(&args.repo_path);
        let project = args
            .project
            .unwrap_or_else(|| default_project_name(repo));
        let Ok(store) = open_store(&project) else {
            return json_err("failed to open store");
        };
        match Indexer::index_repo(&store, repo) {
            Ok(count) => match store.summary() {
                Ok(summary) => json_ok(serde_json::json!({
                    "project": project,
                    "symbols_indexed": count,
                    "summary": summary,
                })),
                Err(e) => json_err(e),
            },
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Check index status for a project.")]
    fn index_status(&self, Parameters(args): Parameters<ProjectArgs>) -> String {
        if !project_exists(&args.project) {
            return json_ok(serde_json::json!({ "project": args.project, "indexed": false }));
        }
        match open_store(&args.project).and_then(|s| s.summary().map_err(|e| e.to_string())) {
            Ok(summary) => json_ok(serde_json::json!({ "indexed": true, "summary": summary })),
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
        match delete_project(&args.project) {
            Ok(()) => json_ok(serde_json::json!({ "deleted": args.project })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Search the code knowledge graph.")]
    fn search_graph(&self, Parameters(args): Parameters<SearchGraphArgs>) -> String {
        let Ok(store) = open_store(&args.project) else {
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
        let Ok(store) = open_store(&args.project) else {
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
        let Ok(store) = open_store(&args.project) else {
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
        let Ok(store) = open_store(&args.project) else {
            return json_err("failed to open store");
        };
        match store.trace(&args.function_name, args.depth, &args.direction) {
            Ok(hops) => json_ok(serde_json::json!({ "hops": hops })),
            Err(e) => json_err(e),
        }
    }

    #[tool(description = "Architecture overview.")]
    fn get_architecture(&self, Parameters(args): Parameters<ProjectArgs>) -> String {
        let Ok(store) = open_store(&args.project) else {
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
        let Ok(store) = open_store(&args.project) else {
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
        match self.rlm.with_session(&args.session_id, |s| {
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
impl ServerHandler for CbmRlmServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "Rust code intelligence MCP with RLM. Index first, filter with search_graph, map with get_code_snippet, chunk huge files with rlm_chunk.",
            )
    }
}