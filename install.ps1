# Install codebase-rlm-memory-mcp (CBRLM) MCP server + rlm skill + hooks (Windows).
#
# Usage:
#   .\install.ps1              # build + install + configure agents + hooks
#   .\install.ps1 -SkipConfig  # build + install binary only

param(
    [switch]$SkipConfig
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SkillName = "rlm"
$userHome = $env:USERPROFILE

$CbrlmBinDir = Join-Path $userHome ".config\opencode-cbrlm\bin"
$CbrlmHooksDir = Join-Path $userHome ".config\opencode-cbrlm\hooks"
$BuiltBinary = Join-Path $ScriptDir "target\release\cbrlm.exe"
$McpBinaryName = "cbrlm.exe"
$McpServerName = "codebase-rlm-memory-mcp"
$CodexHookBegin = "# >>> codebase-rlm-memory-mcp SessionStart >>>"
$CodexHookEnd = "# <<< codebase-rlm-memory-mcp SessionStart <<<"
$CodexReminderCmd = 'echo "Code discovery: prefer codebase-rlm-memory-mcp (search_graph, trace_path, rlm_filter, rlm_read_symbol) over grep/file-read; projects use cbrlm+ prefix; run index_repository first if not indexed."'

function Write-Step([string]$Msg) {
    Write-Host ""
    Write-Host $Msg -ForegroundColor DarkGray
}

function Install-HookScript {
    param(
        [string]$TemplateName,
        [string]$DestPath,
        [string]$BinaryPath
    )
    $template = Join-Path $ScriptDir "hooks\$TemplateName"
    if (-not (Test-Path $template)) {
        throw "Hook template not found: $template"
    }
    $content = Get-Content $template -Raw
    $escaped = $BinaryPath -replace '\\', '/'
    $content = $content.Replace('{{CBRLM_BIN}}', $escaped)
    Set-Content -Path $DestPath -Value $content -Encoding UTF8 -NoNewline
}

function Install-Hooks {
    param([string]$BinaryPath)

    $claudeHooksDir = if ($env:CLAUDE_CONFIG_DIR) {
        Join-Path $env:CLAUDE_CONFIG_DIR "hooks"
    } else {
        Join-Path $userHome ".claude\hooks"
    }

    foreach ($dir in @($CbrlmHooksDir, $claudeHooksDir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Install-HookScript "cbrlm-code-discovery-gate.ps1" (Join-Path $dir "cbrlm-code-discovery-gate.ps1") $BinaryPath
        Install-HookScript "cbrlm-session-reminder.ps1" (Join-Path $dir "cbrlm-session-reminder.ps1") $BinaryPath
        Install-HookScript "cbrlm-code-discovery-gate.sh" (Join-Path $dir "cbrlm-code-discovery-gate") $BinaryPath
        Install-HookScript "cbrlm-session-reminder.sh" (Join-Path $dir "cbrlm-session-reminder") $BinaryPath
    }
    Write-Host "  ✓ Hook scripts → $claudeHooksDir" -ForegroundColor Green
    Write-Host "  ✓ Hook scripts → $CbrlmHooksDir" -ForegroundColor Green
}

function Update-CodexSessionHooks {
    param([string]$ConfigPath)
    if (-not (Test-Path $ConfigPath)) {
        Write-Host "  ! config.toml not found, skipped Codex hooks" -ForegroundColor Yellow
        return
    }
    $block = @"

$CodexHookBegin
[[hooks.SessionStart]]
matcher = "startup|resume|clear|compact"

[[hooks.SessionStart.hooks]]
type = "command"
command = '$CodexReminderCmd'
$CodexHookEnd
"@
    $toml = Get-Content $ConfigPath -Raw
    if ($toml -match [regex]::Escape($CodexHookBegin)) {
        $toml = $toml -replace "(?s)\r?\n?$([regex]::Escape($CodexHookBegin)).*?$([regex]::Escape($CodexHookEnd))\r?\n?", ""
    }
    $toml = $toml.TrimEnd() + $block
    Set-Content $ConfigPath $toml -Encoding UTF8 -NoNewline
    Write-Host "  ✓ Codex SessionStart hooks" -ForegroundColor Green
}

function Update-ClaudeHooks {
    param(
        [string]$SettingsPath,
        [string]$GateCommand,
        [string]$SessionCommand
    )
    $settings = @{}
    if (Test-Path $SettingsPath) {
        $settings = Get-Content $SettingsPath -Raw | ConvertFrom-Json -AsHashtable -Depth 20
    }
    if (-not $settings.ContainsKey('hooks')) {
        $settings['hooks'] = @{}
    }

    $pre = @()
    if ($settings['hooks'].ContainsKey('PreToolUse')) {
        $pre = @($settings['hooks']['PreToolUse']) | Where-Object {
            $cmd = ''
            if ($_.hooks -and $_.hooks.Count -gt 0) { $cmd = [string]$_.hooks[0].command }
            $cmd -notlike '*cbrlm-code-discovery-gate*'
        }
    }
    $pre += @{
        matcher = 'Grep|Glob'
        hooks   = @(
            @{
                type    = 'command'
                command = $GateCommand
                timeout = 5
            }
        )
    }
    $settings['hooks']['PreToolUse'] = $pre

    $sessionMatchers = @('startup', 'resume', 'clear', 'compact')
    $session = @()
    if ($settings['hooks'].ContainsKey('SessionStart')) {
        $session = @($settings['hooks']['SessionStart']) | Where-Object {
            $cmd = ''
            if ($_.hooks -and $_.hooks.Count -gt 0) { $cmd = [string]$_.hooks[0].command }
            -not ($cmd -like '*cbrlm-session-reminder*')
        }
    }
    foreach ($matcher in $sessionMatchers) {
        $session += @{
            matcher = $matcher
            hooks   = @(
                @{
                    type    = 'command'
                    command = $SessionCommand
                }
            )
        }
    }
    $settings['hooks']['SessionStart'] = $session

    $settings | ConvertTo-Json -Depth 20 | Set-Content $SettingsPath -Encoding UTF8
    Write-Host "  ✓ Claude hooks ($SettingsPath)" -ForegroundColor Green
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
if (Test-Path $dest) {
    $oldDest = "$dest.old"
    Remove-Item $oldDest -Force -ErrorAction SilentlyContinue
    try {
        Rename-Item $dest $oldDest -ErrorAction Stop
    } catch {
        Write-Host "  ! Could not replace in-use binary; copying alongside" -ForegroundColor Yellow
    }
}
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

Write-Step "Installing CBRLM hooks..."
Install-Hooks $dest

if ($SkipConfig) {
    Write-Host ""
    Write-Host "Skipping agent MCP configuration (-SkipConfig)." -ForegroundColor Yellow
    exit 0
}

$openCodeConfig = Join-Path $userHome ".config\opencode\opencode.json"
$openCodeSidecar = Join-Path $userHome ".config\opencode\codebase-rlm-memory-mcp.json"
$codexConfig = Join-Path $userHome ".codex\config.toml"
$claudeSettings = Join-Path $userHome ".claude\settings.json"

$claudeHooksDir = if ($env:CLAUDE_CONFIG_DIR) {
    Join-Path $env:CLAUDE_CONFIG_DIR "hooks"
} else {
    Join-Path $userHome ".claude\hooks"
}
$gatePs1 = Join-Path $claudeHooksDir "cbrlm-code-discovery-gate.ps1"
$sessionPs1 = Join-Path $claudeHooksDir "cbrlm-session-reminder.ps1"
$gateCmd = "pwsh -NoProfile -File `"$gatePs1`""
$sessionCmd = "pwsh -NoProfile -File `"$sessionPs1`""

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

Write-Step "Configuring Codex MCP + SessionStart hooks..."
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
    Write-Host "  ✓ Updated Codex MCP in $codexConfig" -ForegroundColor Green
    Update-CodexSessionHooks $codexConfig
} else {
    Write-Host "  ! config.toml not found, skipped" -ForegroundColor Yellow
}

Write-Step "Configuring Claude Code hooks..."
New-Item -ItemType Directory -Force -Path (Split-Path $claudeSettings) | Out-Null
Update-ClaudeHooks $claudeSettings $gateCmd $sessionCmd

Write-Host ""
Write-Host "Done! Restart your coding agent." -ForegroundColor Green
Write-Host ""
Write-Host "Project:  codebase-rlm-memory-mcp (CBRLM)" -ForegroundColor DarkGray
Write-Host "MCP name: $McpServerName" -ForegroundColor DarkGray
Write-Host "Binary:   $dest" -ForegroundColor DarkGray
Write-Host "Hooks:    SessionStart (Codex/Claude) + PreToolUse Grep|Glob augment (Claude)" -ForegroundColor DarkGray
Write-Host "Projects: cbrlm+<upstream_key> in ~/.cache/codebase-memory-mcp/" -ForegroundColor DarkGray
Write-Host ""