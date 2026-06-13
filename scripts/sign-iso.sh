#!/usr/bin/env bash
set -euo pipefail
# RedNode-OS – ISO Signing – minisign / cosign
# Privacy-first – reproducible – verifiable

ISO="${1:-}"
if [ -z "$ISO" ] || [ ! -f "$ISO" ]; then
  echo "Usage: $0 rednode-os-0.3.1-x86_64.iso"
  exit 1
fi

KEY_PRIV="secrets/rednode-iso.minisign.key"
KEY_PUB="secrets/rednode-iso.pub"

if [ ! -f "$KEY_PRIV" ]; then
  echo "Generating new RedNode ISO signing keypair…"
  mkdir -p secrets
  minisign -G -p "$KEY_PUB" -s "$KEY_PRIV" <<EOF


EOF
  echo ""
  echo "=== PUBLIC KEY – commit this to git ==="
  cat "$KEY_PUB"
  echo ""
  echo "=== PRIVATE KEY – BACK THIS UP SECURELY – DO NOT COMMIT ==="
  echo "  $KEY_PRIV"
  echo ""
fi

echo "Signing $ISO with minisign…"
minisign -S -m "$ISO" -s "$KEY_PRIV" -t "RedNode-OS – Personal Autonomous Operating System – $(basename "$ISO") – $(date -u +%Y-%m-%d)"

echo ""
echo "✓ Signed:"
ls -lh "$ISO" "$ISO.minisig"
echo ""
echo "Verify:"
echo "  minisign -V -m $ISO -p $KEY_PUB"
echo ""
echo "SHA256:"
sha256sum "$ISO"

# Also generate cosign signature – optional – for OCI / SLSA
if command -v cosign >/dev/null 2>&1 && [ -f secrets/cosign.key ]; then
  echo ""
  echo "Cosign signing…"
  COSIGN_PASSWORD="" cosign sign-blob --yes --key secrets/cosign.key --output-signature "$ISO.cosign.sig" "$ISO"
  echo "✓ Cosign: $ISO.cosign.sig"
fi

cat <<EOF

=== RedNode-OS Release Checklist ===
[ ] ISO boots in QEMU – qemu-system-x86_64 -cdrom $ISO -m 4096 -enable-kvm
[ ] Cold boot to voice ready <45s
[ ] CNS API responding – curl http://localhost:8787/health
[ ] Audit log – 0 escapes in fuzz runs
[ ] RAG precision@3 >0.82
[ ] 72h autonomous run – 0 crashes
[ ] Security Agent – detects test CVE in <5min – auto-patches
[ ] Sentience Engine – drives stable – goals generating
[ ] Android APK – biometric approval works – FCM push delivered
[ ] SHA256SUMS + minisig published
[ ] SBOM generated – syft $ISO -o spdx-json > rednode-os.spdx.json
[ ] Git tag – v0.3.1 – signed

EOF
