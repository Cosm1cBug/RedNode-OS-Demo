# RedNode-OS — Optional Extra Services
#
# Import this module to enable additional infrastructure:
#   - WireGuard VPN (remote access to RedNode)
#   - NUT (Network UPS Tools — battery monitoring)
#   - Suricata IDS (intrusion detection)
#
# All disabled by default — enable via environment variables or
# by uncommenting the relevant sections.
#
# To enable: add ./extras.nix to imports in configuration.nix
{ config, pkgs, lib, ... }:
{
  # ──────────────────────────────────────────────
  # WireGuard VPN — Secure Remote Access
  # ──────────────────────────────────────────────
  # Generates server keys on first boot.
  # Peer configs created via: rednode intent "create VPN peer for my phone"
  # networking.wg-quick.interfaces.wg0 = {
  #   address = [ "10.100.0.1/24" ];
  #   listenPort = 51820;
  #   privateKeyFile = "/var/lib/rednode/wireguard/server.key";
  #   # Peers added dynamically by network-agent
  #   postUp = ''
  #     ${pkgs.iptables}/bin/iptables -A FORWARD -i wg0 -j ACCEPT
  #     ${pkgs.iptables}/bin/iptables -t nat -A POSTROUTING -o enp0s31f6 -j MASQUERADE
  #   '';
  #   postDown = ''
  #     ${pkgs.iptables}/bin/iptables -D FORWARD -i wg0 -j ACCEPT
  #     ${pkgs.iptables}/bin/iptables -t nat -D POSTROUTING -o enp0s31f6 -j MASQUERADE
  #   '';
  # };
  # networking.firewall.allowedUDPPorts = [ 51820 ];

  # ──────────────────────────────────────────────
  # NUT — Network UPS Tools
  # ──────────────────────────────────────────────
  # Auto-shutdown on low battery. Status available via:
  #   rednode intent "show UPS status"
  power.ups = {
    enable = false;  # Set to true if you have a UPS
    mode = "standalone";
    ups.rednode-ups = {
      driver = "usbhid-ups";
      port = "auto";
      description = "RedNode UPS";
    };
  };

  # Auto-shutdown when battery < 20%
  # power.ups.schedulerRules = ''
  #   AT LOW * EXECUTE "/run/current-system/sw/bin/systemctl poweroff"
  # '';

  # ──────────────────────────────────────────────
  # Suricata IDS — Intrusion Detection
  # ──────────────────────────────────────────────
  # Monitors network traffic for intrusion signatures.
  # Alerts fed to security-agent for auto-blocking.
  # services.suricata = {
  #   enable = true;
  #   settings = {
  #     af-packet = [{
  #       interface = "enp0s31f6";
  #       cluster-id = 99;
  #       cluster-type = "cluster_flow";
  #       defrag = true;
  #     }];
  #     outputs = [{
  #       eve-log = {
  #         enabled = true;
  #         filetype = "regular";
  #         filename = "/var/log/suricata/eve.json";
  #         types = [
  #           { alert.payload = true; alert.payload-printable = true; }
  #           { dns = {}; }
  #           { tls = {}; }
  #           { http = {}; }
  #         ];
  #       };
  #     }];
  #   };
  # };

  # ──────────────────────────────────────────────
  # Extra Packages for new features
  # ──────────────────────────────────────────────
  environment.systemPackages = with pkgs; [
    wireguard-tools    # wg, wg-quick
    qrencode           # QR codes for VPN peer configs
    nmap               # network scanning
    nut                # UPS monitoring
    smartmontools      # SMART disk monitoring
    lm_sensors         # CPU/GPU temperature
    ethtool            # network interface diagnostics
    whois              # domain lookups
    openssl            # SSL certificate checks
  ];
}
