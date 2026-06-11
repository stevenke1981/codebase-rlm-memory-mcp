use std::path::{Path, PathBuf};

pub fn cache_root() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("codebase-memory-rlm-rs")
}

pub fn project_db_path(project: &str) -> PathBuf {
    let safe: String = project
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    cache_root().join(format!("{safe}.db"))
}

pub fn project_key(repo_path: &Path) -> String {
    repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf())
        .to_string_lossy()
        .replace(':', "-")
}