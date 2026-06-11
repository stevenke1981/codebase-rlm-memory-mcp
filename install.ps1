# Install codebase-memory-rlm-rs MCP server + rlm skill (Windows).

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SkillName = "rlm"
$userHome = $env:USERPROFILE
$Binary = Join-Path $ScriptDir "target\release\codebase-memory-rlm.exe"

Write-Host ""
Write-Host "Building release binary..." -ForegroundColor DarkGray
Push-Location $ScriptDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}

function Install-Skill {
    param([string]$TargetDir, [string]$Label)
    New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
    Copy-Item (Join-Path $ScriptDir "SKILL.md") (Join-Path $TargetDir "SKILL.md") -Force
    Write-Host "  ✓ $Label" -ForegroundColor Green
    Write-Host "    → $TargetDir\SKILL.md" -ForegroundColor DarkGray
}

Write-Host "Installing rlm skill..." -ForegroundColor DarkGray
Install-Skill (Join-Path $userHome ".codex\skills\$SkillName") "Codex"
Install-Skill (Join-Path $userHome ".claude\skills\$SkillName") "Claude Code"
Install-Skill (Join-Path $userHome ".agents\skills\$SkillName") "OpenCode / Codex"
Install-Skill (Join-Path $userHome ".config\opencode\skills\$SkillName") "OpenCode native"

Write-Host ""
Write-Host "Binary: $Binary" -ForegroundColor Green
Write-Host ""
Write-Host "Add to agent MCP config:" -ForegroundColor DarkGray
Write-Host "  command: [""$Binary""]" -ForegroundColor DarkGray
Write-Host ""
Write-Host "Or: cargo install --path .  →  codebase-memory-rlm on PATH" -ForegroundColor DarkGray
Write-Host ""