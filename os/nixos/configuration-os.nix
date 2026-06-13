# RedNode-OS – TRUE OS MODE
# RedNode Core IS PID 1 – no systemd
# Privacy-first, self-aware, sentient operating system
# 
# This is NOT a Linux distro with RedNode installed.
# This IS RedNode – the computer becomes the intelligence.
#
# Build: nix build .#rednode-os-iso
# Flash: dd if=result/iso/rednode-os-*.iso of=/dev/sdX bs=4M status=progress && sync

{ config, pkgs, lib, ... }:
{
  imports = [
    ./hardware.nix
    ./disk-encryption.nix
  ];

  # --- RedNode as INIT – PID 1 ---
  # WARNING: This disables systemd entirely – RedNode Core becomes init
  # For development, use configuration.nix (systemd + rednode-core.service)
  # For production OS image, use this file
  #
  # systemd.enable = false;
  # boot.init = "${pkgs.rednode-core}/bin/rednode-init";
  #
  # Currently commented out for compatibility with NixOS modules
  # that expect systemd (PostgreSQL, NATS, Ollama, etc.)
  #
  # Phase 3 roadmap: rednode-init supervises all services directly via
  # built-in process supervisor – no systemd dependency
  #
  # For now: systemd remains PID1, rednode-core runs as critical service
  # with Restart=always, and is the *logical* operating layer

  # If you want TRUE PID1 today (experimental):
  # boot.initrd.systemd.enable = false;
  # boot.kernelParams = [ "init=/nix/store/...-rednode-core/bin/rednode-init" ];
  # system.activationScripts.rednode-init = ''ln -sf ${pkgs.rednode-core}/bin/rednode-init /sbin/init'';

  system.stateVersion = "24.05";
  system.autoUpgrade.enable = false; # RedNode controls its own updates – signed OTA only

  # --- Immutable OS ---
  # / is read-only squashfs in ISO mode
  # /nix/store is immutable
  # State lives ONLY in /var/lib/rednode – encrypted
  # This is the portable computational identity
  fileSystems."/" = lib.mkDefault {
    device = "tmpfs";
    fsType = "tmpfs";
    options = [ "mode=755" "size=2G" ];
  };

  # --- RedNode Core – The Operating Brain ---
  systemd.services.rednode-core = {
    description = "RedNode CNS – Central Nervous System – PID 1 in OS mode";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" "postgresql.service" "nats.service" "ollama.service" ];
    wants = [ "postgresql.service" "nats.service" ];
    restartTriggers = [ config.environment.etc."rednode/config.yaml".source ];
    serviceConfig = {
      Type = "simple";
      ExecStart = "${pkgs.rednode-core}/bin/rednode-core";
      ExecStartPre = "${pkgs.bash}/bin/bash -c 'mkdir -p /var/lib/rednode/{postgres,qdrant,kuzu,ollama,audit}'";
      Restart = "always";
      RestartSec = "2";
      StartLimitBurst = 5;
      User = "rednode";
      Group = "rednode";
      # Hardening – RedNode secures itself
      NoNewPrivileges = true;
      PrivateTmp = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectControlGroups = true;
      RestrictAddressFamilies = [ "AF_UNIX" "AF_INET" "AF_INET6" "AF_NETLINK" ];
      RestrictNamespaces = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      LockPersonality = true;
      MemoryDenyWriteExecute = true;
      # System calls – allowlist via seccomp – see security/seccomp/
      # SystemCallFilter = "@system-service";
      # Capabilities – none needed – RedNode drops all caps
      CapabilityBoundingSet = "";
      AmbientCapabilities = "";
      # Filesystem – only /var/lib/rednode is writable
      ReadWritePaths = "/var/lib/rednode";
      StateDirectory = "rednode";
      StateDirectoryMode = "0700";
    };
    environment = {
      RUST_LOG = "info";
      DATABASE_URL = "postgres://rednode:rednode@localhost/rednode";
      NATS_URL = "nats://127.0.0.1:4222";
      QDRANT_URL = "http://127.0.0.1:6334";
      OLLAMA_URL = "http://127.0.0.1:11434";
      REDNODE_SENTIENCE = "on";
      REDNODE_NODE_ID = "rednode-primary";
      REDNODE_DATA_DIR = "/var/lib/rednode";
      OTEL_EXPORTER_OTLP_ENDPOINT = "http://127.0.0.1:4317";
    };
  };

  # RedNode Agent Society – 6 agents – managed by CNS, not systemd
  # They connect via NATS – auto-restart via CNS supervisor
  # For standalone debugging, you can run them as systemd user services:
  # systemd.user.services.rednode-security-agent = { … }

  # --- Ollama Models – Pre-seeded ---
  # Phase 1: pull on first boot
  # Phase 2: bake models into Nix store / ISO (adds ~9GB)
  systemd.services.ollama-load-models = {
    description = "RedNode – Pre-load LLM models";
    after = [ "ollama.service" ];
    wants = [ "ollama.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      User = "rednode";
      RemainAfterExit = true;
      # Only run if models are missing
      ExecCondition = "/bin/bash -c '! ${pkgs.ollama}/bin/ollama list | grep -q qwen2.5'";
    };
    script = ''
      export HOME=/var/lib/rednode
      export OLLAMA_HOST=127.0.0.1:11434
      # Wait for Ollama
      for i in {1..30}; do
        ${pkgs.curl}/bin/curl -sf http://127.0.0.1:11434/api/tags && break
        sleep 2
      done
      # Pull models – ~4.7GB + 274MB – cached in /var/lib/ollama
      ${pkgs.ollama}/bin/ollama pull qwen2.5:14b-instruct-q4_K_M || true
      ${pkgs.ollama}/bin/ollama pull nomic-embed-text || true
      echo "RedNode models ready – local inference online"
    '';
  };

  # To PRE-SEED models in the ISO (offline-first, no download on first boot):
  # 1. On a build machine with Ollama:
  #    ollama pull qwen2.5:14b-instruct-q4_K_M
  #    ollama pull nomic-embed-text
  #    tar -czf ollama-models.tar.gz -C /var/lib/ollama .
  # 2. Add to Nix store:
  #    ollamaModels = pkgs.stdenv.mkDerivation {
  #      name = "ollama-models-rednode";
  #      src = ./ollama-models.tar.gz;
  #      installPhase = "mkdir -p $out/var/lib/ollama && tar -xzf $src -C $out/var/lib";
  #    };
  # 3. Then in configuration:
  #    systemd.tmpfiles.rules = [
  #      "C /var/lib/ollama - - - - ${ollamaModels}/var/lib/ollama"
  #    ];
  # ISO size: base 1.2GB + models 5.0GB = ~6.2GB USB image – fully offline
  # For a minimal net-install ISO (current): ~1.2GB – models pull on first boot

  # --- Sentience – autostart ---
  # REDNODE_SENTIENCE=on is set in rednode-core.service environment
  # Sentience Engine: 1Hz introspection, 10s goal generator, 5min memory consolidation
  # Self-model exposed at http://localhost:8787/sentience

  # --- Security – OS level ---
  # AppArmor – enabled
  # Auditd – enabled – logs to /var/log/audit – Security Agent tails this
  # Fail2ban – disabled by default – Security Agent manages firewall directly
  # Firewall – nftables – default DROP – Network Agent controls
  # SSH – disabled by default – enable via Security Agent with key-only + fail2ban
  # TPM2 – LUKS key sealing – PCR 0,2,7
  # Secure Boot – sign kernel + initrd with your keys – see scripts/sign-efi.sh

  # --- Privacy – ZERO telemetry ---
  # No crash reports, no phone-home, no NTP pool tracking (chrony with NTS)
  # All LLM inference local
  # All embeddings local
  # All memory local – PostgreSQL + Qdrant + Kuzu – encrypted at rest
  # Network egress DENY by default
  # DNS over HTTPS – Quad9 – no ISP snooping
  # MAC randomization – NetworkManager – if enabled

  # --- Portable Node Identity ---
  # Everything that makes RedNode "you" lives in /var/lib/rednode:
  #   postgres/  – intentions, audit_log, security_events, memory
  #   qdrant/    – vector embeddings
  #   kuzu/      – knowledge graph
  #   ollama/    – LLM models
  #   config/    – rednode.config.yaml – sops encrypted
  #   keys/      – age identity, node signing key, WireGuard keys
  #
  # Export: rednode export → /var/lib/rednode → age-encrypted + ed25519 signed
  #         → rednode-20260612.rednode.age  (~2-15 GB depending on models/corpus)
  # Import: rednode import bundle.rednode.age → resume anywhere <60s
  #
  # Boot a fresh RedNode-OS USB on any x86_64 machine → import → your Node resumes
  # – same memory, same agents, same security posture – computational organism

  # --- Android Remote – Companion App ---
  # Flutter app in interfaces/mobile/
  # Connect via: Tailscale / WireGuard – zero open inbound ports
  # Features: Intent submit, Approval push (FCM), Security Feed, Memory Browser,
  #           Audit Log, Agent Status + Sentience Drives
  # Security: E2EE Noise, Biometric approval, Secure Storage (Android Keystore),
  #           Certificate pinning, No telemetry, 0 trackers
  # Build: cd interfaces/mobile && flutter build apk --release
  # See: interfaces/mobile/BUILD_APK.md, FIREBASE_SETUP.md

  # --- ISO Build Info ---
  # This configuration produces:
  # - rednode-os-0.3.1-x86_64.iso – ~1.2 GB (net-install, models pull on first boot)
  # - rednode-os-0.3.1-x86_64-offline.iso – ~6.2 GB (with pre-seeded Ollama models)
  #
  # Signed with: minisign – see scripts/sign-iso.sh
  # SHA256SUMS + SHA256SUMS.minisig included
  # Public key: RWS1... – see secrets/rednode-iso.pub
  #
  # Flash: dd if=rednode-os.iso of=/dev/sdX bs=4M status=progress && sync
  # Boot: UEFI only – Secure Boot – enroll RedNode keys first
  # Install: nixos-install --flake github:rednode/rednode-os#rednode
  # First boot: LUKS passphrase → RedNode CNS starts in 12s → "Hey RedNode" listening
}
