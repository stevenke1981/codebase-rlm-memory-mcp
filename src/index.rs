use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use ignore::WalkBuilder;
use regex::Regex;

use crate::error::Result;
use crate::models::Symbol;
use crate::store::Store;

const SKIP_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "svg", "woff", "woff2", "ttf", "eot", "mp4", "zip", "gz",
    "tar", "pdf", "exe", "dll", "so", "dylib", "lock", "sum",
];

type LangPatterns = &'static [(&'static str, &'static str, &'static str)];

static RUST_PATTERNS: LangPatterns = &[
    (
        r"(?m)^\s*pub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)",
        "Function",
        "pub fn",
    ),
    (r"(?m)^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)", "Function", "fn"),
    (
        r"(?m)^\s*pub\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)",
        "Class",
        "struct",
    ),
    (
        r"(?m)^\s*pub\s+enum\s+([A-Za-z_][A-Za-z0-9_]*)",
        "Class",
        "enum",
    ),
    (
        r"(?m)^\s*pub\s+trait\s+([A-Za-z_][A-Za-z0-9_]*)",
        "Interface",
        "trait",
    ),
];

static JS_PATTERNS: LangPatterns = &[
    (
        r"(?m)^\s*export\s+(?:async\s+)?function\s+([A-Za-z_$][\w$]*)",
        "Function",
        "function",
    ),
    (
        r"(?m)^\s*export\s+class\s+([A-Za-z_$][\w$]*)",
        "Class",
        "class",
    ),
    (r"(?m)^\s*class\s+([A-Za-z_$][\w$]*)", "Class", "class"),
    (
        r#"(?m)^\s*@(?:Get|Post|Put|Delete|Patch)\([^)]*\)"#,
        "Route",
        "route",
    ),
];

static PY_PATTERNS: LangPatterns = &[
    (r"(?m)^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)", "Function", "def"),
    (
        r"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)",
        "Class",
        "class",
    ),
];

static GO_PATTERNS: LangPatterns = &[
    (
        r"(?m)^\s*func\s+([A-Za-z_][A-Za-z0-9_]*)",
        "Function",
        "func",
    ),
    (
        r"(?m)^\s*type\s+([A-Za-z_][A-Za-z0-9_]*)\s+struct",
        "Class",
        "struct",
    ),
];

static GENERIC_PATTERNS: LangPatterns = &[
    (
        r"(?m)^\s*(?:pub\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)",
        "Function",
        "fn",
    ),
    (
        r"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)",
        "Class",
        "class",
    ),
    (
        r"(?m)^\s*function\s+([A-Za-z_$][\w$]*)",
        "Function",
        "function",
    ),
];

static RUST_RES: LazyLock<Vec<(&'static str, Regex, &'static str)>> =
    LazyLock::new(|| compile_patterns(RUST_PATTERNS));

static JS_RES: LazyLock<Vec<(&'static str, Regex, &'static str)>> =
    LazyLock::new(|| compile_patterns(JS_PATTERNS));

static PY_RES: LazyLock<Vec<(&'static str, Regex, &'static str)>> =
    LazyLock::new(|| compile_patterns(PY_PATTERNS));

static GO_RES: LazyLock<Vec<(&'static str, Regex, &'static str)>> =
    LazyLock::new(|| compile_patterns(GO_PATTERNS));

static GENERIC_RES: LazyLock<Vec<(&'static str, Regex, &'static str)>> =
    LazyLock::new(|| compile_patterns(GENERIC_PATTERNS));

fn compile_patterns(patterns: LangPatterns) -> Vec<(&'static str, Regex, &'static str)> {
    patterns
        .iter()
        .filter_map(|(pat, label, kind)| Regex::new(pat).ok().map(|re| (*label, re, *kind)))
        .collect()
}

pub struct Indexer;

impl Indexer {
    pub fn index_repo(store: &Store, repo_path: &Path, mode: &str) -> Result<usize> {
        let fast = mode == "fast";
        store.clear()?;
        let canonical = repo_path
            .canonicalize()
            .unwrap_or_else(|_| repo_path.to_path_buf());
        store.set_meta("repo_path", &canonical.to_string_lossy())?;
        store.set_meta("indexed_at", &chrono_lite_now())?;
        store.set_meta("index_mode", mode)?;
        store.begin_batch()?;

        let mut symbol_count = 0;
        let result = (|| -> Result<usize> {
            for file in walk_repo(&canonical)? {
                let rel = file
                    .strip_prefix(&canonical)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .replace('\\', "/");
                let content = match std::fs::read_to_string(&file) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let lang = detect_language(&file);
                let line_count = content.lines().count() as i64;
                store.insert_file(&rel, &content, lang, line_count)?;

                let symbols = extract_symbols(&rel, &content, lang);
                let names: Vec<String> = symbols.iter().map(|s| s.name.clone()).collect();
                for sym in &symbols {
                    store.insert_symbol(sym)?;
                    symbol_count += 1;
                }
                if !fast {
                    infer_calls(store, &rel, &content, &symbols, &names)?;
                }
            }
            Ok(symbol_count)
        })();

        match result {
            Ok(count) => {
                store.commit_batch()?;
                Ok(count)
            }
            Err(e) => {
                store.rollback_batch();
                Err(e)
            }
        }
    }
}

fn walk_repo(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();
    for result in walker {
        let entry = result?;
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.into_path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if SKIP_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                continue;
            }
        }
        if path.components().any(|c| {
            matches!(
                c.as_os_str().to_string_lossy().as_ref(),
                "node_modules" | ".git" | "target" | "dist" | "build" | ".venv" | "venv"
            )
        }) {
            continue;
        }
        files.push(path);
    }
    Ok(files)
}

fn detect_language(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("c") | Some("h") => "c",
        Some("cpp") | Some("hpp") | Some("cc") => "cpp",
        Some("cs") => "csharp",
        Some("php") => "php",
        Some("rb") => "ruby",
        Some("swift") => "swift",
        Some("kt") => "kotlin",
        Some("sql") => "sql",
        Some("md") => "markdown",
        Some("json") | Some("yaml") | Some("yml") | Some("toml") => "config",
        _ => "text",
    }
}

fn patterns_for(lang: &str) -> &'static Vec<(&'static str, Regex, &'static str)> {
    match lang {
        "rust" => &RUST_RES,
        "typescript" | "javascript" => &JS_RES,
        "python" => &PY_RES,
        "go" => &GO_RES,
        _ => &GENERIC_RES,
    }
}

fn extract_symbols(file_path: &str, content: &str, lang: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    for (label, re, kind) in patterns_for(lang).iter() {
        for cap in re.captures_iter(content) {
            let Some(name) = cap.get(1) else { continue };
            let name = name.as_str().to_string();
            let line = content[..cap.get(0).unwrap().start()].lines().count() as i64 + 1;
            let qn = format!("{file_path}::{name}");
            out.push(Symbol {
                qualified_name: qn,
                name,
                label: (*label).to_string(),
                file_path: file_path.to_string(),
                line_start: line,
                line_end: line + 20,
                signature: format!("{kind} ..."),
            });
        }
    }
    out
}

fn infer_calls(
    store: &Store,
    file_path: &str,
    content: &str,
    symbols: &[Symbol],
    names: &[String],
) -> Result<()> {
    if symbols.is_empty() {
        return Ok(());
    }
    let caller = symbols
        .iter()
        .find(|s| s.label == "Function")
        .or_else(|| symbols.first());
    let Some(caller) = caller else { return Ok(()) };

    for name in names {
        if name == &caller.name {
            continue;
        }
        let needle = format!("{name}(");
        if content.contains(&needle) {
            let dst = format!("{file_path}::{name}");
            store.insert_edge(&caller.qualified_name, &dst, "CALLS")?;
        }
    }
    Ok(())
}

fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

pub fn default_project_name(repo_path: &Path) -> String {
    crate::paths::default_project_name(repo_path)
}
