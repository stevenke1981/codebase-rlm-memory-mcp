use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::RlmChunk;

const CHUNK_SIZE: usize = 5000;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SESSION_BYTES: usize = 32 * 1024 * 1024;

pub struct RlmSession {
    pub id: String,
    pub root: PathBuf,
    pub files: HashMap<String, String>,
    pub chunk_size: usize,
    skipped_files: usize,
    skipped_bytes: u64,
    chunk_cache: Option<Vec<RlmChunk>>,
}

impl RlmSession {
    pub fn scan(root: &Path) -> Result<Self> {
        let mut files = HashMap::new();
        let mut total_bytes = 0usize;
        let mut skipped_files = 0usize;
        let mut skipped_bytes = 0u64;
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if should_skip(path) {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if size > MAX_FILE_BYTES
                || total_bytes.saturating_add(size as usize) > MAX_SESSION_BYTES
            {
                skipped_files += 1;
                skipped_bytes = skipped_bytes.saturating_add(size);
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(path) {
                total_bytes = total_bytes.saturating_add(content.len());
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
            skipped_files,
            skipped_bytes,
            chunk_cache: None,
        })
    }

    pub fn summary(&self) -> serde_json::Value {
        let total_chars: usize = self.files.values().map(|c| c.chars().count()).sum();
        serde_json::json!({
            "session_id": self.id,
            "root": self.root,
            "files_loaded": self.files.len(),
            "total_chars": total_chars,
            "skipped_files": self.skipped_files,
            "skipped_bytes": self.skipped_bytes,
        })
    }

    pub fn peek(&self, query: &str, limit: usize) -> Vec<String> {
        let mut out = Vec::new();
        for (path, content) in &self.files {
            let mut start = 0;
            while let Some(idx) = content[start..].find(query) {
                let abs = start + idx;
                let s = floor_char_boundary(content, abs.saturating_sub(200));
                let e = ceil_char_boundary(content, (abs + query.len() + 200).min(content.len()));
                out.push(format!("[{path}]: ...{}...", &content[s..e]));
                start = ceil_char_boundary(content, abs + 1);
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
            let bounds = chunk_bounds(content, self.chunk_size);
            let total = bounds.len().max(1);
            for (i, (start, end)) in bounds.into_iter().enumerate() {
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

impl Default for RlmSessionStore {
    fn default() -> Self {
        Self::new()
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

fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    idx = idx.min(s.len());
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

fn chunk_bounds(content: &str, chunk_size: usize) -> Vec<(usize, usize)> {
    if content.is_empty() {
        return vec![(0, 0)];
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < content.len() {
        let mut end = (start + chunk_size).min(content.len());
        end = floor_char_boundary(content, end);
        if end <= start {
            end = ceil_char_boundary(content, start + 1);
        }
        out.push((start, end));
        start = end;
    }
    out
}
