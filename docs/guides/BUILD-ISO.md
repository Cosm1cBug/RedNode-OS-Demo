# RedNode-OS — ISO Build & Deploy Guide

> **Step-by-step: from source code to a running RedNode-OS on bare metal.**

---

## Prerequisites

You need a **build machine** (any Linux with Nix installed) and a **target machine** (your old PC + GPU where RedNode will run).

### Build Machine Requirements
- Any Linux (Ubuntu, Fedora, NixOS, or even WSL2)
- **Nix package manager** installed (not necessarily NixOS — just Nix)
- ~10 GB free disk space (for Nix store + build artifacts)
- Internet connection (to download dependencies)

### Target Machine Requirements
- x86_64 CPU, 4+ cores
- 16 GB+ RAM (32 GB recommended)
- NVIDIA GPU with 6+ GB VRAM
- SSD, 120 GB+
- Ethernet port
- USB drive (8 GB+ for the ISO)

---

## Step 1: Install Nix on Your Build Machine (5 minutes)

If you're already on NixOS, skip this.

```bash
# Install Nix (multi-user, recommended)
sh <(curl -L https://nixos.org/nix/install) --daemon

# Enable flakes
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# Verify
nix --version
# Should show: nix (Nix) 2.18+ 
```

---

## Step 2: Clone the RedNode-OS Repository

```bash
git clone https://github.com/Cosm1cBug/RedNode-OS-Demo.git
cd RedNode-OS-Demo
```

---

## Step 3: Prepare the Build (10 minutes)

### 3a. Verify Cargo.lock exists

```bash
ls core/rednode-core/Cargo.lock
# If it exists, good. If not:
cd core/rednode-core
cargo generate-lockfile
cd ../..
```

### 3b. Customize configuration.nix for YOUR hardware

Edit `os/nixos/configuration.nix`:

```bash
vim os/nixos/configuration.nix
```

**Change these lines:**

```nix
# Line ~57: Your network interface name
# Find it on the target machine with: ip link
networking.interfaces.enp0s31f6 = {    # ← change "enp0s31f6" to your interface
  useDHCP = false;
  ipv4.addresses = [{
    address = "10.0.50.10";             # ← your desired static IP on VLAN 50
    prefixLength = 24;
  }];
};

# Line ~64: Your pfSense gateway
networking.defaultGateway = {
  address = "10.0.50.1";               # ← your pfSense VLAN 50 interface IP
  interface = "enp0s31f6";             # ← same interface as above
};

# Line ~68: Your Pi-hole IP
networking.nameservers = [ "10.0.50.2" ];  # ← your Pi-hole IP

# Line ~79: Your timezone
time.timeZone = "Asia/Kolkata";         # ← your timezone

# Line ~92: Your SSH public key (optional, SSH is disabled by default)
openssh.authorizedKeys.keys = [
  # "ssh-ed25519 AAAA... your-key"
];

# Line ~97: Set your user password
# Generate hash: mkpasswd -m sha-512
initialHashedPassword = "";             # ← replace with mkpasswd output
```

### 3c. Review the flake.nix

The flake references `configuration.nix`, `hardware.nix`, and `disk-encryption.nix`. These should work for most x86_64 systems. The hardware.nix auto-detects Intel/AMD CPU and NVIDIA GPU.

---

## Step 4: Build the ISO (20-40 minutes, first time)

```bash
# Build the ISO
# First build downloads ~1.8 GB of dependencies. Subsequent builds are cached.

nix build .#iso \
  --extra-experimental-features "nix-command flakes" \
  --print-build-logs

# The ISO appears at:
ls -lh result/iso/
# → nixos-24.05-x86_64-linux.iso  (~1.2 GB)
```

### If the build fails

**Common issue: Cargo hash mismatch**

```
error: hash mismatch in fixed-output derivation
  specified: sha256-AAAA...
  got:       sha256-BBBB...
```

Fix: The Nix build needs the correct hash of your Cargo dependencies. Update `flake.nix`:

```bash
# In os/nixos/flake.nix, find the cargoLock section and ensure it points
# to your actual Cargo.lock:
cargoLock = {
  lockFile = ../../core/rednode-core/Cargo.lock;
  allowBuiltinFetchGit = true;
};
```

If still failing, use `cargoHash` instead:
```nix
cargoHash = ""; # Leave empty — Nix will tell you the correct hash in the error
```

**Common issue: Missing system libraries**

```
error: attribute 'sqlite' missing
```

Fix: Ensure `buildInputs` in flake.nix includes all dependencies:
```nix
buildInputs = with pkgs; [ openssl sqlite pkg-config ];
```

---

## Step 5: Flash the ISO to USB (2 minutes)

```bash
# Find your USB device
lsblk
# Look for your USB drive (e.g., /dev/sdb)
# ⚠️ TRIPLE CHECK — this ERASES the device

# Flash
sudo dd if=result/iso/*.iso of=/dev/sdX bs=4M status=progress conv=fsync
# Replace /dev/sdX with YOUR USB device

# Safely eject
sudo sync
sudo eject /dev/sdX
```

---

## Step 6: Boot the Target Machine from USB

1. **Plug the USB** into your target PC
2. **Enter BIOS/UEFI** (usually F2, F12, Del, or Esc during POST)
3. **Set boot order**: USB first
4. **Disable Secure Boot** (or enroll NixOS keys — advanced)
5. **Boot from USB**
6. NixOS live environment loads

---

## Step 7: Install NixOS to Disk (15 minutes)

In the live environment:

```bash
# ─── 7a. Partition the disk ───
# Find your target disk
lsblk
# Usually /dev/sda or /dev/nvme0n1

# Partition (GPT + EFI + root)
# For a simple setup (no LUKS encryption):
sudo parted /dev/sda -- mklabel gpt
sudo parted /dev/sda -- mkpart ESP fat32 1MiB 512MiB
sudo parted /dev/sda -- set 1 esp on
sudo parted /dev/sda -- mkpart primary 512MiB 100%

# For LUKS encryption (recommended):
sudo parted /dev/sda -- mklabel gpt
sudo parted /dev/sda -- mkpart ESP fat32 1MiB 512MiB
sudo parted /dev/sda -- set 1 esp on
sudo parted /dev/sda -- mkpart primary 512MiB 100%
sudo cryptsetup luksFormat /dev/sda2
# Enter your encryption passphrase (REMEMBER THIS)
sudo cryptsetup open /dev/sda2 rednode-root

# ─── 7b. Format ───
sudo mkfs.fat -F32 -n BOOT /dev/sda1

# Without LUKS:
sudo mkfs.ext4 -L rednode /dev/sda2

# With LUKS:
sudo mkfs.ext4 -L rednode /dev/mapper/rednode-root

# ─── 7c. Mount ───
# Without LUKS:
sudo mount /dev/sda2 /mnt

# With LUKS:
sudo mount /dev/mapper/rednode-root /mnt

sudo mkdir -p /mnt/boot
sudo mount /dev/sda1 /mnt/boot

# ─── 7d. Generate hardware config ───
sudo nixos-generate-config --root /mnt

# This creates /mnt/etc/nixos/hardware-configuration.nix
# with your actual disk UUIDs and hardware

# ─── 7e. Copy RedNode config ───
# Copy our configuration files
sudo cp /path/to/RedNode-OS-Demo/os/nixos/configuration.nix /mnt/etc/nixos/configuration.nix
# Keep the generated hardware-configuration.nix as-is

# IMPORTANT: Edit configuration.nix to import the generated hardware config:
sudo nano /mnt/etc/nixos/configuration.nix
# Change the imports line to:
#   imports = [ ./hardware-configuration.nix ];
# (instead of ./hardware.nix + ./disk-encryption.nix)

# If using LUKS, add to configuration.nix:
#   boot.initrd.luks.devices."rednode-root" = {
#     device = "/dev/disk/by-uuid/YOUR-UUID-HERE";
#   };
# Get UUID: sudo blkid /dev/sda2

# ─── 7f. Set user password ───
# Generate password hash:
mkpasswd -m sha-512
# Paste the hash into configuration.nix → initialHashedPassword

# ─── 7g. Install ───
sudo nixos-install

# Set root password when prompted
# Reboot
sudo reboot
# Remove USB drive during reboot
```

---

## Step 8: First Boot — RedNode Comes Alive (5 minutes)

After reboot, log in with your user account.

```bash
# ─── 8a. Verify GPU ───
nvidia-smi
# Should show your GPU

# ─── 8b. Pull LLM models ───
ollama pull qwen2.5:14b-instruct-q4_K_M   # or 7b for 6GB GPU
ollama pull nomic-embed-text

# Verify:
ollama list
ollama run qwen2.5:14b "Hello, are you working?"

# ─── 8c. Clone RedNode repo ───
cd ~
git clone https://github.com/Cosm1cBug/RedNode-OS-Demo.git
cd RedNode-OS-Demo

# ─── 8d. Start infrastructure ───
cd deployment
docker compose up -d
cd ..

# Wait 10 seconds for services to initialize
sleep 10

# Verify:
docker ps  # Should show: nats, postgres, qdrant, mosquitto, loki, prometheus, grafana

# ─── 8e. Build and start CNS ───
cd core/rednode-core
cargo build --release   # First build: 3-5 minutes
cargo run --release &
cd ../..

# Wait for CNS to start
sleep 5

# Test:
curl http://localhost:8787/health
# → {"ok":true,"node":"rednode-cns","version":"0.3.1",...}

# ─── 8f. Start agents ───
pnpm install
pnpm agents &

# ─── 8g. Start web dashboard ───
pnpm web &

# ─── 8h. Verify everything ───
curl http://localhost:8787/sentience | python3 -m json.tool
# Should show real drives, resources, agents

# Open in browser:
echo "Dashboard: http://$(hostname -I | awk '{print $1}'):3000"
```

---

## Step 9: Post-Install — Set Up Your Environment

```bash
# Copy .env.example and fill in your values
cp .env.example .env
vim .env

# Key variables to set:
#   PIHOLE_URL=http://10.0.50.2
#   PIHOLE_PASSWORD=your-password
#   TRUENAS_URL=https://10.0.50.3
#   TRUENAS_API_KEY=your-key
#   REDNODE_API_TOKEN=rn_$(openssl rand -hex 32)

# Source it
source .env

# Restart with environment
./scripts/start-all.sh restart

# ─── Set up Frigate (if cameras ready) ───
# Edit deployment/frigate.yml with your NVR IP and credentials
# Then: docker compose up -d frigate
```

---

## Step 10: Use the startup script going forward

```bash
# Start everything (Docker + CNS + all agents + web)
./scripts/start-all.sh

# Check status
./scripts/start-all.sh status

# Stop everything
./scripts/start-all.sh stop

# Restart
./scripts/start-all.sh restart
```

---

## Quick Reference After Deploy

```bash
# Submit an intent
rednode intent "check system health and show DNS stats"

# Full system status
rednode status

# Run workflows
rednode goodnight
rednode morning
rednode focus

# Infrastructure
rednode cameras
rednode nas
rednode pihole

# Export your brain (backup)
./scripts/rednode-export.sh

# Import on new machine
./scripts/rednode-import.sh backup.rednode.age
```

---

## Troubleshooting

| Problem | Solution |
|---|---|
| `cargo build` fails with OpenSSL error | `nix-shell -p openssl pkg-config` or ensure they're in configuration.nix |
| Ollama slow / no GPU | Check `nvidia-smi`. Ensure `services.ollama.acceleration = "cuda"` in config |
| NATS connection refused | `docker compose up -d nats` and wait 5s |
| Pi-hole agent can't connect | Check PIHOLE_URL and PIHOLE_PASSWORD in .env |
| TrueNAS API 401 | Create API key in TrueNAS UI → Settings → API Keys |
| Cameras not showing | Check Frigate config, NVR RTSP URL, cross-VLAN firewall rules |
| Dashboard blank | Check `pnpm web` is running, browser console for errors |
| Voice "model not found" | `ollama pull qwen2.5:14b-instruct-q4_K_M` |
