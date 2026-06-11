use std::path::Path;

use codebase_memory_rlm_rs::paths::{
    default_project_name, resolve_project_name, rs_project_name, upstream_project_key,
};

#[test]
fn rust_project_name_differs_from_upstream() {
    let dir = Path::new("D:/animejs-skills");
    let upstream = upstream_project_key(dir);
    let rust_name = default_project_name(dir);
    assert!(rust_name.starts_with("rs+"));
    assert_ne!(rust_name, upstream);
    assert_eq!(rust_name, rs_project_name(&upstream));
}

#[test]
fn resolve_accepts_both_forms() {
    assert_eq!(resolve_project_name("rs+my-app"), "rs+my-app");
    assert_eq!(resolve_project_name("my-app"), "rs+my-app");
}