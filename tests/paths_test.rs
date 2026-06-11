use std::path::Path;

use codebase_rlm_memory_mcp::paths::{
    cbrlm_project_name, default_project_name, resolve_project_name, upstream_project_key,
};

#[test]
fn cbrlm_project_name_differs_from_upstream() {
    let dir = Path::new("D:/animejs-skills");
    let upstream = upstream_project_key(dir);
    let cbrlm_name = default_project_name(dir);
    assert!(cbrlm_name.starts_with("cbrlm+"));
    assert_ne!(cbrlm_name, upstream);
    assert_eq!(cbrlm_name, cbrlm_project_name(&upstream));
}

#[test]
fn resolve_accepts_forms() {
    assert_eq!(resolve_project_name("cbrlm+my-app"), "cbrlm+my-app");
    assert_eq!(resolve_project_name("my-app"), "cbrlm+my-app");
    assert_eq!(resolve_project_name("rs+my-app"), "cbrlm+my-app");
}