# RedNode-OS – Disk Encryption – LUKS2 + TPM2
# Portable computational identity – encrypted at rest
{ config, lib, ... }:
{
  # Example – adjust UUIDs for your install
  # boot.initrd.luks.devices."rednode-root" = {
  #   device = "/dev/disk/by-uuid/xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx";
  #   preLVM = true;
  #   allowDiscards = true;
  #   # TPM2 auto-unlock – sealed to PCR 0,2,7
  #   crypttabExtraOpts = ["tpm2-device=auto"];
  # };

  # For ISO / live USB – no encryption (user encrypts on install)
  # For installed system – use:
  # nixos-generate-config – then add LUKS above

  # Btrfs layout – snapshots for self-healing rollback
  # subvolumes:
  #   @ -> /
  #   @nix -> /nix
  #   @state -> /var/lib/rednode   # <-- This is the Node – portable
  #   @snapshots -> /.snapshots
  #
  # Security Agent auto-patcher:
  # btrfs subvolume snapshot -r / /.snapshots/pre-patch-<timestamp>
  # if patch fails → btrfs subvolume snapshot /.snapshots/pre-patch-xxx / – rollback in <3s

  # Swap – encrypted, random key
  # swapDevices = [{ device = "/dev/disk/by-partlabel/swap"; randomEncryption = true; }];

  # /var/lib/rednode – encrypted, this IS your computational identity
  # export: rednode export → age-encrypted tar.zst of /var/lib/rednode
  # import: rednode import bundle.rednode.age → resume anywhere <60s
}
