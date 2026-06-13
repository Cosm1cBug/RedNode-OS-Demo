# RedNode-OS – NixOS System Configuration
# This IS the operating system. Not a layer on top.
{ config, pkgs, lib, ... }:
{
  imports = [
    ./hardware.nix
    ./disk-encryption.nix
  ];

  # Boot
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
  ];
  # Hardened kernel – see ../kernel/rednode-kernel.config

  # Networking – privacy first
  networking.hostName = "rednode";
  networking.useDHCP = false;
  networking.firewall.enable = true;
  networking.firewall.allowedTCPPorts = [ ]; # Zero open ports by default – Network Agent opens explicitly
  networking.networkmanager.enable = false;
  services.resolved.enable = true;
  # DNS over HTTPS – Quad9
  services.resolved.dnsovertls = "true";

  # Time
  time.timeZone = "UTC";
  services.chrony.enable = true;

  # Locale
  i18n.defaultLocale = "en_US.UTF-8";
  console.keyMap = "us";

  # Users – RedNode owns the system, human is owner
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
    extraGroups = [ "wheel" "docker" "rednode" ];
    # Set with: mkpasswd -m sha-512
    initialHashedPassword = "";
    openssh.authorizedKeys.keys = [ ];
  };

  # Security – foundation, not feature
  security.sudo.wheelNeedsPassword = true;
  security.apparmor.enable = true;
  security.audit.enable = true;
  security.auditd.enable = true;
  # TPM2 – for LUKS key sealing
  security.tpm2.enable = true;
  security.tpm2.tctiEnvironment.enable = true;

  # No telemetry – ever
  # nixpkgs.config – no unfree telemetry blobs

  # Services – minimal base OS, RedNode owns orchestration
  services.openssh.enable = false; # SSH disabled by default – Security Agent can enable with hardening
  
  # PostgreSQL – Structured Memory
  services.postgresql = {
    enable = true;
    package = pkgs.postgresql_16;
    ensureDatabases = [ "rednode" ];
    ensureUsers = [{
      name = "rednode";
      ensureDBOwnership = true;
    }];
    authentication = "local rednode rednode trust";
    settings = {
      shared_preload_libraries = "vector";
    };
  };

  # NATS – Central Nervous System Bus
  services.nats = {
    enable = true;
    serverName = "rednode-nats";
    settings = {
      jetstream = { store_dir = "/var/lib/nats"; };
      accounts = {
        REDNODE = { users = [{ user = "rednode"; password = "$2a$..."; }]; };
      };
    };
  };

  # Qdrant – Vector Memory
  virtualisation.oci-containers.containers.qdrant = {
    image = "qdrant/qdrant:v1.9";
    ports = ["127.0.0.1:6333:6333"];
    volumes = ["/var/lib/qdrant/storage:/qdrant/storage"];
  };

  # Ollama – Local LLM
  services.ollama = {
    enable = true;
    acceleration = "cuda"; # set false for CPU-only
    host = "127.0.0.1";
    port = 11434;
  };

  # Observability – OpenTelemetry
  services.grafana = {
    enable = true;
    settings.server.http_addr = "127.0.0.1";
    settings.server.http_port = 3001;
  };
  services.prometheus = {
    enable = true;
    listenAddress = "127.0.0.1";
  };
  services.loki = {
    enable = true;
    configuration = {
      auth_enabled = false;
      server.http_listen_port = 3100;
    };
  };

  # Container runtime – Docker, NOT Kubernetes
  virtualisation.docker.enable = true;
  virtualisation.docker.daemon.settings = {
    live-restore = true;
    userland-proxy = false;
    no-new-privileges = true;
  };

  # Falco – eBPF Security Telemetry
  # services.falco.enable = true; # add custom module

  # RedNode Core – the operating brain
  # See flake.nix – systemd.services.rednode-core
  # For true OS mode (PID1):
  # systemd.enable = false;
  # boot.init = "${rednode-core}/bin/rednode-init";

  # Immutable OS – / is read-only in production image
  # system.etc.overlay.enable = true;
  # boot.initrd.systemd.enable = true;

  # Fonts – for dashboard
  fonts.packages = with pkgs; [ inter roboto-mono ];

  # Minimal packages – RedNode manages software
  environment.systemPackages = with pkgs; [
    # Base debug only – everything else via RedNode agents
    vim htop iotop btop
    git curl wget
    tmux
    age sops
    natscli
    qdrant
    # Security tools – exposed to Security Agent only
    falco
    lynis
    chkrootkit
    yara
  ];

  # No X11 / Wayland by default – RedNode is headless
  # Web UI served at http://localhost:3000
  # For kiosk mode: services.xserver.enable = true;

  # State – everything RedNode owns lives here – encrypted
  # /var/lib/rednode – Postgres, Qdrant, Kuzu, Ollama models, memory, audit log
  # This is the portable computational identity
  # rednode export → age-encrypted tar.zst of /var/lib/rednode

  system.stateVersion = "24.05";

  # Privacy – no telemetry, no phone-home, no crash reports
  nix.settings = {
    sandbox = true;
    auto-optimise-store = true;
  };
  # Disable all NixOS telemetry – there is none upstream, keep it that way
  # Disable popular-contrib, etc. – none enabled by default

  # RedNode-OS – The computer becomes the intelligence
  # – Privacy-first
  # – Self-aware
  # – Sentient – homeostatic drives, self-model, continuous autonomy
  # – Portable computational organism
}
