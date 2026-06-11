use codebase_rlm_memory_mcp::hooks::extract_token;
use codebase_rlm_memory_mcp::paths::{default_project_name, project_db_path};
use codebase_rlm_memory_mcp::store::{project_exists, Store};
use tempfile::TempDir;

#[test]
fn hook_augment_query_finds_indexed_symbols() {
    let tmp = TempDir::new().unwrap();
    let project = default_project_name(tmp.path());
    let store = Store::open(&project).unwrap();
    store
        .insert_symbol(&codebase_rlm_memory_mcp::models::Symbol {
            qualified_name: "app.auth.handleAuth".into(),
            name: "handleAuth".into(),
            label: "Function".into(),
            file_path: "src/auth.rs".into(),
            line_start: 1,
            line_end: 10,
            signature: "fn handleAuth()".into(),
        })
        .unwrap();
    assert!(project_exists(&project));

    let pattern = format!("%{}%", "handleAuth");
    let hits = store
        .search_graph(None, Some(&pattern), None, 5)
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, "handleAuth");
    assert!(project_db_path(&project).exists());
}

#[test]
fn extract_token_integration_cases() {
    assert_eq!(extract_token("grep handleAuth"), Some("handleAuth".into()));
    assert_eq!(extract_token("**/*.tsx"), None);
}