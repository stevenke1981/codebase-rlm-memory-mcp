use std::path::Path;
use std::process::Command;

use crate::error::Result;

pub fn detect_changed_files(repo_path: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "status", "--porcelain"])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in text.lines() {
        if line.len() < 4 {
            continue;
        }
        let path = line[3..].trim();
        if !path.is_empty() {
            out.push(path.replace('\\', "/"));
        }
    }
    Ok(out)
}
