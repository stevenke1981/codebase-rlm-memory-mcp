use std::path::{Path, PathBuf};

/// Compatible with upstream `CBM_CACHE_DIR` (default: `~/.cache/codebase-memory-mcp`).
pub fn cache_root() -> PathBuf {
    if let Ok(dir) = std::env::var("CBM_CACHE_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("codebase-memory-mcp")
}

/// Prefix for Rust-owned projects. Must differ from upstream CBM names in the same cache dir.
pub fn project_prefix() -> String {
    std::env::var("CBM_RS_PROJECT_PREFIX").unwrap_or_else(|_| "rs+".into())
}

/// Mirror upstream `cbm_project_name_from_path` (without the Rust prefix).
pub fn upstream_project_key(repo_path: &Path) -> String {
    let canonical = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let s = canonical.to_string_lossy().replace('\\', "/");

    let mut normalized = String::with_capacity(s.len());
    let mut prev = '\0';
    for c in s.chars() {
        let safe =
            c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-';
        let ch = if safe { c } else { '-' };
        if (ch == '-' && prev == '-') || (ch == '.' && prev == '.') {
            continue;
        }
        normalized.push(ch);
        prev = ch;
    }

    let trimmed = normalized.trim_matches(|c| c == '-' || c == '.');
    if trimmed.is_empty() {
        "root".into()
    } else {
        trimmed.into()
    }
}

/// Default Rust project name: `rs+<upstream_key>` — same cache dir, distinct DB file.
pub fn default_project_name(repo_path: &Path) -> String {
    rs_project_name(&upstream_project_key(repo_path))
}

pub fn rs_project_name(upstream_key: &str) -> String {
    format!("{}{}", project_prefix(), upstream_key)
}

/// Ensure tool calls resolve to the Rust-namespaced project.
pub fn resolve_project_name(name: &str) -> String {
    let prefix = project_prefix();
    if name.starts_with(&prefix) {
        name.to_string()
    } else {
        format!("{prefix}{name}")
    }
}

pub fn is_rs_project(name: &str) -> bool {
    name.starts_with(&project_prefix())
}

pub fn upstream_alias(rs_name: &str) -> Option<String> {
    let prefix = project_prefix();
    rs_name
        .strip_prefix(&prefix)
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

pub fn project_db_path(project: &str) -> PathBuf {
    let safe: String = project
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '+' {
                c
            } else {
                '_'
            }
        })
        .collect();
    cache_root().join(format!("{safe}.db"))
}

/// Legacy helper — prefer `upstream_project_key` + `rs_project_name`.
pub fn project_key(repo_path: &Path) -> String {
    upstream_project_key(repo_path)
}