use std::fs;
use tempfile::tempdir;

use codebase_rlm_memory_mcp::index::Indexer;
use codebase_rlm_memory_mcp::models::Symbol;
use codebase_rlm_memory_mcp::rlm::RlmSession;
use codebase_rlm_memory_mcp::store::{SearchGraphFilter, Store};

#[test]
fn index_rust_repo() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.rs"), "pub fn hello() {}\npub struct Foo;\n").unwrap();

    let project = format!("test-{}", uuid::Uuid::new_v4().simple());
    let store = Store::open(&project).unwrap();
    let count = Indexer::index_repo(&store, dir.path(), "full").unwrap();
    assert!(count >= 1);

    let page = store
        .search_graph(SearchGraphFilter {
            query: Some("hello"),
            limit: 10,
            ..SearchGraphFilter::default()
        })
        .unwrap();
    assert!(!page.results.is_empty());
}

#[test]
fn search_graph_filters_and_paginates_like_cbm() {
    let project = format!("test-{}", uuid::Uuid::new_v4().simple());
    let store = Store::open(&project).unwrap();

    for name in ["handleAuth", "handleOrder", "renderHome"] {
        store
            .insert_symbol(&Symbol {
                qualified_name: format!("src/api.rs::{name}"),
                name: name.into(),
                label: "Function".into(),
                file_path: "src/api.rs".into(),
                line_start: 1,
                line_end: 3,
                signature: format!("fn {name}()"),
            })
            .unwrap();
    }
    store
        .insert_edge("src/api.rs::handleAuth", "src/api.rs::handleOrder", "CALLS")
        .unwrap();

    let page = store
        .search_graph(SearchGraphFilter {
            name_pattern: Some(".*handle.*"),
            file_pattern: Some("src/*.rs"),
            relationship: Some("CALLS"),
            min_degree: Some(1),
            limit: 1,
            ..SearchGraphFilter::default()
        })
        .unwrap();

    assert_eq!(page.total, 2);
    assert_eq!(page.results.len(), 1);
    assert!(page.has_more);
}

#[test]
fn query_graph_accepts_limit_and_cbm_table_aliases() {
    let project = format!("test-{}", uuid::Uuid::new_v4().simple());
    let store = Store::open(&project).unwrap();
    store
        .insert_symbol(&Symbol {
            qualified_name: "src/lib.rs::hello".into(),
            name: "hello".into(),
            label: "Function".into(),
            file_path: "src/lib.rs".into(),
            line_start: 1,
            line_end: 3,
            signature: "fn hello()".into(),
        })
        .unwrap();

    let rows = store
        .query_graph_sql("SELECT name FROM nodes ORDER BY name LIMIT 1", 10)
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "hello");
}

#[test]
fn rlm_chunks_unicode_safely() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("unicode.txt"), "你好世界".repeat(2000)).unwrap();

    let mut session = RlmSession::scan(dir.path()).unwrap();
    let (_total, chunks) = session.chunks(Some("unicode"), 0, 3);

    assert!(!chunks.is_empty());
    assert!(chunks
        .iter()
        .all(|chunk| std::str::from_utf8(chunk.content.as_bytes()).is_ok()));
}
