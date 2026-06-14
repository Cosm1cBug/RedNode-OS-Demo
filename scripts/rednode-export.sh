#!/usr/bin/env bash
# RedNode-OS – Export Computational Identity
# Creates an age-encrypted bundle of all RedNode state
#
# What's exported:
#   - PostgreSQL dump (intentions, audit_log, security_events, memory)
#   - Qdrant snapshot (vector embeddings)
#   - Configuration (.env, agent configs)
#   - Kuzu knowledge graph data
#   - NOT exported: Ollama models (too large, re-pull on import)
#   - NOT exported: Frigate recordings (stored on TrueNAS)
#
# Usage:
#   ./scripts/rednode-export.sh
#   ./scripts/rednode-export.sh /path/to/output.rednode.age
#
# Requires: age (encryption), pg_dump (postgres)

set -euo pipefail

REDNODE_DATA="${REDNODE_DATA_DIR:-/var/lib/rednode}"
DB_URL="${DATABASE_URL:-postgres://rednode:rednode@localhost:5432/rednode}"
AGE_KEY="${AGE_KEY_FILE:-$REDNODE_DATA/keys/rednode.age.key}"
TIMESTAMP=$(date -u +%Y%m%d-%H%M%S)
OUTPUT="${1:-rednode-export-$TIMESTAMP.rednode.age}"
TMPDIR=$(mktemp -d)

trap "rm -rf $TMPDIR" EXIT

echo "🧠 RedNode-OS – Exporting Computational Identity"
echo ""

# 1. PostgreSQL dump
echo "[1/5] Dumping PostgreSQL..."
if command -v pg_dump >/dev/null 2>&1; then
  pg_dump "$DB_URL" --format=custom --file="$TMPDIR/postgres.dump" 2>/dev/null && \
    echo "  ✅ Postgres dump: $(du -h "$TMPDIR/postgres.dump" | cut -f1)" || \
    echo "  ⚠️ Postgres dump failed — skipping (DB may not be running)"
else
  echo "  ⚠️ pg_dump not found — skipping Postgres"
fi

# 2. Qdrant snapshot
echo "[2/5] Snapshotting Qdrant..."
QDRANT_URL="${QDRANT_URL:-http://localhost:6333}"
if curl -sf "$QDRANT_URL/collections" > /dev/null 2>&1; then
  # Create snapshot via API
  SNAP=$(curl -sf -X POST "$QDRANT_URL/collections/rednode_docs/snapshots" | python3 -c "import sys,json; print(json.load(sys.stdin).get('result',{}).get('name',''))" 2>/dev/null || echo "")
  if [ -n "$SNAP" ]; then
    curl -sf "$QDRANT_URL/collections/rednode_docs/snapshots/$SNAP" -o "$TMPDIR/qdrant-snapshot.tar" 2>/dev/null
    echo "  ✅ Qdrant snapshot: $(du -h "$TMPDIR/qdrant-snapshot.tar" 2>/dev/null | cut -f1 || echo "?")"
  else
    echo "  ⚠️ Qdrant snapshot failed — skipping"
  fi
else
  echo "  ⚠️ Qdrant not reachable — skipping"
fi

# 3. Configuration
echo "[3/5] Copying configuration..."
mkdir -p "$TMPDIR/config"
cp -r "$REDNODE_DATA/config" "$TMPDIR/config/" 2>/dev/null || true
cp .env "$TMPDIR/config/.env" 2>/dev/null || true
cp .env.example "$TMPDIR/config/.env.example" 2>/dev/null || true
echo "  ✅ Configuration copied"

# 4. Kuzu graph data
echo "[4/5] Copying Kuzu knowledge graph..."
if [ -d "$REDNODE_DATA/kuzu" ]; then
  cp -r "$REDNODE_DATA/kuzu" "$TMPDIR/kuzu" 2>/dev/null || true
  echo "  ✅ Kuzu data copied"
else
  echo "  ⚠️ No Kuzu data found — skipping"
fi

# 5. Create tarball and encrypt
echo "[5/5] Creating encrypted bundle..."

# Create metadata
cat > "$TMPDIR/manifest.json" <<EOF
{
  "rednode_version": "0.3.1",
  "export_timestamp": "$(date -u --iso-8601=seconds)",
  "node_id": "${REDNODE_NODE_ID:-unknown}",
  "hostname": "$(hostname)",
  "contents": {
    "postgres_dump": $([ -f "$TMPDIR/postgres.dump" ] && echo true || echo false),
    "qdrant_snapshot": $([ -f "$TMPDIR/qdrant-snapshot.tar" ] && echo true || echo false),
    "kuzu_data": $([ -d "$TMPDIR/kuzu" ] && echo true || echo false),
    "config": true
  }
}
EOF

# Create tar
TAR_FILE="$TMPDIR/rednode-bundle.tar.zst"
tar -C "$TMPDIR" --zstd -cf "$TAR_FILE" \
  manifest.json \
  $([ -f "$TMPDIR/postgres.dump" ] && echo "postgres.dump") \
  $([ -f "$TMPDIR/qdrant-snapshot.tar" ] && echo "qdrant-snapshot.tar") \
  $([ -d "$TMPDIR/kuzu" ] && echo "kuzu/") \
  config/ 2>/dev/null

# Encrypt with age
if command -v age >/dev/null 2>&1; then
  if [ ! -f "$AGE_KEY" ]; then
    echo "  Generating new age identity key..."
    mkdir -p "$(dirname "$AGE_KEY")"
    age-keygen -o "$AGE_KEY" 2>/dev/null
    echo "  ⚠️ BACK UP THIS KEY: $AGE_KEY"
  fi
  AGE_PUBKEY=$(grep "public key:" "$AGE_KEY" | sed 's/.*: //')
  age -r "$AGE_PUBKEY" -o "$OUTPUT" "$TAR_FILE"
  echo ""
  echo "✅ Encrypted bundle: $OUTPUT ($(du -h "$OUTPUT" | cut -f1))"
  echo "   Decrypt with: age -d -i $AGE_KEY $OUTPUT | tar --zstd -xf -"
else
  # No age — save unencrypted (with warning)
  OUTPUT="${OUTPUT%.age}.tar.zst"
  cp "$TAR_FILE" "$OUTPUT"
  echo ""
  echo "⚠️ UNENCRYPTED bundle (install 'age' for encryption): $OUTPUT"
fi

echo ""
echo "Import on another machine:"
echo "  ./scripts/rednode-import.sh $OUTPUT"
echo ""
echo "The computer is portable. The identity is yours. – RedNode-OS"
