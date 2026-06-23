# RedNode-OS – NixOS System Configuration
# This IS the operating system. Not a layer on top.
#
# Deploy: sudo nixos-rebuild switch --flake .#rednode
# Test VM: nix build .#vm && ./result/bin/run-rednode-vm
{ config, pkgs, lib, ... }:
{
  imports = [
    ./hardware.nix
    ./disk-encryption.nix
  ];

  # ──────────────────────────────────────────────
  # Boot
  # ──────────────────────────────────────────────
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  boot.kernelPackages = pkgs.linuxPackages_6_9;
  boot.kernelParams = [
    "quiet"
    "lsm=landlock,lockdown,yama,integrity,bpf"
    "lockdown=confidentiality"
    "slab_nomerge"
    "init_on_alloc=1"
    "init_on_free=1"
    "page_alloc.shuffle=1"
    # IOMMU — not needed (no Proxmox, GPU used natively)
  ];

  # ──────────────────────────────────────────────
  # Networking – VLAN-aware for home infrastructure
  # ──────────────────────────────────────────────
  networking.hostName = "rednode";
  networking.useDHCP = false;
  networking.firewall.enable = true;
  networking.firewall.allowedTCPPorts = [
    8787   # CNS API
    3000   # Web dashboard
    5000   # Frigate UI
    3001   # Grafana
    1883   # MQTT (internal, for Frigate)
  ];
  # All other ports (NATS 4222, Postgres 5432, Qdrant 6333, Ollama 11434)
  # are localhost-only — not exposed to the network

  # Static IP on Management VLAN (VLAN 50)
  # Adjust interface name to match your hardware (run: ip link)
  networking.interfaces.enp0s31f6 = {
    useDHCP = false;
    ipv4.addresses = [{
      address = "10.0.50.10";
      prefixLength = 24;
    }];
  };
  networking.defaultGateway = {
    address = "10.0.50.1";  # pfSense VLAN 50 interface
    interface = "enp0s31f6";
  };
  networking.nameservers = [ "10.0.50.2" ];  # Pi-hole

  # NetworkManager disabled — static config, no surprises
  networking.networkmanager.enable = false;

  # DNS — use Pi-hole, fallback to Quad9
  services.resolved.enable = true;
  services.resolved.fallbackDns = [ "9.9.9.9" "149.112.112.112" ];

  # ──────────────────────────────────────────────
  # Time & Locale
  # ──────────────────────────────────────────────
  time.timeZone = "Asia/Kolkata";
  services.chrony.enable = true;
  i18n.defaultLocale = "en_US.UTF-8";
  console.keyMap = "us";

  # ──────────────────────────────────────────────
  # Users
  # ──────────────────────────────────────────────
  users.mutableUsers = false;

  users.users.rednode = {
    isSystemUser = true;
    group = "rednode";
    home = "/var/lib/rednode";
    createHome = true;
  };
  users.groups.rednode = {};

  users.users.owner = {
    isNormalUser = true;
    extraGroups = [ "wheel" "docker" "rednode" "video" "render" ];
    # Set with: mkpasswd -m sha-512
    initialHashedPassword = "";
    openssh.authorizedKeys.keys = [
      # Add your SSH public key here when Security Agent enables SSH
    ];
  };

  # ──────────────────────────────────────────────
  # Security — foundation, not feature
  # ──────────────────────────────────────────────
  security.sudo.wheelNeedsPassword = true;
  security.apparmor.enable = true;
  security.audit.enable = true;
  security.auditd.enable = true;
  security.tpm2.enable = true;
  security.tpm2.tctiEnvironment.enable = true;

  # SSH disabled by default — Security Agent can enable with hardening
  services.openssh.enable = false;

  # No telemetry — ever
  nix.settings = {
    sandbox = true;
    auto-optimise-store = true;
    experimental-features = [ "nix-command" "flakes" ];
  };

  # ──────────────────────────────────────────────
  # PostgreSQL – Structured Memory
  # ──────────────────────────────────────────────
  services.postgresql = {
    enable = true;
    package = pkgs.postgresql_16;
    ensureDatabases = [ "rednode" ];
    ensureUsers = [{
      name = "rednode";
      ensureDBOwnership = true;
    }];
    authentication = lib.mkForce ''
      local rednode rednode trust
      host  rednode rednode 127.0.0.1/32 trust
    '';
    # pgvector extension for embeddings
    extraPlugins = with pkgs.postgresql16Packages; [
      pgvector
    ];
    settings = {
      shared_preload_libraries = "vector";
      # Tuned for RedNode workload (many small reads/writes)
      shared_buffers = "256MB";
      effective_cache_size = "1GB";
      work_mem = "16MB";
    };
  };

  # ──────────────────────────────────────────────
  # NATS – Central Nervous System Bus
  # ──────────────────────────────────────────────
  services.nats = {
    enable = true;
    serverName = "rednode-nats";
    settings = {
      jetstream = {
        store_dir = "/var/lib/nats/jetstream";
        max_memory_store = 256000000;   # 256 MB
        max_file_store = 1073741824;    # 1 GB
      };
      # Bind to localhost only — agents connect locally
      host = "127.0.0.1";
      port = 4222;
      http_port = 8222;
    };
  };

  # ──────────────────────────────────────────────
  # Qdrant – Vector Memory
  # ──────────────────────────────────────────────
  virtualisation.oci-containers.containers.qdrant = {
    image = "qdrant/qdrant:v1.9";
    ports = [ "127.0.0.1:6333:6333" "127.0.0.1:6334:6334" ];
    volumes = [ "/var/lib/rednode/qdrant:/qdrant/storage" ];
    extraOptions = [ "--network=host" ];
  };

  # ──────────────────────────────────────────────
  # Ollama – Local LLM (GPU-accelerated)
  # ──────────────────────────────────────────────
  # Ollama – Local LLM
  # acceleration: "cuda" for NVIDIA, "rocm" for AMD, null for CPU-only
  # The setup-first-boot.sh script auto-detects and configures this
  services.ollama = {
    enable = true;
    acceleration = null; # auto-detected at first boot; set to "cuda" or "rocm" after detection
    host = "127.0.0.1";
    port = 11434;
  };

  # ──────────────────────────────────────────────
  # MQTT Broker – for Frigate events
  # ──────────────────────────────────────────────
  services.mosquitto = {
    enable = true;
    listeners = [{
      address = "127.0.0.1";
      port = 1883;
      users = {
        frigate = {
          acl = [ "readwrite frigate/#" ];
          password = "rednode-mqtt";
        };
        rednode = {
          acl = [ "readwrite #" ];
          password = "rednode-mqtt";
        };
      };
    }];
  };

  # ──────────────────────────────────────────────
  # Observability – OpenTelemetry → Grafana
  # ──────────────────────────────────────────────
  services.grafana = {
    enable = true;
    settings.server = {
      http_addr = "0.0.0.0";
      http_port = 3001;
    };
    settings.security.admin_password = "rednode";
  };
  services.prometheus = {
    enable = true;
    listenAddress = "127.0.0.1";
    port = 9090;
  };
  services.loki = {
    enable = true;
    configuration = {
      auth_enabled = false;
      server.http_listen_port = 3100;
      common = {
        ring.kvstore.store = "inmemory";
        replication_factor = 1;
        path_prefix = "/var/lib/loki";
      };
      schema_config.configs = [{
        from = "2024-01-01";
        store = "tsdb";
        object_store = "filesystem";
        schema = "v13";
        index = {
          prefix = "index_";
          period = "24h";
        };
      }];
      storage_config.filesystem.directory = "/var/lib/loki/chunks";
    };
  };

  # ──────────────────────────────────────────────
  # Docker – for Frigate, Qdrant, and future containers
  # ──────────────────────────────────────────────
  virtualisation.docker.enable = true;
  virtualisation.docker.daemon.settings = {
    live-restore = true;
    userland-proxy = false;
    no-new-privileges = true;
    default-runtime = "runc";
  };
  # NVIDIA Container Toolkit — for Frigate TensorRT + Ollama in Docker
  hardware.nvidia-container-toolkit.enable = true;

  # ──────────────────────────────────────────────
  # Packages — minimal, RedNode manages the rest
  # ──────────────────────────────────────────────
  environment.systemPackages = with pkgs; [
    # Core tools
    vim htop btop iotop
    git curl wget
    tmux
    jq

    # Secrets
    age sops

    # NATS tools
    natscli

    # Security tools — exposed to Security Agent only
    firejail bubblewrap
    lynis
    yara

    # Node.js — for agents
    nodejs_22
    nodePackages.pnpm

    # Python — for voice interface
    (python312.withPackages (ps: with ps; [
      fastapi uvicorn
      # faster-whisper and piper-tts installed via pip in venv
    ]))

    # Build tools (for cargo build of rednode-core)
    rustc cargo clippy rustfmt
    pkg-config openssl
  ];

  # ──────────────────────────────────────────────
  # Fonts – for dashboard
  # ──────────────────────────────────────────────
  fonts.packages = with pkgs; [ inter roboto-mono ];

  # ──────────────────────────────────────────────
  # Audio — for Voice Interface (Whisper + Piper)
  # ──────────────────────────────────────────────
  # PipeWire enabled in hardware.nix
  # Microphone + speakers needed for voice loop

  # ──────────────────────────────────────────────
  # No X11 / Wayland — RedNode is headless
  # Web UI served at http://10.0.50.10:3000
  # ──────────────────────────────────────────────

  system.stateVersion = "24.05";
}
