use rusqlite::{params, Connection};

use crate::error::{AppError, Result};
use crate::models::{IndexSummary, SearchHit, Symbol, TraceHop};
use crate::paths::project_db_path;

pub struct Store {
    conn: Connection,
    pub project: String,
}

impl Store {
    pub fn open(project: &str) -> Result<Self> {
        let path = project_db_path(project);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let store = Self {
            conn,
            project: project.to_string(),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                language TEXT NOT NULL,
                line_count INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                qualified_name TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                label TEXT NOT NULL,
                file_path TEXT NOT NULL,
                line_start INTEGER NOT NULL,
                line_end INTEGER NOT NULL,
                signature TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS edges (
                src_qn TEXT NOT NULL,
                dst_qn TEXT NOT NULL,
                edge_type TEXT NOT NULL
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
                qualified_name, name, signature, file_path
            );
            "#,
        )?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM edges; DELETE FROM symbols; DELETE FROM files; DELETE FROM meta;",
        )?;
        self.conn.execute("DELETE FROM symbols_fts", [])?;
        Ok(())
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_file(&self, path: &str, content: &str, language: &str, line_count: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO files(path, content, language, line_count) VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET content=excluded.content, language=excluded.language, line_count=excluded.line_count",
            params![path, content, language, line_count],
        )?;
        Ok(())
    }

    pub fn insert_symbol(&self, sym: &Symbol) -> Result<()> {
        self.conn.execute(
            "INSERT INTO symbols(qualified_name, name, label, file_path, line_start, line_end, signature)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(qualified_name) DO UPDATE SET
               name=excluded.name, label=excluded.label, file_path=excluded.file_path,
               line_start=excluded.line_start, line_end=excluded.line_end, signature=excluded.signature",
            params![
                sym.qualified_name,
                sym.name,
                sym.label,
                sym.file_path,
                sym.line_start,
                sym.line_end,
                sym.signature
            ],
        )?;
        self.conn.execute(
            "INSERT INTO symbols_fts(qualified_name, name, signature, file_path)
             VALUES(?1, ?2, ?3, ?4)",
            params![sym.qualified_name, sym.name, sym.signature, sym.file_path],
        )?;
        Ok(())
    }

    pub fn insert_edge(&self, src: &str, dst: &str, edge_type: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO edges(src_qn, dst_qn, edge_type) VALUES(?1, ?2, ?3)",
            params![src, dst, edge_type],
        )?;
        Ok(())
    }

    pub fn search_graph(
        &self,
        query: Option<&str>,
        name_pattern: Option<&str>,
        label: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let mut hits = Vec::new();
        if let Some(q) = query {
            let sql = if label.is_some() {
                "SELECT s.qualified_name, s.name, s.label, s.file_path, s.line_start, bm25(symbols_fts) AS score
                 FROM symbols_fts f JOIN symbols s ON s.qualified_name = f.qualified_name
                 WHERE symbols_fts MATCH ?1 AND s.label = ?2
                 ORDER BY score LIMIT ?3"
            } else {
                "SELECT s.qualified_name, s.name, s.label, s.file_path, s.line_start, bm25(symbols_fts) AS score
                 FROM symbols_fts f JOIN symbols s ON s.qualified_name = f.qualified_name
                 WHERE symbols_fts MATCH ?1
                 ORDER BY score LIMIT ?2"
            };
            let mut stmt = self.conn.prepare(sql)?;
            if let Some(lbl) = label {
                let mut rows = stmt.query(params![q, lbl, limit as i64])?;
                while let Some(row) = rows.next()? {
                    hits.push(row_to_hit(row)?);
                }
            } else {
                let mut rows = stmt.query(params![q, limit as i64])?;
                while let Some(row) = rows.next()? {
                    hits.push(row_to_hit(row)?);
                }
            }
            return Ok(hits);
        }

        let like_pat = name_pattern.map(|p| p.replace(".*", "%").replace('^', "").replace('$', ""));

        match (like_pat.as_deref(), label) {
            (Some(lp), Some(lbl)) => {
                let mut stmt = self.conn.prepare(
                    "SELECT qualified_name, name, label, file_path, line_start, 1.0 AS score
                     FROM symbols WHERE name LIKE ?1 AND label = ?2 LIMIT ?3",
                )?;
                let mut rows = stmt.query(params![lp, lbl, limit as i64])?;
                while let Some(row) = rows.next()? {
                    hits.push(row_to_hit(row)?);
                }
            }
            (Some(lp), None) => {
                let mut stmt = self.conn.prepare(
                    "SELECT qualified_name, name, label, file_path, line_start, 1.0 AS score
                     FROM symbols WHERE name LIKE ?1 LIMIT ?2",
                )?;
                let mut rows = stmt.query(params![lp, limit as i64])?;
                while let Some(row) = rows.next()? {
                    hits.push(row_to_hit(row)?);
                }
            }
            (None, Some(lbl)) => {
                let mut stmt = self.conn.prepare(
                    "SELECT qualified_name, name, label, file_path, line_start, 1.0 AS score
                     FROM symbols WHERE label = ?1 LIMIT ?2",
                )?;
                let mut rows = stmt.query(params![lbl, limit as i64])?;
                while let Some(row) = rows.next()? {
                    hits.push(row_to_hit(row)?);
                }
            }
            (None, None) => {
                let mut stmt = self.conn.prepare(
                    "SELECT qualified_name, name, label, file_path, line_start, 1.0 AS score
                     FROM symbols LIMIT ?1",
                )?;
                let mut rows = stmt.query(params![limit as i64])?;
                while let Some(row) = rows.next()? {
                    hits.push(row_to_hit(row)?);
                }
            }
        }
        Ok(hits)
    }

    pub fn search_code_files(&self, pattern: &str, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT file_path FROM files WHERE content LIKE ?1 LIMIT ?2",
        )?;
        let like = format!("%{pattern}%");
        let mut rows = stmt.query(params![like, limit as i64])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row.get(0)?);
        }
        Ok(out)
    }

    pub fn search_code_matches(&self, pattern: &str, limit: usize) -> Result<Vec<(String, i64, String)>> {
        let mut stmt = self.conn.prepare("SELECT path, content FROM files")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let path: String = row.get(0)?;
            let content: String = row.get(1)?;
            for (i, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    out.push((path.clone(), (i as i64) + 1, line.to_string()));
                    if out.len() >= limit {
                        return Ok(out);
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn get_symbol(&self, qualified_name: &str) -> Result<Option<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT qualified_name, name, label, file_path, line_start, line_end, signature
             FROM symbols WHERE qualified_name = ?1 OR name = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query(params![qualified_name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Symbol {
                qualified_name: row.get(0)?,
                name: row.get(1)?,
                label: row.get(2)?,
                file_path: row.get(3)?,
                line_start: row.get(4)?,
                line_end: row.get(5)?,
                signature: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_snippet(&self, qualified_name: &str) -> Result<Option<String>> {
        let Some(sym) = self.get_symbol(qualified_name)? else {
            return Ok(None);
        };
        let content: String = self.conn.query_row(
            "SELECT content FROM files WHERE path = ?1",
            params![sym.file_path],
            |r| r.get(0),
        )?;
        let lines: Vec<&str> = content.lines().collect();
        let start = (sym.line_start as usize).saturating_sub(1);
        let end = (sym.line_end as usize).min(lines.len());
        let snippet = lines.get(start..end).unwrap_or(&[]).join("\n");
        Ok(Some(format!(
            "// {} ({}:{}-{})\n{}",
            sym.qualified_name, sym.file_path, sym.line_start, sym.line_end, snippet
        )))
    }

    pub fn trace(&self, function_name: &str, depth: i64, direction: &str) -> Result<Vec<TraceHop>> {
        let seed = self
            .get_symbol(function_name)?
            .ok_or_else(|| AppError::msg(format!("symbol not found: {function_name}")))?;
        let mut out = Vec::new();
        let mut frontier = vec![(seed.qualified_name.clone(), 0i64)];
        let mut seen = std::collections::HashSet::new();

        while let Some((qn, d)) = frontier.pop() {
            if d > depth || !seen.insert(qn.clone()) {
                continue;
            }
            if let Some(sym) = self.get_symbol(&qn)? {
                out.push(TraceHop {
                    qualified_name: sym.qualified_name.clone(),
                    name: sym.name.clone(),
                    file_path: sym.file_path.clone(),
                    direction: direction.to_string(),
                    depth: d,
                });
            }
            if direction == "outbound" || direction == "both" {
                let mut stmt = self.conn.prepare(
                    "SELECT dst_qn FROM edges WHERE src_qn = ?1 AND edge_type = 'CALLS'",
                )?;
                let mut rows = stmt.query(params![qn])?;
                while let Some(row) = rows.next()? {
                    frontier.push((row.get(0)?, d + 1));
                }
            }
            if direction == "inbound" || direction == "both" {
                let mut stmt = self.conn.prepare(
                    "SELECT src_qn FROM edges WHERE dst_qn = ?1 AND edge_type = 'CALLS'",
                )?;
                let mut rows = stmt.query(params![qn])?;
                while let Some(row) = rows.next()? {
                    frontier.push((row.get(0)?, d + 1));
                }
            }
        }
        Ok(out)
    }

    pub fn summary(&self) -> Result<IndexSummary> {
        let files: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        let symbols: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))?;
        let edges: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?;
        Ok(IndexSummary {
            project: self.project.clone(),
            repo_path: self.get_meta("repo_path")?.unwrap_or_default(),
            files_indexed: files as usize,
            symbols_indexed: symbols as usize,
            edges_indexed: edges as usize,
            indexed_at: self.get_meta("indexed_at")?.unwrap_or_default(),
        })
    }

    pub fn list_symbol_labels(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT label, COUNT(*) FROM symbols GROUP BY label ORDER BY 2 DESC")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push((row.get(0)?, row.get(1)?));
        }
        Ok(out)
    }

    pub fn graph_schema(&self) -> serde_json::Value {
        let labels = self.list_symbol_labels().unwrap_or_default();
        let edge_types = self.list_edge_types().unwrap_or_default();
        serde_json::json!({
            "engine": "sqlite",
            "cypher_supported": false,
            "node_labels": labels.iter().map(|(l, c)| serde_json::json!({ "label": l, "count": c })).collect::<Vec<_>>(),
            "edge_types": edge_types.iter().map(|(t, c)| serde_json::json!({ "type": t, "count": c })).collect::<Vec<_>>(),
            "tables": ["meta", "files", "symbols", "edges", "symbols_fts"],
            "notes": "Use search_graph/trace_path. query_graph accepts read-only SELECT on these tables."
        })
    }

    fn list_edge_types(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT edge_type, COUNT(*) FROM edges GROUP BY edge_type ORDER BY 2 DESC")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push((row.get(0)?, row.get(1)?));
        }
        Ok(out)
    }

    pub fn query_graph_sql(&self, query: &str, max_rows: usize) -> Result<Vec<serde_json::Value>> {
        let upper = query.trim().to_uppercase();
        if !upper.starts_with("SELECT") {
            return Err(AppError::msg(
                "v0.1: Cypher not supported. Use read-only SELECT on symbols/edges/files, or search_graph/trace_path.",
            ));
        }
        for bad in ["INSERT", "UPDATE", "DELETE", "DROP", "ATTACH", "DETACH", "ALTER", "CREATE"] {
            if upper.contains(bad) {
                return Err(AppError::msg(format!("forbidden keyword in query: {bad}")));
            }
        }
        let sql = format!("{query} LIMIT {}", max_rows.max(1));
        let mut stmt = self.conn.prepare(&sql)?;
        let col_count = stmt.column_count();
        let col_names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("").to_string())
            .collect();
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let mut obj = serde_json::Map::new();
            for (i, name) in col_names.iter().enumerate() {
                let val: rusqlite::types::Value = row.get(i)?;
                obj.insert(name.clone(), sqlite_value_to_json(val));
            }
            out.push(serde_json::Value::Object(obj));
            if out.len() >= max_rows {
                break;
            }
        }
        Ok(out)
    }

    pub fn top_packages(&self, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, COUNT(*) as c FROM symbols GROUP BY file_path ORDER BY c DESC LIMIT ?1",
        )?;
        let mut rows = stmt.query(params![limit as i64])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push((row.get(0)?, row.get(1)?));
        }
        Ok(out)
    }
}

fn sqlite_value_to_json(val: rusqlite::types::Value) -> serde_json::Value {
    match val {
        rusqlite::types::Value::Null => serde_json::Value::Null,
        rusqlite::types::Value::Integer(i) => serde_json::json!(i),
        rusqlite::types::Value::Real(f) => serde_json::json!(f),
        rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
        rusqlite::types::Value::Blob(b) => serde_json::json!(format!("<blob {} bytes>", b.len())),
    }
}

fn row_to_hit(row: &rusqlite::Row<'_>) -> Result<SearchHit> {
    Ok(SearchHit {
        qualified_name: row.get(0)?,
        name: row.get(1)?,
        label: row.get(2)?,
        file_path: row.get(3)?,
        line_start: row.get(4)?,
        score: row.get(5)?,
    })
}

pub fn list_projects() -> Result<Vec<serde_json::Value>> {
    let root = crate::paths::cache_root();
    if !root.exists() {
        return Ok(Vec::new());
    }
    let prefix = crate::paths::project_prefix();
    let mut projects = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".db") {
            continue;
        }
        let project = name.trim_end_matches(".db").to_string();
        if !project.starts_with(&prefix) {
            continue;
        }
        projects.push(serde_json::json!({
            "name": project,
            "upstream_alias": crate::paths::upstream_alias(&project),
            "engine": "codebase-memory-rlm-rs",
        }));
    }
    projects.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    Ok(projects)
}

pub fn delete_project(project: &str) -> Result<()> {
    let path = project_db_path(project);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn project_exists(project: &str) -> bool {
    project_db_path(project).exists()
}