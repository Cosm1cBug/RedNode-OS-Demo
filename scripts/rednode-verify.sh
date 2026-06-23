#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════
# RedNode-OS — Pre-Push Verification & Lint/Format Script
#
# Usage:
#   ./scripts/rednode-verify.sh              # run all 10 checks
#   ./scripts/rednode-verify.sh --fix        # auto-fix formatting + linting
#   ./scripts/rednode-verify.sh --check 3    # run only check #3
#
# Checks:
#   1.  Version sync (8 files)
#   2.  External references (zero tolerance)
#   3.  Agent scaffolding (package.json + tsconfig.json + src/index.ts)
#   4.  Security audit (unsafe, SQL injection, hardcoded secrets)
#   5.  Tool registry (valid JSON, required fields)
#   6.  Syntax checks (bash, python, JSON)
#   7.  pnpm workspace coverage
#   8.  start-all.sh agent coverage
#   9.  File counts & LOC
#   10. .gitignore coverage
#
# Format & Lint (--fix):
#   - Rust:       cargo fmt + cargo clippy
#   - TypeScript: npx prettier --write + npx eslint --fix
#   - Nix:        nixfmt (if installed)
#   - Shell:      shfmt (if installed)
#   - Python:     ruff format + ruff check --fix (if installed)
#   - JSON:       python3 -m json.tool (reformat)
# ═══════════════════════════════════════════════════════════

set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

PASS=true
TOTAL=0
PASSED=0

pass() { TOTAL=$((TOTAL+1)); PASSED=$((PASSED+1)); printf "   ${GREEN}✅${NC} %s\n" "$1"; }
fail() { TOTAL=$((TOTAL+1)); PASS=false; printf "   ${RED}❌${NC} %s\n" "$1"; }
warn() { printf "   ${YELLOW}⚠️${NC}  %s\n" "$1"; }
info() { printf "   ${BLUE}ℹ${NC}  %s\n" "$1"; }

# ─── Parse args ───
MODE="verify"
CHECK_NUM=""
for arg in "$@"; do
  case "$arg" in
    --fix) MODE="fix" ;;
    --check) MODE="single" ;;
    [0-9]*) CHECK_NUM="$arg" ;;
  esac
done

should_run() {
  [ "$MODE" != "single" ] || [ "$CHECK_NUM" = "$1" ]
}

# ═══════════════════════════════════════════════════════════
# CHECK 1: Version Sync
# ═══════════════════════════════════════════════════════════
if should_run 1; then
echo ""
echo -e "${BOLD}1. VERSION SYNC${NC}"

EXPECTED_VER=$(grep '"version"' package.json | head -1 | grep -oP '\d+\.\d+\.\d+')
info "Expected version: $EXPECTED_VER"

check_ver() {
  local file="$1" actual="$2"
  if echo "$actual" | grep -q "$EXPECTED_VER"; then
    pass "$file → $EXPECTED_VER"
  else
    fail "$file → $actual (expected $EXPECTED_VER)"
  fi
}

check_ver "package.json" "$(grep '"version"' package.json | grep -oP '\d+\.\d+\.\d+')"
check_ver "Cargo.toml" "$(grep '^version' core/rednode-core/Cargo.toml | grep -oP '\d+\.\d+\.\d+')"
check_ver "api.rs" "$(grep -oP '"(\d+\.\d+\.\d+)"' core/rednode-core/src/api.rs | head -1 | tr -d '"')"
check_ver "flake.nix" "$(grep 'version = "' os/nixos/flake.nix | head -1 | grep -oP '\d+\.\d+\.\d+')"
check_ver "selfheal.sh" "$(grep -oP 'v\d+\.\d+\.\d+' scripts/rednode-selfheal.sh | head -1 | tr -d 'v')"
check_ver "setup-first-boot.sh" "$(grep -oP 'v\d+\.\d+\.\d+' scripts/setup-first-boot.sh | head -1 | tr -d 'v')"
check_ver ".env.example" "$(grep -oP 'v\d+\.\d+\.\d+' .env.example | head -1 | tr -d 'v')"
check_ver "kiosk.nix" "$(grep -oP '\d+\.\d+\.\d+' os/nixos/kiosk.nix | head -1)"
fi

# ═══════════════════════════════════════════════════════════
# CHECK 2: External References
# ═══════════════════════════════════════════════════════════
if should_run 2; then
echo ""
echo -e "${BOLD}2. EXTERNAL REFERENCES${NC}"

hits=$(grep -rn "\bECC\b\|ruflo\|CogKernel\|Tencent\|\brobin\b\|affaan" \
  --include="*.rs" --include="*.ts" --include="*.nix" \
  --include="*.md" --include="*.json" --include="*.py" . 2>/dev/null \
  | grep -v node_modules | grep -v target | grep -v ".git/" \
  | grep -v "robin_hood\|ROBIN\|Robin Hood" \
  | grep -v "rednode-verify.sh" || true)

if [ -z "$hits" ]; then
  pass "Zero external references found"
else
  fail "External references found:"
  echo "$hits" | sed 's/^/      /'
fi
fi

# ═══════════════════════════════════════════════════════════
# CHECK 3: Agent Scaffolding
# ═══════════════════════════════════════════════════════════
if should_run 3; then
echo ""
echo -e "${BOLD}3. AGENT SCAFFOLDING${NC}"

for d in agents/*/; do
  name=$(basename "$d")
  [ "$name" = "shared" ] && continue
  missing=""
  [ ! -f "$d/package.json" ] && missing="$missing package.json"
  [ ! -f "$d/tsconfig.json" ] && missing="$missing tsconfig.json"
  [ ! -f "$d/src/index.ts" ] && missing="$missing src/index.ts"
  if [ -z "$missing" ]; then
    pass "$name"
  else
    fail "$name — MISSING:$missing"
  fi
done
fi

# ═══════════════════════════════════════════════════════════
# CHECK 4: Security Audit
# ═══════════════════════════════════════════════════════════
if should_run 4; then
echo ""
echo -e "${BOLD}4. SECURITY AUDIT${NC}"

unsafe=$(grep -rn "unsafe" core/rednode-core/src/ --include="*.rs" | grep -v "test\|//" | wc -l)
sqli=$(grep -rn 'format!.*\(SELECT\|INSERT\|DELETE\|UPDATE\)' core/rednode-core/src/ --include="*.rs" | grep -v "test" | wc -l)
secrets=$(grep -rn 'password\s*=\s*"[^"]*"' core/rednode-core/src/ --include="*.rs" | grep -v "test\|//" | wc -l)
unwraps=$(grep -rn '\.unwrap()' core/rednode-core/src/ --include="*.rs" | wc -l)

[ "$unsafe" -le 2 ] && pass "Unsafe Rust: $unsafe (init.rs FFI only)" || fail "Unsafe Rust: $unsafe (expected ≤2)"
[ "$sqli" -eq 0 ] && pass "SQL injection: $sqli" || fail "SQL injection: $sqli — use parameterized queries"
[ "$secrets" -eq 0 ] && pass "Hardcoded secrets: $secrets" || fail "Hardcoded secrets: $secrets"
info ".unwrap() count: $unwraps (review each — prefer .expect() or ?)"
fi

# ═══════════════════════════════════════════════════════════
# CHECK 5: Tool Registry
# ═══════════════════════════════════════════════════════════
if should_run 5; then
echo ""
echo -e "${BOLD}5. TOOL REGISTRY${NC}"

python3 -c "
import json, sys
tools = json.load(open('execution/tool-registry/tools.json'))
errors = []
names = set()
for i, t in enumerate(tools):
    for field in ['name', 'agent', 'risk']:
        if field not in t:
            errors.append(f'Tool #{i}: missing \"{field}\"')
    risk = t.get('risk', '')
    if risk not in ('low', 'medium', 'high', 'critical'):
        errors.append(f'{t.get(\"name\",\"?\")}: invalid risk=\"{risk}\"')
    name = t.get('name','')
    if name in names:
        errors.append(f'{name}: DUPLICATE tool name')
    names.add(name)

from collections import Counter
agents = Counter(t['agent'] for t in tools)

if errors:
    for e in errors: print(f'   ❌ {e}')
    sys.exit(1)
else:
    print(f'   ✅ {len(tools)} tools — all have name + agent + risk')
    print(f'   ✅ {len(agents)} agent types — no duplicates')
    print(f'   ✅ Risk breakdown: {sum(1 for t in tools if t[\"risk\"]==\"low\")} low, {sum(1 for t in tools if t[\"risk\"]==\"medium\")} medium, {sum(1 for t in tools if t[\"risk\"]==\"high\")} high, {sum(1 for t in tools if t[\"risk\"]==\"critical\")} critical')
" 2>&1
result=$?
[ $result -eq 0 ] && PASSED=$((PASSED+3)) && TOTAL=$((TOTAL+3)) || { PASS=false; TOTAL=$((TOTAL+1)); }
fi

# ═══════════════════════════════════════════════════════════
# CHECK 6: Syntax Checks
# ═══════════════════════════════════════════════════════════
if should_run 6; then
echo ""
echo -e "${BOLD}6. SYNTAX CHECKS${NC}"

# Bash scripts
for f in scripts/*.sh; do
  bash -n "$f" 2>/dev/null && pass "$(basename $f) (bash)" || fail "$(basename $f) — bash syntax error"
done

# Python
for f in $(find scripts/ -name "*.py" 2>/dev/null); do
  python3 -c "import ast; ast.parse(open('$f').read())" 2>/dev/null && pass "$(basename $f) (python)" || fail "$(basename $f) — python syntax error"
done

# JSON files
for f in execution/tool-registry/tools.json agents/*/package.json; do
  [ -f "$f" ] || continue
  python3 -c "import json; json.load(open('$f'))" 2>/dev/null && pass "$(echo $f | sed 's|agents/||;s|/package.json| pkg.json|') (json)" || fail "$f — invalid JSON"
done
fi

# ═══════════════════════════════════════════════════════════
# CHECK 7: pnpm Workspace
# ═══════════════════════════════════════════════════════════
if should_run 7; then
echo ""
echo -e "${BOLD}7. PNPM WORKSPACE${NC}"

if grep -q "agents/\*" pnpm-workspace.yaml 2>/dev/null; then
  pass "pnpm-workspace.yaml has agents/* glob"
else
  fail "learning-agent not covered by pnpm workspace"
fi

# Check root package.json workspaces too
if grep -q '"agents/\*"' package.json 2>/dev/null; then
  pass "package.json workspaces has agents/*"
else
  warn "package.json workspaces may not cover agents/*"
fi
fi

# ═══════════════════════════════════════════════════════════
# CHECK 8: start-all.sh Coverage
# ═══════════════════════════════════════════════════════════
if should_run 8; then
echo ""
echo -e "${BOLD}8. START-ALL.SH COVERAGE${NC}"

for agent in system-agent security-agent coding-agent research-agent automation-agent \
             network-agent infra-agent storage-agent surveillance-agent comms-agent learning-agent; do
  if grep -q "$agent" scripts/start-all.sh; then
    pass "$agent"
  else
    fail "$agent NOT in start-all.sh"
  fi
done
fi

# ═══════════════════════════════════════════════════════════
# CHECK 9: File Counts
# ═══════════════════════════════════════════════════════════
if should_run 9; then
echo ""
echo -e "${BOLD}9. FILE COUNTS${NC}"

files=$(find . -type f ! -path "*/node_modules/*" ! -path "*/target/*" ! -path "*/.git/*" ! -name "*.lock" | wc -l)
loc=$(find . -type f \( -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "*.nix" -o -name "*.sh" -o -name "*.py" \) ! -path "*/node_modules/*" ! -path "*/target/*" ! -path "*/.git/*" -exec cat {} + | wc -l)
rust_mods=$(find core/rednode-core/src/ -name "*.rs" | wc -l)
agent_dirs=$(find agents/ -maxdepth 1 -type d ! -name shared | tail -n+2 | wc -l)
tools=$(python3 -c "import json; print(len(json.load(open('execution/tool-registry/tools.json'))))" 2>/dev/null)
nix_mods=$(find os/nixos/ -name "*.nix" | wc -l)
docs=$(find . -name "*.md" ! -path "*node_modules*" ! -path "*/.git/*" | wc -l)
env_vars=$(grep -c "^[A-Z]" .env.example 2>/dev/null || echo "?")

info "Files:         $files"
info "Source LOC:    $loc"
info "Rust modules:  $rust_mods"
info "Agents:        $agent_dirs"
info "Tools:         $tools"
info "NixOS modules: $nix_mods"
info "Docs:          $docs"
info "Env vars:      $env_vars"
TOTAL=$((TOTAL+1)); PASSED=$((PASSED+1))
fi

# ═══════════════════════════════════════════════════════════
# CHECK 10: .gitignore
# ═══════════════════════════════════════════════════════════
if should_run 10; then
echo ""
echo -e "${BOLD}10. .GITIGNORE${NC}"

for pattern in "node_modules" "target/" ".env" "__pycache__" "*.age" "age.key"; do
  if grep -qF "$pattern" .gitignore 2>/dev/null; then
    pass "$pattern"
  else
    warn "$pattern not in .gitignore"
  fi
done
fi

# ═══════════════════════════════════════════════════════════
# FORMAT & LINT (only with --fix)
# ═══════════════════════════════════════════════════════════
if [ "$MODE" = "fix" ]; then
echo ""
echo -e "${BOLD}═══ FORMAT & LINT ═══${NC}"
echo ""

# ── Rust ──
echo -e "${BOLD}Rust:${NC}"
if command -v cargo >/dev/null 2>&1; then
  echo "  Running cargo fmt..."
  cd core/rednode-core
  cargo fmt 2>&1 | sed 's/^/    /'
  FMT_EXIT=$?
  [ $FMT_EXIT -eq 0 ] && echo -e "    ${GREEN}✅ cargo fmt done${NC}" || echo -e "    ${RED}❌ cargo fmt failed${NC}"

  echo "  Running cargo clippy..."
  cargo clippy --release -- -W clippy::all 2>&1 | tail -5 | sed 's/^/    /'
  CLIPPY_EXIT=$?
  [ $CLIPPY_EXIT -eq 0 ] && echo -e "    ${GREEN}✅ cargo clippy clean${NC}" || echo -e "    ${YELLOW}⚠️  cargo clippy has warnings${NC}"
  cd "$ROOT"
else
  echo -e "    ${YELLOW}⚠️  cargo not found — install Rust toolchain${NC}"
  echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi
echo ""

# ── TypeScript ──
echo -e "${BOLD}TypeScript:${NC}"
if command -v npx >/dev/null 2>&1; then
  echo "  Running prettier..."
  npx prettier --write "agents/*/src/**/*.ts" --log-level warn 2>&1 | sed 's/^/    /'
  echo -e "    ${GREEN}✅ prettier done${NC}"

  # ESLint (optional — only if config exists)
  if [ -f ".eslintrc.json" ] || [ -f "eslint.config.js" ]; then
    echo "  Running eslint --fix..."
    npx eslint --fix "agents/*/src/**/*.ts" 2>&1 | tail -5 | sed 's/^/    /'
  else
    echo -e "    ${BLUE}ℹ${NC}  No eslint config found — skipping (prettier handles formatting)"
  fi
else
  echo -e "    ${YELLOW}⚠️  npx not found — install Node.js${NC}"
  echo "    nix-shell -p nodejs_22 nodePackages.pnpm"
fi
echo ""

# ── Nix ──
echo -e "${BOLD}Nix:${NC}"
if command -v nixfmt >/dev/null 2>&1; then
  echo "  Running nixfmt..."
  nixfmt os/nixos/*.nix 2>&1 | sed 's/^/    /'
  echo -e "    ${GREEN}✅ nixfmt done${NC}"
elif command -v nixpkgs-fmt >/dev/null 2>&1; then
  echo "  Running nixpkgs-fmt..."
  nixpkgs-fmt os/nixos/*.nix 2>&1 | sed 's/^/    /'
  echo -e "    ${GREEN}✅ nixpkgs-fmt done${NC}"
else
  echo -e "    ${YELLOW}⚠️  nixfmt not found — install with: nix-env -i nixfmt${NC}"
  echo "    Or: nix-shell -p nixfmt"
fi
echo ""

# ── Shell ──
echo -e "${BOLD}Shell:${NC}"
if command -v shfmt >/dev/null 2>&1; then
  echo "  Running shfmt..."
  shfmt -w -i 2 -ci scripts/*.sh 2>&1 | sed 's/^/    /'
  echo -e "    ${GREEN}✅ shfmt done${NC}"
else
  echo -e "    ${YELLOW}⚠️  shfmt not found — install with: nix-env -i shfmt${NC}"
  echo "    Or: nix-shell -p shfmt"
  echo "    Skipping — shell scripts are still valid (checked by bash -n)"
fi
echo ""

# ── Python ──
echo -e "${BOLD}Python:${NC}"
if command -v ruff >/dev/null 2>&1; then
  echo "  Running ruff format..."
  ruff format scripts/finetune/*.py 2>&1 | sed 's/^/    /'
  echo "  Running ruff check --fix..."
  ruff check --fix scripts/finetune/*.py 2>&1 | sed 's/^/    /'
  echo -e "    ${GREEN}✅ ruff done${NC}"
elif command -v black >/dev/null 2>&1; then
  echo "  Running black..."
  black scripts/finetune/*.py 2>&1 | sed 's/^/    /'
  echo -e "    ${GREEN}✅ black done${NC}"
else
  echo -e "    ${YELLOW}⚠️  ruff/black not found — install with: pip install ruff${NC}"
  echo "    Or: nix-shell -p ruff"
  echo "    Skipping — Python files still have valid syntax (checked by ast.parse)"
fi
echo ""

# ── JSON ──
echo -e "${BOLD}JSON:${NC}"
echo "  Reformatting tools.json..."
python3 -c "
import json
with open('execution/tool-registry/tools.json') as f:
    data = json.load(f)
with open('execution/tool-registry/tools.json', 'w') as f:
    json.dump(data, f, indent=2)
    f.write('\n')
" 2>&1
echo -e "    ${GREEN}✅ tools.json reformatted${NC}"

echo "  Checking agent package.json files..."
for f in agents/*/package.json; do
  python3 -c "
import json
with open('$f') as fh: data = json.load(fh)
with open('$f', 'w') as fh: json.dump(data, fh, indent=2); fh.write('\n')
" 2>/dev/null
done
echo -e "    ${GREEN}✅ All package.json files reformatted${NC}"
echo ""

echo -e "${BOLD}═══ FORMAT & LINT COMPLETE ═══${NC}"
echo ""
echo "Tools used:"
echo "  Rust:       cargo fmt + cargo clippy"
echo "  TypeScript: prettier (+ eslint if config exists)"
echo "  Nix:        nixfmt or nixpkgs-fmt"
echo "  Shell:      shfmt"
echo "  Python:     ruff (or black)"
echo "  JSON:       python3 json.dump(indent=2)"
echo ""
echo "Install all formatters at once (NixOS):"
echo "  nix-shell -p rustc cargo clippy rustfmt nodejs_22 nodePackages.pnpm nixfmt shfmt ruff"
echo ""
fi

# ═══════════════════════════════════════════════════════════
# RESULT
# ═══════════════════════════════════════════════════════════
if [ "$MODE" != "fix" ]; then
echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
if $PASS; then
  echo -e "  ${GREEN}${BOLD}✅ ALL CHECKS PASSED ($PASSED/$TOTAL) — SAFE TO PUSH${NC}"
else
  echo -e "  ${RED}${BOLD}❌ ISSUES FOUND — FIX BEFORE PUSHING${NC}"
  echo -e "  Run: ${BOLD}./scripts/rednode-verify.sh --fix${NC} to auto-format"
fi
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo ""
fi
