#!/usr/bin/env bash
# codebase-rlm-memory-mcp search augmenter (Claude Code PreToolUse).
# NEVER blocks a tool call — only adds graph context. Failures are silent (exit 0).
set -euo pipefail
BIN="${CBRLM_BIN:-{{CBRLM_BIN}}}"
[ -x "$BIN" ] || exit 0
"$BIN" hook-augment 2>/dev/null || true
exit 0