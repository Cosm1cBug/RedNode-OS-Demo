# RedNode-OS – Hardware Profile – Generic x86_64
# Target: ThinkPad / Framework / NUC – 8c/32GB min, 14c/64GB + RTX recommended
{ config, lib, pkgs, modulesPath, ... }:
{
  imports = [ (modulesPath + "/installer/scan/not-detected.nix") ];

  boot.initrd.availableKernelModules = [ "xhci_pci" "ahci" "nvme" "usbhid" "usb_storage" "sd_mod" "thunderbolt" ];
  boot.initrd.kernelModules = [ "dm-crypt" ];
  boot.kernelModules = [ "kvm-intel" "kvm-amd" ];
  boot.extraModulePackages = [ ];

  # Filesystems – set in disk-encryption.nix
  # fileSystems."/" = { device = "/dev/mapper/rednode-root"; fsType = "btrfs"; options = [ "compress=zstd" "noatime" ]; };
  # fileSystems."/boot" = { device = "/dev/disk/by-label/REDNODE_BOOT"; fsType = "vfat"; };

  # CPU microcode
  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
  hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;

  # GPU – NVIDIA – for local LLM
  hardware.opengl.enable = true;
  services.xserver.videoDrivers = [ "nvidia" ];
  hardware.nvidia = {
    modesetting.enable = true;
    powerManagement.enable = true;
    open = false;
    nvidiaSettings = false;
    package = config.boot.kernelPackages.nvidiaPackages.stable;
  };

  # Bluetooth – disabled by default – Security Agent can enable
  hardware.bluetooth.enable = false;

  # Audio – PipeWire – for Voice Interface
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    pulse.enable = true;
  };

  # TPM2 – for LUKS key sealing
  boot.initrd.systemd.tpm2.enable = true;

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
}
