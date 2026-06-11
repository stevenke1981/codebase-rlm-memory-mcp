#!/usr/bin/env bash
# Install codebase-rlm-memory-mcp (CBRLM) MCP server + rlm skill + hooks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_NAME="rlm"
BINARY="$SCRIPT_DIR/target/release/cbrlm"
INSTALL_DIR="$HOME/.config/opencode-cbrlm/bin"
HOOKS_DIR="$HOME/.config/opencode-cbrlm/hooks"
CLAUDE_HOOKS_DIR="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/hooks"
CODEX_CONFIG="$HOME/.codex/config.toml"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"
CODEX_HOOK_BEGIN="# >>> codebase-rlm-memory-mcp SessionStart >>>"
CODEX_HOOK_END="# <<< codebase-rlm-memory-mcp SessionStart <<<"
CODEX_REMINDER_CMD='echo "Code discovery: prefer codebase-rlm-memory-mcp (search_graph, trace_path, rlm_filter, rlm_read_symbol) over grep/file-read; projects use cbrlm+ prefix; run index_repository first if not indexed."'

GREEN='\033[0;32m'
GRAY='\033[0;90m'
YELLOW='\033[1;33m'
NC='\033[0m'

SKIP_CONFIG=false
for arg in "$@"; do
  case "$arg" in
    --skip-config) SKIP_CONFIG=true ;;
  esac
done

install_hook() {
  local template="$1"
  local dest="$2"
  local bin_path="$3"
  sed "s|{{CBRLM_BIN}}|${bin_path//\//\\/}|g" "$SCRIPT_DIR/hooks/$template" > "$dest"
  chmod +x "$dest"
}

install_hooks() {
  local bin_path="$1"
  mkdir -p "$HOOKS_DIR" "$CLAUDE_HOOKS_DIR"
  for dir in "$HOOKS_DIR" "$CLAUDE_HOOKS_DIR"; do
    install_hook "cbrlm-code-discovery-gate.sh" "$dir/cbrlm-code-discovery-gate" "$bin_path"
    install_hook "cbrlm-session-reminder.sh" "$dir/cbrlm-session-reminder" "$bin_path"
  done
  echo -e "${GREEN}  ✓ Hook scripts → $CLAUDE_HOOKS_DIR${NC}"
  echo -e "${GREEN}  ✓ Hook scripts → $HOOKS_DIR${NC}"
}

upsert_codex_hooks() {
  local config="$1"
  [ -f "$config" ] || return 0
  local block
  block=$(cat <<EOF

${CODEX_HOOK_BEGIN}
[[hooks.SessionStart]]
matcher = "startup|resume|clear|compact"

[[hooks.SessionStart.hooks]]
type = "command"
command = '${CODEX_REMINDER_CMD}'
${CODEX_HOOK_END}
EOF
)
  python3 - "$config" "$CODEX_HOOK_BEGIN" "$CODEX_HOOK_END" "$block" <<'PY'
import re, sys
path, begin, end, block = sys.argv[1:5]
with open(path, encoding="utf-8") as f:
    text = f.read()
pattern = re.compile(r"\n?" + re.escape(begin) + r".*?" + re.escape(end) + r"\n?", re.S)
text = pattern.sub("", text).rstrip() + block
with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(text)
PY
  echo -e "${GREEN}  ✓ Codex SessionStart hooks${NC}"
}

upsert_claude_hooks() {
  local settings="$1"
  local gate_cmd="$2"
  local session_cmd="$3"
  mkdir -p "$(dirname "$settings")"
  python3 - "$settings" "$gate_cmd" "$session_cmd" <<'PY'
import json, os, sys
path, gate_cmd, session_cmd = sys.argv[1:4]
data = {}
if os.path.isfile(path):
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
hooks = data.setdefault("hooks", {})
old = {"Grep|Glob", "Grep|Glob|Read", "Grep|Glob|Read|Search"}
pre = []
for entry in hooks.get("PreToolUse", []):
    cmd = ""
    if entry.get("hooks"):
        cmd = entry["hooks"][0].get("command", "")
    if entry.get("matcher") in old and "cbrlm-code-discovery-gate" in cmd:
        continue
    pre.append(entry)
pre.append({
    "matcher": "Grep|Glob",
    "hooks": [{"type": "command", "command": gate_cmd, "timeout": 5}],
})
hooks["PreToolUse"] = pre
session = []
for entry in hooks.get("SessionStart", []):
    cmd = ""
    if entry.get("hooks"):
        cmd = entry["hooks"][0].get("command", "")
    if "cbrlm-session-reminder" in cmd:
        continue
    session.append(entry)
for matcher in ("startup", "resume", "clear", "compact"):
    session.append({
        "matcher": matcher,
        "hooks": [{"type": "command", "command": session_cmd}],
    })
hooks["SessionStart"] = session
with open(path, "w", encoding="utf-8") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
PY
  echo -e "${GREEN}  ✓ Claude hooks ($settings)${NC}"
}

echo ""
echo -e "${GRAY}Building release binary (cbrlm)...${NC}"
(cd "$SCRIPT_DIR" && cargo build --release)

mkdir -p "$INSTALL_DIR"
cp "$BINARY" "$INSTALL_DIR/cbrlm"
chmod +x "$INSTALL_DIR/cbrlm"
INSTALLED_BIN="$INSTALL_DIR/cbrlm"

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

echo -e "${GRAY}Installing CBRLM hooks...${NC}"
install_hooks "$INSTALLED_BIN"

if [ "$SKIP_CONFIG" = true ]; then
  echo ""
  echo -e "${YELLOW}Skipping agent configuration (--skip-config).${NC}"
  exit 0
fi

echo -e "${GRAY}Configuring Codex SessionStart hooks...${NC}"
upsert_codex_hooks "$CODEX_CONFIG"

GATE_CMD="$CLAUDE_HOOKS_DIR/cbrlm-code-discovery-gate"
SESSION_CMD="$CLAUDE_HOOKS_DIR/cbrlm-session-reminder"
echo -e "${GRAY}Configuring Claude Code hooks...${NC}"
upsert_claude_hooks "$CLAUDE_SETTINGS" "$GATE_CMD" "$SESSION_CMD"

echo ""
echo -e "${GREEN}CBRLM binary: ${INSTALLED_BIN}${NC}"
echo ""
echo -e "${GRAY}Add to agent MCP config:${NC}"
echo -e "${GRAY}  \"codebase-rlm-memory-mcp\": {"
echo -e "${GRAY}    \"command\": [\"${INSTALLED_BIN}\"],"
echo -e "${GRAY}    \"environment\": { \"CBRLM_PROJECT_PREFIX\": \"cbrlm+\" }"
echo -e "${GRAY}  }${NC}"
echo ""
echo -e "${GRAY}Hooks: SessionStart (Codex/Claude) + PreToolUse Grep|Glob augment (Claude)${NC}"
echo ""