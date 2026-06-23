# RedNode-OS — Minimal Appliance Profile
#
# This module strips NixOS down to the absolute minimum needed for RedNode.
# Import this in your configuration or flake ISO target to produce a lean system.
#
# NixOS "required" packages (cannot remove — kernel/systemd depend on them):
#   acl, attr, bash, bzip2, coreutils, cpio, curl, diffutils, findutils,
#   gawk, glibc, getent, getconf, grep, patch, sed, tar, gzip, xz, less,
#   libcap, ncurses, netcat, openssh, mkpasswd, procps, su, time,
#   util-linux, which, zstd
#
# NixOS "default" packages (CAN remove — not strictly needed):
#   perl, rsync, strace  →  we remove all three
#
# What we KEEP (RedNode needs these):
#   Everything in configuration.nix's environment.systemPackages
#   + the services declared there (postgres, nats, ollama, etc.)
#
# What we REMOVE:
#   - perl, rsync, strace (NixOS defaults — not needed)
#   - All X11/Wayland/GUI libraries
#   - All fonts except what dashboard needs
#   - All sound themes, icon themes, MIME databases
#   - Documentation (man pages, info pages)
#   - Nano (we have vim)
#   - All "profiles/base.nix" bloat (mdadm, etc.)
#
# Result: ~800 MB installed (vs ~3 GB typical NixOS), ~1.2 GB ISO
{ config, pkgs, lib, ... }:
{
  # ──────────────────────────────────────────────
  # Remove NixOS default packages (perl, rsync, strace)
  # ──────────────────────────────────────────────
  environment.defaultPackages = lib.mkForce [];

  # ──────────────────────────────────────────────
  # Disable GUI/desktop cruft (unless kiosk.nix overrides)
  # ──────────────────────────────────────────────
  # No X11 by default — kiosk.nix uses Wayland/Cage directly
  services.xserver.enable = lib.mkDefault false;

  # No XDG desktop integration (icons/sounds for desktop environments)
  xdg.icons.enable = lib.mkDefault false;
  xdg.mime.enable = lib.mkDefault false;
  xdg.sounds.enable = lib.mkDefault false;

  # Fontconfig kept enabled — dashboard + kiosk both need it
  fonts.fontconfig.enable = lib.mkDefault true;

  # ──────────────────────────────────────────────
  # Disable unnecessary NixOS modules
  # ──────────────────────────────────────────────
  # No software RAID (we use single-disk or ZFS/btrfs)
  boot.swraid.enable = lib.mkForce false;

  # No printing
  services.printing.enable = false;

  # No Avahi/mDNS (we use static IPs + Pi-hole DNS)
  services.avahi.enable = false;

  # No CUPS browsing
  # No USB automount (headless server)
  services.udisks2.enable = false;

  # No power management (always-on server)
  powerManagement.enable = false;
  services.thermald.enable = false;

  # No NTP via systemd-timesyncd (we use chrony in configuration.nix)
  services.timesyncd.enable = false;

  # ──────────────────────────────────────────────
  # Minimize documentation
  # ──────────────────────────────────────────────
  documentation.enable = false;
  documentation.doc.enable = false;
  documentation.info.enable = false;
  documentation.man.enable = false;
  documentation.nixos.enable = false;

  # ──────────────────────────────────────────────
  # Strip extra outputs (dev headers, docs)
  # ──────────────────────────────────────────────
  environment.extraOutputsToInstall = lib.mkForce [];

  # ──────────────────────────────────────────────
  # Nix store optimization
  # ──────────────────────────────────────────────
  nix.settings = {
    auto-optimise-store = true;
    # Garbage collect on low disk
    min-free = 1073741824;        # 1 GB
    max-free = 5368709120;        # 5 GB
  };

  # Auto garbage-collect old generations
  nix.gc = {
    automatic = true;
    dates = "weekly";
    options = "--delete-older-than 14d";
  };

  # ──────────────────────────────────────────────
  # Kernel: remove unnecessary modules
  # ──────────────────────────────────────────────
  # Don't load every hardware module under the sun
  boot.initrd.includeDefaultModules = lib.mkDefault false;

  # Only include what we actually need
  boot.initrd.availableKernelModules = [
    # Storage
    "ahci" "nvme" "sd_mod" "usb_storage" "usbhid"
    # USB controller
    "xhci_pci"
    # Thunderbolt (Framework/ThinkPad)
    "thunderbolt"
    # Encryption
    "dm-crypt" "dm-mod"
    # Filesystem
    "ext4" "btrfs" "vfat"
    # Virtio (for VM testing)
    "virtio_pci" "virtio_blk" "virtio_net" "virtio_scsi"
  ];

  # ──────────────────────────────────────────────
  # Console: minimal but functional for emergencies
  # ──────────────────────────────────────────────
  # vim is our editor (installed in configuration.nix)
  programs.nano.enable = false;
  environment.variables.EDITOR = "vim";
}
