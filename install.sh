#!/usr/bin/env bash
# Install codebase-rlm-memory-mcp (CBRLM) MCP server + rlm skill.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_NAME="rlm"
BINARY="$SCRIPT_DIR/target/release/cbrlm"
INSTALL_DIR="$HOME/.config/opencode-cbrlm/bin"

GREEN='\033[0;32m'
GRAY='\033[0;90m'
NC='\033[0m'

echo ""
echo -e "${GRAY}Building release binary (cbrlm)...${NC}"
(cd "$SCRIPT_DIR" && cargo build --release)

mkdir -p "$INSTALL_DIR"
cp "$BINARY" "$INSTALL_DIR/cbrlm"
chmod +x "$INSTALL_DIR/cbrlm"

install_skill() {
  local target_dir="$1"
  local label="$2"
  mkdir -p "$target_dir"
  cp "$SCRIPT_DIR/SKILL.md" "$target_dir/SKILL.md"
  echo -e "${GREEN}  ✓ ${label}${NC}"
}

echo -e "${GRAY}Installing rlm skill...${NC}"
install_skill "$HOME/.codex/skills/$SKILL_NAME" "Codex"
install_skill "$HOME/.claude/skills/$SKILL_NAME" "Claude Code"
install_skill "$HOME/.agents/skills/$SKILL_NAME" "OpenCode / Codex"
install_skill "$HOME/.config/opencode/skills/$SKILL_NAME" "OpenCode"

echo ""
echo -e "${GREEN}CBRLM binary: ${INSTALL_DIR}/cbrlm${NC}"
echo ""
echo -e "${GRAY}Add to agent MCP config:${NC}"
echo -e "${GRAY}  \"codebase-rlm-memory-mcp\": {"
echo -e "${GRAY}    \"command\": [\"${INSTALL_DIR}/cbrlm\"],"
echo -e "${GRAY}    \"environment\": { \"CBRLM_PROJECT_PREFIX\": \"cbrlm+\" }"
echo -e "${GRAY}  }${NC}"
echo ""