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