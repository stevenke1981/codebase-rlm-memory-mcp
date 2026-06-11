# Install codebase-rlm-memory-mcp (CBRLM) MCP server + rlm skill (Windows).
#
# Usage:
#   .\install.ps1              # build + install + configure agents
#   .\install.ps1 -SkipConfig  # build + install binary only

param(
    [switch]$SkipConfig
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SkillName = "rlm"
$userHome = $env:USERPROFILE

$CbrlmBinDir = Join-Path $userHome ".config\opencode-cbrlm\bin"
$BuiltBinary = Join-Path $ScriptDir "target\release\cbrlm.exe"
$McpBinaryName = "cbrlm.exe"
$McpServerName = "codebase-rlm-memory-mcp"

function Write-Step([string]$Msg) {
    Write-Host ""
    Write-Host $Msg -ForegroundColor DarkGray
}

Write-Step "Building release binary (cbrlm)..."
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

New-Item -ItemType Directory -Force -Path $CbrlmBinDir | Out-Null
$dest = Join-Path $CbrlmBinDir $McpBinaryName
Copy-Item $BuiltBinary $dest -Force
Write-Host "  ✓ CBRLM binary" -ForegroundColor Green
Write-Host "    → $dest" -ForegroundColor DarkGray

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
$openCodeSidecar = Join-Path $userHome ".config\opencode\codebase-rlm-memory-mcp.json"
$codexConfig = Join-Path $userHome ".codex\config.toml"

$mcpCommand = @(
    "pwsh",
    "-NoProfile",
    "-Command",
    "& `"$dest`""
)

Write-Step "Configuring OpenCode MCP ($McpServerName)..."
if (Test-Path $openCodeConfig) {
    $cfg = Get-Content $openCodeConfig -Raw | ConvertFrom-Json
    $mcp = @{}
    if ($cfg.mcp) {
        $cfg.mcp.PSObject.Properties | ForEach-Object { $mcp[$_.Name] = $_.Value }
    }
    $mcp[$McpServerName] = @{
        type        = "local"
        command     = $mcpCommand
        enabled     = $true
        timeout     = 120000
        environment = @{
            CBRLM_PROJECT_PREFIX = "cbrlm+"
        }
    }
    $cfg | Add-Member -NotePropertyName mcp -NotePropertyValue $mcp -Force
    $cfg | ConvertTo-Json -Depth 20 | Set-Content $openCodeConfig -Encoding UTF8
    Write-Host "  ✓ Updated $openCodeConfig" -ForegroundColor Green
} else {
    Write-Host "  ! opencode.json not found, skipped" -ForegroundColor Yellow
}

@{
    timeout         = 120000
    mcpName         = $McpServerName
    abbrev          = "cbrlm"
    autoDownload    = $false
    binaryPath      = $dest
    mcpRelativePath = ".config/opencode-cbrlm/bin/cbrlm.exe"
    mcpCommand      = $mcpCommand
    engine          = "codebase-rlm-memory-mcp"
    projectPrefix   = "cbrlm+"
} | ConvertTo-Json -Depth 5 | Set-Content $openCodeSidecar -Encoding UTF8
Write-Host "  ✓ Updated $openCodeSidecar" -ForegroundColor Green

Write-Step "Configuring Codex MCP ($McpServerName)..."
$codexBinary = $dest -replace '\\', '/'
if (Test-Path $codexConfig) {
    $toml = Get-Content $codexConfig -Raw
    $section = "[mcp_servers.$McpServerName]"
    $newBlock = @"
$section
type = "stdio"
command = "$codexBinary"

[mcp_servers.$McpServerName.env]
CBRLM_PROJECT_PREFIX = "cbrlm+"
"@
    if ($toml -match "\[mcp_servers\.$([regex]::Escape($McpServerName))\]") {
        $toml = $toml -replace "(?s)\[mcp_servers\.$([regex]::Escape($McpServerName))\][^\[]*", "$newBlock"
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
Write-Host "Project:  codebase-rlm-memory-mcp (CBRLM)" -ForegroundColor DarkGray
Write-Host "MCP name: $McpServerName" -ForegroundColor DarkGray
Write-Host "Binary:   $dest" -ForegroundColor DarkGray
Write-Host "Projects: cbrlm+<upstream_key> in ~/.cache/codebase-memory-mcp/" -ForegroundColor DarkGray
Write-Host ""