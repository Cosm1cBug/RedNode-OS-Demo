# RedNode-OS — Autonomous Deployment & Self-Healing NixOS Module
#
# This module ensures RedNode-OS is fully deployed, running, and
# self-healing after every boot. No manual git clone needed.
#
# What it does:
#   1. Copies rednode-selfheal.sh into /usr/local/bin
#   2. Creates systemd services:
#      - rednode-deploy.service  → one-shot, runs on first boot
#      - rednode-selfheal.service → continuous watchdog
#      - rednode-selfheal.timer   → periodic health checks
#   3. All state lives in /var/lib/rednode/
#   4. Logs go to /var/lib/rednode/logs/
#   5. Source is cloned to /var/lib/rednode/source/
#
# The user never needs to manually clone, build, or start anything.
# On first boot: install. On every boot: verify + repair if needed.
{ config, pkgs, lib, ... }:
{
  # ──────────────────────────────────────────────
  # Self-heal script — placed on PATH
  # ──────────────────────────────────────────────
  environment.etc."rednode/selfheal.sh" = {
    source = ../../scripts/rednode-selfheal.sh;
    mode = "0755";
  };

  # Symlink to /usr/local/bin for easy CLI access
  environment.systemPackages = [
    (pkgs.writeShellScriptBin "rednode-selfheal" ''
      exec /etc/rednode/selfheal.sh "$@"
    '')
    (pkgs.writeShellScriptBin "rednode" ''
      REDNODE_SOURCE="/var/lib/rednode/source"
      case "''${1:-help}" in
        status)
          /etc/rednode/selfheal.sh diagnose
          ;;
        repair)
          sudo /etc/rednode/selfheal.sh repair
          ;;
        intent)
          shift
          INTENT="$*"
          TOKEN=$(grep "^REDNODE_API_TOKEN=" "$REDNODE_SOURCE/.env" 2>/dev/null | cut -d= -f2-)
          curl -s -X POST http://127.0.0.1:8787/intent \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $TOKEN" \
            -d "{\"intent\":\"$INTENT\"}" | python3 -m json.tool
          ;;
        logs)
          less /var/lib/rednode/logs/selfheal.log
          ;;
        update)
          sudo /etc/rednode/selfheal.sh repair
          ;;
        help|--help|-h|"")
          echo ""
          echo "🧠 RedNode-OS CLI"
          echo ""
          echo "Usage: rednode <command>"
          echo ""
          echo "Commands:"
          echo "  status             Show system health"
          echo "  repair             Auto-repair any broken services"
          echo "  intent \"text\"       Send intent to CNS"
          echo "  logs               View self-heal logs"
          echo "  update             Pull latest + rebuild + restart"
          echo ""
          ;;
        *)
          echo "Unknown command: $1 — try: rednode help"
          ;;
      esac
    '')
  ];

  # ──────────────────────────────────────────────
  # Directories
  # ──────────────────────────────────────────────
  systemd.tmpfiles.rules = [
    "d /var/lib/rednode 0750 rednode rednode -"
    "d /var/lib/rednode/logs 0750 rednode rednode -"
    "d /var/lib/rednode/source 0750 rednode rednode -"
    "d /var/lib/rednode/backups 0750 rednode rednode -"
    "d /var/lib/rednode/qdrant 0750 rednode rednode -"
  ];

  # ──────────────────────────────────────────────
  # Service: rednode-deploy — ONE-SHOT on first boot
  # Clones the repo, builds, configures, starts everything.
  # Idempotent — safe to re-run. Skips steps that are done.
  # ──────────────────────────────────────────────
  systemd.services.rednode-deploy = {
    description = "RedNode-OS — First Boot Autonomous Deployment";
    wantedBy = [ "multi-user.target" ];
    after = [
      "network-online.target"
      "postgresql.service"
      "nats.service"
      "ollama.service"
      "docker.service"
    ];
    wants = [ "network-online.target" ];

    # Only run if install hasn't completed yet
    # (the selfheal script saves state to /var/lib/rednode/.selfheal-state)
    unitConfig = {
      ConditionPathExists = "!/var/lib/rednode/.deploy-complete";
    };

    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "/etc/rednode/selfheal.sh install";
      ExecStartPost = "${pkgs.coreutils}/bin/touch /var/lib/rednode/.deploy-complete";
      TimeoutStartSec = "3600";  # 1 hour max (model downloads take time)
      Restart = "on-failure";
      RestartSec = "30";

      # Run as root for service management, but clone/build as rednode user
      # (the script handles permission correctly)
      User = "root";

      # Environment
      Environment = [
        "HOME=/var/lib/rednode"
        "REDNODE_HOME=/var/lib/rednode"
        "REDNODE_SOURCE=/var/lib/rednode/source"
        "REDNODE_REPO=https://github.com/Cosm1cBug/RedNode-OS-Demo.git"
        "REDNODE_BRANCH=main"
        "PATH=${lib.makeBinPath (with pkgs; [
          git curl wget coreutils gnugrep gnused gawk
          openssh openssl
          rustc cargo
          nodejs_22 nodePackages.pnpm
          docker
          ollama
          python312
          iproute2 dnsutils iputils systemd
        ])}:/run/current-system/sw/bin:/usr/bin:/bin"
        "RUST_LOG=info"
      ];
    };
  };

  # ──────────────────────────────────────────────
  # Service: rednode-selfheal — CONTINUOUS WATCHDOG
  # Monitors all subsystems, auto-repairs on failure.
  # Also checks for git updates once per day.
  # ──────────────────────────────────────────────
  systemd.services.rednode-selfheal = {
    description = "RedNode-OS — Self-Healing Watchdog";
    wantedBy = [ "multi-user.target" ];
    after = [
      "rednode-deploy.service"
      "network-online.target"
    ];
    wants = [ "network-online.target" ];

    serviceConfig = {
      Type = "simple";
      ExecStart = "/etc/rednode/selfheal.sh watch";
      Restart = "always";
      RestartSec = "10";
      User = "root";

      Environment = [
        "HOME=/var/lib/rednode"
        "REDNODE_HOME=/var/lib/rednode"
        "REDNODE_SOURCE=/var/lib/rednode/source"
        "PATH=${lib.makeBinPath (with pkgs; [
          git curl wget coreutils gnugrep gnused gawk
          openssh openssl
          rustc cargo
          nodejs_22 nodePackages.pnpm
          docker
          ollama
          python312
          iproute2 dnsutils iputils systemd
        ])}:/run/current-system/sw/bin:/usr/bin:/bin"
        "RUST_LOG=info"
      ];

      # Watchdog — systemd restarts if script crashes
      WatchdogSec = "600";  # 10 min — script should heartbeat

      # Resource limits — don't let repair loops eat the system
      MemoryMax = "512M";
      CPUQuota = "50%";
    };
  };

  # ──────────────────────────────────────────────
  # Timer: rednode-health — runs diagnose every 15 min
  # Separate from watchdog — logs health to journal
  # ──────────────────────────────────────────────
  systemd.services.rednode-health = {
    description = "RedNode-OS — Periodic Health Check";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "/etc/rednode/selfheal.sh diagnose";
      User = "root";
      Environment = [
        "REDNODE_HOME=/var/lib/rednode"
        "REDNODE_SOURCE=/var/lib/rednode/source"
        "PATH=${lib.makeBinPath (with pkgs; [
          curl coreutils gnugrep docker systemd python312
        ])}:/run/current-system/sw/bin:/usr/bin:/bin"
      ];
    };
  };

  systemd.timers.rednode-health = {
    description = "RedNode-OS — Health Check Timer";
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnBootSec = "5min";
      OnUnitActiveSec = "15min";
      Persistent = true;
    };
  };

  # ──────────────────────────────────────────────
  # MOTD — show RedNode status on login
  # ──────────────────────────────────────────────
  environment.etc."motd".text = ''

    ╔═══════════════════════════════════════════╗
    ║  🧠 RedNode-OS — Autonomous Intelligence  ║
    ╚═══════════════════════════════════════════╝

    Commands:
      rednode status      — system health
      rednode intent "…"  — talk to RedNode
      rednode repair      — auto-fix issues
      rednode logs        — self-heal log

    Docs:   /var/lib/rednode/source/docs/
    Config: /var/lib/rednode/source/.env

  '';
}
