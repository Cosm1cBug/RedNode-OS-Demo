#!/usr/bin/env bash
# RedNode-OS – Import Computational Identity
# Restores a previously exported RedNode bundle
#
# Usage:
#   ./scripts/rednode-import.sh rednode-export-20260613.rednode.age
#
# Requires: age (decryption), pg_restore (postgres)

set -euo pipefail

BUNDLE="${1:-}"
if [ -z "$BUNDLE" ] || [ ! -f "$BUNDLE" ]; then
  echo "Usage: $0 <bundle.rednode.age>"
  echo ""
  echo "Restores RedNode computational identity from an export bundle."
  echo "  - PostgreSQL database (intentions, audit, security events, memory)"
  echo "  - Qdrant vector embeddings"
  echo "  - Kuzu knowledge graph"
  echo "  - Configuration"
  exit 1
fi

REDNODE_DATA="${REDNODE_DATA_DIR:-/var/lib/rednode}"
DB_URL="${DATABASE_URL:-postgres://rednode:rednode@localhost:5432/rednode}"
AGE_KEY="${AGE_KEY_FILE:-$REDNODE_DATA/keys/rednode.age.key}"
TMPDIR=$(mktemp -d)

trap "rm -rf $TMPDIR" EXIT

echo "🧠 RedNode-OS – Importing Computational Identity"
echo "   Bundle: $BUNDLE"
echo ""

# 1. Decrypt
echo "[1/5] Decrypting bundle..."
if [[ "$BUNDLE" == *.age ]]; then
  if [ ! -f "$AGE_KEY" ]; then
    echo "  ❌ Age identity key not found at $AGE_KEY"
    echo "     This key was generated during export. You need the SAME key to decrypt."
    exit 1
  fi
  age -d -i "$AGE_KEY" "$BUNDLE" | tar --zstd -xf - -C "$TMPDIR"
  echo "  ✅ Decrypted and extracted"
else
  # Unencrypted bundle
  tar --zstd -xf "$BUNDLE" -C "$TMPDIR"
  echo "  ✅ Extracted (unencrypted)"
fi

# Show manifest
if [ -f "$TMPDIR/manifest.json" ]; then
  echo ""
  echo "  Manifest:"
  cat "$TMPDIR/manifest.json" | python3 -m json.tool 2>/dev/null || cat "$TMPDIR/manifest.json"
  echo ""
fi

# 2. Restore PostgreSQL
echo "[2/5] Restoring PostgreSQL..."
if [ -f "$TMPDIR/postgres.dump" ] && command -v pg_restore >/dev/null 2>&1; then
  # Drop and recreate database
  psql "$DB_URL" -c "SELECT 1;" >/dev/null 2>&1 && {
    pg_restore --clean --if-exists --no-owner --dbname="$DB_URL" "$TMPDIR/postgres.dump" 2>/dev/null && \
      echo "  ✅ PostgreSQL restored" || \
      echo "  ⚠️ pg_restore had warnings (this is often OK — tables may already exist)"
  } || echo "  ⚠️ PostgreSQL not reachable — skipping (start it first)"
else
  echo "  ⚠️ No Postgres dump in bundle or pg_restore not found — skipping"
fi

# 3. Restore Qdrant
echo "[3/5] Restoring Qdrant..."
QDRANT_URL="${QDRANT_URL:-http://localhost:6333}"
if [ -f "$TMPDIR/qdrant-snapshot.tar" ]; then
  if curl -sf "$QDRANT_URL/collections" > /dev/null 2>&1; then
    # Upload snapshot
    curl -sf -X POST "$QDRANT_URL/collections/rednode_docs/snapshots/upload" \
      -F "snapshot=@$TMPDIR/qdrant-snapshot.tar" > /dev/null 2>&1 && \
      echo "  ✅ Qdrant snapshot restored" || \
      echo "  ⚠️ Qdrant restore failed — you may need to recreate the collection"
  else
    echo "  ⚠️ Qdrant not reachable — skipping (start it first)"
  fi
else
  echo "  ⚠️ No Qdrant snapshot in bundle — skipping"
fi

# 4. Restore Kuzu
echo "[4/5] Restoring Kuzu knowledge graph..."
if [ -d "$TMPDIR/kuzu" ]; then
  mkdir -p "$REDNODE_DATA"
  cp -r "$TMPDIR/kuzu" "$REDNODE_DATA/kuzu"
  echo "  ✅ Kuzu data restored to $REDNODE_DATA/kuzu"
else
  echo "  ⚠️ No Kuzu data in bundle — skipping"
fi

# 5. Restore config
echo "[5/5] Restoring configuration..."
if [ -d "$TMPDIR/config" ]; then
  mkdir -p "$REDNODE_DATA/config"
  cp -r "$TMPDIR/config/"* "$REDNODE_DATA/config/" 2>/dev/null || true
  if [ -f "$TMPDIR/config/.env" ]; then
    echo "  ℹ️  .env found in bundle — review and copy to project root if needed"
    echo "     diff <(cat $TMPDIR/config/.env) .env"
  fi
  echo "  ✅ Configuration restored"
else
  echo "  ⚠️ No config in bundle — skipping"
fi

echo ""
echo "✅ Import complete. Restart RedNode to use the restored data:"
echo "   ./scripts/start-all.sh restart"
echo ""
echo "The identity has been restored. RedNode resumes. – RedNode-OS"
