use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub qualified_name: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
    pub line_start: i64,
    pub line_end: i64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSummary {
    pub project: String,
    pub repo_path: String,
    pub files_indexed: usize,
    pub symbols_indexed: usize,
    pub edges_indexed: usize,
    pub indexed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub qualified_name: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
    pub line_start: i64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMatch {
    pub file_path: String,
    pub line: i64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceHop {
    pub qualified_name: String,
    pub name: String,
    pub file_path: String,
    pub direction: String,
    pub depth: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmChunk {
    pub source: String,
    pub chunk_id: usize,
    pub total_chunks: usize,
    pub content: String,
}