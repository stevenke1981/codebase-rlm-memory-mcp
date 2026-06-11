#!/usr/bin/env bash
# Install codebase-memory-rlm-rs MCP server + rlm skill.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_NAME="rlm"
BINARY="$SCRIPT_DIR/target/release/codebase-memory-rlm"

GREEN='\033[0;32m'
GRAY='\033[0;90m'
NC='\033[0m'

echo ""
echo -e "${GRAY}Building release binary...${NC}"
(cd "$SCRIPT_DIR" && cargo build --release)

install_skill() {
  local target_dir="$1"
  local label="$2"
  mkdir -p "$target_dir"
  cp "$SCRIPT_DIR/SKILL.md" "$target_dir/SKILL.md"
  echo -e "${GREEN}  ✓ ${label}${NC}"
  echo -e "${GRAY}    → ${target_dir}/SKILL.md${NC}"
}

echo -e "${GRAY}Installing rlm skill...${NC}"
install_skill "$HOME/.codex/skills/$SKILL_NAME" "Codex (~/.codex/skills/)"
install_skill "$HOME/.claude/skills/$SKILL_NAME" "Claude Code (~/.claude/skills/)"
install_skill "$HOME/.agents/skills/$SKILL_NAME" "OpenCode / Codex (~/.agents/skills/)"
install_skill "$HOME/.config/opencode/skills/$SKILL_NAME" "OpenCode (~/.config/opencode/skills/)"

echo ""
echo -e "${GREEN}Binary: ${BINARY}${NC}"
echo ""
echo -e "${GRAY}Add to your agent MCP config:${NC}"
echo -e "${GRAY}  \"codebase-memory-rlm-rs\": {"
echo -e "${GRAY}    \"command\": [\"${BINARY}\"]"
echo -e "${GRAY}  }${NC}"
echo ""
echo -e "${GRAY}Or: cargo install --path .  →  codebase-memory-rlm on PATH${NC}"
echo ""