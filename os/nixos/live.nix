# RedNode-OS — Live Boot Profile
#
# This module configures the ISO for live booting on ANY machine
# without installation. Changes are lost on reboot.
#
# Networking is already DHCP + NetworkManager in the base config,
# so this module only adds:
#   - Auto-login (no password prompt)
#   - SSH enabled with temporary password
#   - CPU-only Ollama (for GPU compatibility)
#   - Live-specific branding
#   - Extra diagnostic tools
#
# Build:  cd os/nixos && nix build .#iso-live
# Flash:  sudo dd if=result/iso/*.iso of=/dev/sdX bs=4M status=progress
# Boot:   Plug USB → boot from USB → RedNode starts automatically
{ config, pkgs, lib, ... }:
{
  # ──────────────────────────────────────────────
  # Live user — auto-login, simple password
  # ──────────────────────────────────────────────
  users.mutableUsers = lib.mkForce true;

  users.users.rednode-live = {
    isNormalUser = true;
    extraGroups = [ "wheel" "docker" "networkmanager" "video" "render" ];
    initialPassword = "rednode";
    description = "RedNode Live User";
  };

  # Auto-login on TTY1
  services.getty.autologinUser = lib.mkForce "rednode-live";

  # Passwordless sudo for live user
  security.sudo.wheelNeedsPassword = lib.mkForce false;

  # ──────────────────────────────────────────────
  # SSH — enabled for live access from laptop
  # ──────────────────────────────────────────────
  services.openssh = {
    enable = lib.mkForce true;
    settings = {
      PermitRootLogin = "no";
      PasswordAuthentication = true;  # OK for live demo, not production
    };
  };

  # ──────────────────────────────────────────────
  # Live boot banner
  # ──────────────────────────────────────────────
  environment.etc.issue.text = lib.mkForce ''

    ══════════════════════════════════════════════════════════
     🧠  R E D N O D E - O S    v0.9.0   [ LIVE MODE ]
         The Personal Autonomous Operating System
    ──────────────────────────────────────────────────────────
     Dashboard: http://\4:3000
     API:       http://\4:8787
     SSH:       ssh rednode-live@\4  (password: rednode)
    ──────────────────────────────────────────────────────────
     This is a LIVE session — changes are lost on reboot.
     To install permanently: sudo rednode-install
    ══════════════════════════════════════════════════════════

  '';

  services.getty.greetingLine = lib.mkForce ''
    \e{bold}\e{lightred}RedNode-OS LIVE\e{reset} — \l @ \n (\4)
  '';

  services.getty.helpLine = lib.mkForce ''
    Auto-logged in. Dashboard: http://\4:3000 | SSH password: rednode
  '';

  # ──────────────────────────────────────────────
  # Live MOTD
  # ──────────────────────────────────────────────
  environment.etc."motd".text = lib.mkForce ''

    ╔════════════════════════════════════════════════════╗
    ║  🧠 RedNode-OS — LIVE DEMO MODE                    ║
    ╚════════════════════════════════════════════════════╝

    Services starting automatically...
      PostgreSQL, NATS, Ollama, Qdrant, Grafana

    Commands:
      rednode status            — check all services
      rednode intent "hello"    — talk to RedNode
      rednode gui on            — start kiosk display on this monitor
      rednode gui off           — back to terminal

    Dashboard:  http://THIS-IP:3000  (check IP with: ip addr)
    API:        http://THIS-IP:8787
    Grafana:    http://THIS-IP:3001  (admin/rednode)

    ⚠️  This is a LIVE session — nothing is saved to disk.
    To install RedNode permanently on this machine:
      sudo rednode-install

  '';

  # ──────────────────────────────────────────────
  # Ollama — CPU mode for live demo (GPU may not be configured)
  # ──────────────────────────────────────────────
  services.ollama.acceleration = lib.mkForce null;

  # ──────────────────────────────────────────────
  # Extra packages for live environment
  # ──────────────────────────────────────────────
  environment.systemPackages = with pkgs; [
    pciutils usbutils   # Hardware detection
  ];
}
