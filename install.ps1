# Install codebase-memory-rlm-rs as codebase-memory-mcp compatible MCP server (Windows).
#
# Usage:
#   .\install.ps1              # build + install + configure agents
#   .\install.ps1 -SkipConfig  # build + install binary only
#   .\install.ps1 -NoReplace   # install to rlm-rs dir only, don't touch upstream binary

param(
    [switch]$SkipConfig,
    [switch]$NoReplace
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SkillName = "rlm"
$userHome = $env:USERPROFILE

$OpenCodeBinDir = Join-Path $userHome ".config\opencode-codebase-memory-mcp\bin"
$CodexBinDir = Join-Path $env:LOCALAPPDATA "Programs\codebase-memory-mcp"
$RlmBinDir = Join-Path $userHome ".config\opencode-codebase-memory-rlm-rs\bin"

$BuiltBinary = Join-Path $ScriptDir "target\release\codebase-memory-rlm.exe"
$McpBinaryName = "codebase-memory-mcp.exe"

function Write-Step([string]$Msg) {
    Write-Host ""
    Write-Host $Msg -ForegroundColor DarkGray
}

Write-Step "Building release binary..."
Push-Location $ScriptDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
    Pop-Location
}

if (-not (Test-Path $BuiltBinary)) {
    throw "Binary not found: $BuiltBinary"
}

function Install-Binary {
    param(
        [string]$TargetDir,
        [string]$Label,
        [bool]$BackupExisting = $true
    )
    New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
    $dest = Join-Path $TargetDir $McpBinaryName
    if ($BackupExisting -and (Test-Path $dest)) {
        $bak = "$dest.upstream.bak"
        Copy-Item $dest $bak -Force -ErrorAction SilentlyContinue
        Write-Host "  backed up → $bak" -ForegroundColor DarkGray
        $old = "$dest.old"
        Remove-Item $old -Force -ErrorAction SilentlyContinue
        try {
            Rename-Item $dest $old -ErrorAction Stop
        } catch {
            Write-Host "  ! locked, skipped in-place replace ($dest)" -ForegroundColor Yellow
            return $null
        }
    }
    Copy-Item $BuiltBinary $dest -Force
    Write-Host "  ✓ $Label" -ForegroundColor Green
    Write-Host "    → $dest" -ForegroundColor DarkGray
    return $dest
}

Write-Step "Installing binaries..."
$primaryBinary = Install-Binary -TargetDir $RlmBinDir -Label "RLM-RS primary" -BackupExisting $false
Copy-Item $BuiltBinary (Join-Path $RlmBinDir "codebase-memory-rlm.exe") -Force

$openCodeInstalled = $null
$codexInstalled = $null
if (-not $NoReplace) {
    $openCodeInstalled = Install-Binary -TargetDir $OpenCodeBinDir -Label "OpenCode (drop-in)"
    $codexInstalled = Install-Binary -TargetDir $CodexBinDir -Label "Codex (drop-in)"
}

function Install-Skill {
    param([string]$TargetDir, [string]$Label)
    New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
    Copy-Item (Join-Path $ScriptDir "SKILL.md") (Join-Path $TargetDir "SKILL.md") -Force
    Write-Host "  ✓ $Label" -ForegroundColor Green
}

Write-Step "Installing rlm skill..."
Install-Skill (Join-Path $userHome ".codex\skills\$SkillName") "Codex"
Install-Skill (Join-Path $userHome ".claude\skills\$SkillName") "Claude Code"
Install-Skill (Join-Path $userHome ".agents\skills\$SkillName") "OpenCode / Codex"
Install-Skill (Join-Path $userHome ".config\opencode\skills\$SkillName") "OpenCode native"

if ($SkipConfig) {
    Write-Host ""
    Write-Host "Skipping agent MCP configuration (-SkipConfig)." -ForegroundColor Yellow
    exit 0
}

$openCodeConfig = Join-Path $userHome ".config\opencode\opencode.json"
$openCodeSidecar = Join-Path $userHome ".config\opencode\codebase-memory-mcp.json"
$codexConfig = Join-Path $userHome ".codex\config.toml"

$activeBinary = if ($openCodeInstalled) {
    $openCodeInstalled
} else {
    Join-Path $RlmBinDir $McpBinaryName
}

$mcpCommand = @(
    "pwsh",
    "-NoProfile",
    "-Command",
    "& `"$activeBinary`""
)

Write-Step "Configuring OpenCode MCP..."
if (Test-Path $openCodeConfig) {
    $cfg = Get-Content $openCodeConfig -Raw | ConvertFrom-Json
    if (-not $cfg.mcp) { $cfg | Add-Member -NotePropertyName mcp -NotePropertyValue (@{}) }
    $cfg.mcp."codebase-memory-mcp" = @{
        type    = "local"
        command = $mcpCommand
        enabled = $true
        timeout = 120000
    }
    $cfg | ConvertTo-Json -Depth 20 | Set-Content $openCodeConfig -Encoding UTF8
    Write-Host "  ✓ Updated $openCodeConfig" -ForegroundColor Green
} else {
    Write-Host "  ! opencode.json not found, skipped" -ForegroundColor Yellow
}

@{
    timeout         = 120000
    mcpName         = "codebase-memory-mcp"
    autoDownload    = $false
    variant         = "rust-rlm"
    binaryPath      = $activeBinary
    mcpRelativePath = $activeBinary.Replace("$userHome\", "").Replace("\", "/")
    mcpCommand      = $mcpCommand
    engine          = "codebase-memory-rlm-rs"
} | ConvertTo-Json -Depth 5 | Set-Content $openCodeSidecar -Encoding UTF8
Write-Host "  ✓ Updated $openCodeSidecar" -ForegroundColor Green

Write-Step "Configuring Codex MCP..."
$codexBinary = if ($codexInstalled) { $codexInstalled } else { $activeBinary }
$codexBinary = $codexBinary -replace '\\', '/'
if (Test-Path $codexConfig) {
    $toml = Get-Content $codexConfig -Raw
    $section = "[mcp_servers.codebase-memory-mcp]"
    $newBlock = "$section`ntype = `"stdio`"`ncommand = `"$codexBinary`"`n"
    if ($toml -match '\[mcp_servers\.codebase-memory-mcp\]') {
        $toml = $toml -replace '(?s)\[mcp_servers\.codebase-memory-mcp\][^\[]*', "$newBlock"
    } else {
        $toml = $toml.TrimEnd() + "`n`n$newBlock"
    }
    Set-Content $codexConfig $toml -Encoding UTF8 -NoNewline
    Write-Host "  ✓ Updated $codexConfig" -ForegroundColor Green
} else {
    Write-Host "  ! config.toml not found, skipped" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Done! Restart your coding agent." -ForegroundColor Green
Write-Host ""
Write-Host "MCP server name: codebase-memory-mcp (compatible with upstream skills)" -ForegroundColor DarkGray
Write-Host "Primary binary:  $primaryBinary" -ForegroundColor DarkGray
Write-Host "Cache dir:       $env:LOCALAPPDATA\codebase-memory-mcp (or set CBM_CACHE_DIR)" -ForegroundColor DarkGray
Write-Host ""
Write-Host "Upstream binaries backed up as *.upstream.bak if replaced." -ForegroundColor DarkGray
Write-Host "Use -NoReplace to keep upstream binary and only install to rlm-rs dir." -ForegroundColor DarkGray
Write-Host ""