use std::fs;
use tempfile::tempdir;

use codebase_memory_rlm_rs::index::Indexer;
use codebase_memory_rlm_rs::store::Store;

#[test]
fn index_rust_repo() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        "pub fn hello() {}\npub struct Foo;\n",
    )
    .unwrap();

    let project = format!("test-{}", uuid::Uuid::new_v4().simple());
    let store = Store::open(&project).unwrap();
    let count = Indexer::index_repo(&store, dir.path()).unwrap();
    assert!(count >= 1);

    let hits = store.search_graph(Some("hello"), None, None, 10).unwrap();
    assert!(!hits.is_empty());
}