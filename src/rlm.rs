use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::RlmChunk;

const CHUNK_SIZE: usize = 5000;

pub struct RlmSession {
    pub id: String,
    pub root: PathBuf,
    pub files: HashMap<String, String>,
    pub chunk_size: usize,
    chunk_cache: Option<Vec<RlmChunk>>,
}

impl RlmSession {
    pub fn scan(root: &Path) -> Result<Self> {
        let mut files = HashMap::new();
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if should_skip(path) {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(path) {
                let key = path
                    .strip_prefix(root)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(key, content);
            }
        }
        Ok(Self {
            id: Uuid::new_v4().simple().to_string()[..12].to_string(),
            root: root.to_path_buf(),
            files,
            chunk_size: CHUNK_SIZE,
            chunk_cache: None,
        })
    }

    pub fn summary(&self) -> serde_json::Value {
        let total_chars: usize = self.files.values().map(|c| c.len()).sum();
        serde_json::json!({
            "session_id": self.id,
            "root": self.root,
            "files_loaded": self.files.len(),
            "total_chars": total_chars,
        })
    }

    pub fn peek(&self, query: &str, limit: usize) -> Vec<String> {
        let mut out = Vec::new();
        for (path, content) in &self.files {
            let mut start = 0;
            while let Some(idx) = content[start..].find(query) {
                let abs = start + idx;
                let s = abs.saturating_sub(200);
                let e = (abs + query.len() + 200).min(content.len());
                out.push(format!("[{path}]: ...{}...", &content[s..e]));
                start = abs + 1;
                if out.len() >= limit {
                    return out;
                }
            }
        }
        out
    }

    pub fn chunks(
        &mut self,
        file_pattern: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> (usize, Vec<RlmChunk>) {
        if self.chunk_cache.is_none() {
            self.chunk_cache = Some(self.build_chunks(None));
        }
        let all = if let Some(pat) = file_pattern {
            self.build_chunks(Some(pat))
        } else {
            self.chunk_cache.clone().unwrap_or_default()
        };
        let total = all.len();
        let page: Vec<RlmChunk> = all.into_iter().skip(offset).take(limit).collect();
        (total, page)
    }

    fn build_chunks(&self, file_pattern: Option<&str>) -> Vec<RlmChunk> {
        let mut all = Vec::new();
        for (path, content) in &self.files {
            if let Some(pat) = file_pattern {
                if !path.contains(pat) && !glob_match_simple(pat, path) {
                    continue;
                }
            }
            let total = content.len().div_ceil(self.chunk_size).max(1);
            for i in 0..total {
                let start = i * self.chunk_size;
                let end = (start + self.chunk_size).min(content.len());
                all.push(RlmChunk {
                    source: path.clone(),
                    chunk_id: i,
                    total_chunks: total,
                    content: content[start..end].to_string(),
                });
            }
        }
        all
    }
}

pub struct RlmSessionStore {
    inner: RwLock<HashMap<String, RlmSession>>,
}

impl RlmSessionStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn create(&self, path: &Path) -> Result<serde_json::Value> {
        let session = RlmSession::scan(path)?;
        let summary = session.summary();
        let id = session.id.clone();
        self.inner
            .write()
            .map_err(|e| AppError::msg(e.to_string()))?
            .insert(id, session);
        Ok(summary)
    }

    pub fn with_session<T>(&self, id: &str, f: impl FnOnce(&RlmSession) -> T) -> Result<T> {
        let guard = self
            .inner
            .read()
            .map_err(|e| AppError::msg(e.to_string()))?;
        let session = guard
            .get(id)
            .ok_or_else(|| AppError::msg(format!("unknown session: {id}")))?;
        Ok(f(session))
    }

    pub fn with_session_mut<T>(&self, id: &str, f: impl FnOnce(&mut RlmSession) -> T) -> Result<T> {
        let mut guard = self
            .inner
            .write()
            .map_err(|e| AppError::msg(e.to_string()))?;
        let session = guard
            .get_mut(id)
            .ok_or_else(|| AppError::msg(format!("unknown session: {id}")))?;
        Ok(f(session))
    }

    pub fn list(&self) -> Vec<serde_json::Value> {
        self.inner
            .read()
            .ok()
            .map(|g| g.values().map(|s| s.summary()).collect())
            .unwrap_or_default()
    }

    pub fn delete(&self, id: &str) -> bool {
        self.inner
            .write()
            .ok()
            .map(|mut g| g.remove(id).is_some())
            .unwrap_or(false)
    }
}

fn should_skip(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c.as_os_str().to_string_lossy().as_ref(),
            ".git" | "node_modules" | "target" | "__pycache__" | ".venv" | "venv"
        )
    })
}

fn glob_match_simple(pattern: &str, path: &str) -> bool {
    if pattern.contains('*') {
        let suffix = pattern.trim_start_matches('*');
        path.ends_with(suffix) || path.contains(suffix.trim_start_matches('.'))
    } else {
        false
    }
}