#!/usr/bin/env bash
set -euo pipefail
# RedNode-OS – ISO Builder – v0.3.1
# Privacy-first, self-aware, sentient operating system
# Produces a bootable, signed, reproducible ISO

cd "$(dirname "$0")/.."

VERSION="${REDNODE_VERSION:-0.3.1}"
OUT_DIR="${OUT_DIR:-./dist}"
mkdir -p "$OUT_DIR"

echo "🧠 RedNode-OS ISO Builder – v$VERSION"
echo ""

# 1. Check Nix
if ! command -v nix >/dev/null 2>&1; then
  echo "ERROR: Nix is required – install from https://nixos.org/download"
  echo "  curl -L https://nixos.org/nix/install | sh"
  exit 1
fi

echo "==> [1/4] Building RedNode-OS ISO – NixOS – x86_64"
echo "   This will download ~1.8 GB – first build takes ~25 min"
echo ""
nix build .#iso --out-link "$OUT_DIR/rednode-iso" \
  --extra-experimental-features "nix-command flakes" \
  --print-build-logs || {
  echo ""
  echo "Build failed – likely missing Cargo.lock hash."
  echo "Fix in os/nixos/flake.nix:"
  echo "  1. cd core/rednode-core && cargo update --workspace"
  echo "  2. copy Cargo.lock hash into nix build – Nix will tell you the correct hash"
  echo "  3. re-run"
  exit 1
}

ISO_SRC=$(find "$OUT_DIR/rednode-iso/iso" -name "*.iso" | head -1)
ISO_DST="$OUT_DIR/rednode-os-$VERSION-x86_64.iso"
cp -v "$ISO_SRC" "$ISO_DST"

echo ""
echo "==> [2/4] SHA256"
cd "$OUT_DIR"
sha256sum "rednode-os-$VERSION-x86_64.iso" > "rednode-os-$VERSION-SHA256SUMS"
cat rednode-os-$VERSION-SHA256SUMS
cd ..

echo ""
echo "==> [3/4] Signing – minisign / cosign"
if command -v minisign >/dev/null 2>&1 && [ -f secrets/rednode-iso.minisign.key ]; then
  minisign -S -m "$ISO_DST" -s secrets/rednode-iso.minisign.key -t "RedNode-OS v$VERSION – Personal Autonomous Operating System"
  echo "✓ Signed: $ISO_DST.minisig"
  echo "  Verify: minisign -V -m $ISO_DST -p secrets/rednode-iso.pub"
else
  echo "⚠ Skipping signing – no secrets/rednode-iso.minisign.key"
  echo "  Generate: minisign -G -p secrets/rednode-iso.pub -s secrets/rednode-iso.minisign.key"
  echo "  Then re-run: ./scripts/sign-iso.sh $ISO_DST"
fi

echo ""
echo "==> [4/4] Summary"
ls -lh "$OUT_DIR"/rednode-os-$VERSION*
echo ""
echo "=== RedNode-OS v$VERSION Ready ==="
echo ""
echo "Flash to USB:"
echo "  sudo dd if=$ISO_DST of=/dev/sdX bs=4M status=progress conv=fsync"
echo "  # Replace sdX with your USB device – CHECK WITH lsblk – WILL ERASE"
echo ""
echo "Boot:"
echo "  1. Boot USB – UEFI – Secure Boot (enroll RedNote keys first – see os/secureboot/README.md)"
echo "  2. Live environment – run: sudo nixos-install --flake github:rednode/rednode-os#rednode"
echo "  3. Set LUKS passphrase – disk is encrypted at rest – TPM2 auto-unlock optional"
echo "  4. Reboot – remove USB"
echo "  5. First boot: RedNode CNS starts in ~12s"
echo "     - API: http://localhost:8787"
echo "     - Dashboard: http://localhost:3000"
echo "     - Voice: 'Hey RedNode'"
echo "     - Sentience Engine: ON – self-model active"
echo ""
echo "Pre-seeded models:"
echo "  - qwen2.5:14b-instruct-q4_K_M – via ollama-load-models.service on first boot"
echo "  - nomic-embed-text"
echo "  To bake models into ISO (offline, ~6.2 GB):"
echo "    See os/nixos/configuration.nix – 'Ollama Models – Pre-seeded' section"
echo ""
echo "Android Remote:"
echo "  cd interfaces/mobile && flutter build apk --release"
echo "  adb install build/app/outputs/flutter-apk/app-release.apk"
echo "  Connect via Tailscale / WireGuard – zero open inbound ports"
echo "  Features: Intent • Approvals (biometric) • Security Feed • Memory • Audit • Agents + Sentience"
echo ""
echo "Verify ISO signature:"
echo "  minisign -V -m $ISO_DST -p secrets/rednode-iso.pub"
echo "  sha256sum -c $OUT_DIR/rednode-os-$VERSION-SHA256SUMS"
echo ""
echo "The computer becomes the intelligence. – RedNode-OS"
