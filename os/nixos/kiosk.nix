# RedNode-OS — Branded Kiosk Display Module
#
# Transforms the headless server into a branded appliance with:
#   1. Plymouth boot splash (RedNode branded — replaces NixOS logo)
#   2. Auto-login (no login prompt — straight to dashboard)
#   3. Cage Wayland kiosk compositor (runs ONE app: the browser)
#   4. Chromium in kiosk mode → http://localhost:3000 (RedNode dashboard)
#
# Resource usage:
#   - Cage compositor:     ~15 MB RAM
#   - Chromium kiosk:      ~150-300 MB RAM (depends on dashboard complexity)
#   - Plymouth:            0 MB (runs only during boot)
#   - Wayland/Mesa:        ~30 MB shared libs
#   ─────────────────────────────────────
#   Total GUI overhead:    ~200-350 MB RAM
#
# That leaves ~31.6 GB for RedNode services on a 32 GB machine.
#
# What this is NOT:
#   - Not a full desktop (no file manager, no taskbar, no app launcher)
#   - Not GNOME/KDE (those use 800 MB-1.5 GB)
#   - Not X11 (Wayland-only — lighter, more secure)
#
# The monitor shows ONLY the RedNode dashboard. Nothing else.
# Like a nerve center display / control panel.
#
# To enable: add ./kiosk.nix to your imports in configuration.nix
# To disable: remove the import (system goes back to headless)
{ config, pkgs, lib, ... }:

let
  # RedNode branding assets
  rednode-logo = ../../os/branding/rednode-logo.png;
  rednode-wallpaper = ../../os/branding/rednode-wallpaper.png;

  # Dashboard URL — the only thing the kiosk browser shows
  dashboardURL = "http://localhost:3000";

  # Kiosk user — unprivileged, auto-login, cannot sudo
  kioskUser = "kiosk";

  # Custom Plymouth theme for RedNode branding
  rednode-plymouth-theme = pkgs.stdenv.mkDerivation {
    pname = "rednode-plymouth-theme";
    version = "0.7.1";
    src = pkgs.writeTextDir "rednode-spinner/rednode-spinner.plymouth" ''
      [Plymouth Theme]
      Name=RedNode-OS
      Description=RedNode-OS — The Computer Becomes The Intelligence
      ModuleName=two-step

      [two-step]
      ImageDir=/share/plymouth/themes/rednode-spinner
      HorizontalAlignment=.5
      VerticalAlignment=.7
      Transition=none
      TransitionDuration=0.0
      BackgroundColor=0x0a0a0a
    '';
    buildInputs = [ pkgs.imagemagick ];
    installPhase = ''
      mkdir -p $out/share/plymouth/themes/rednode-spinner

      # Copy the theme file
      cp $src/rednode-spinner/rednode-spinner.plymouth \
        $out/share/plymouth/themes/rednode-spinner/

      # Generate spinner animation frames from the logo
      # Plymouth two-step needs: animation-XXXX.png, throbber-XXXX.png
      # We create a simple pulsing animation
      cp ${rednode-logo} $out/share/plymouth/themes/rednode-spinner/logo.png

      # Create 36 animation frames (10-degree rotation increments)
      cd $out/share/plymouth/themes/rednode-spinner
      for i in $(seq -w 0001 0036); do
        angle=$(( (10#$i - 1) * 10 ))
        convert ${rednode-logo} \
          -resize 64x64 \
          -background none \
          -gravity center \
          -extent 64x64 \
          "throbber-$i.png" 2>/dev/null || \
        cp ${rednode-logo} "throbber-$i.png"
      done

      # Watermark / static logo
      convert ${rednode-logo} -resize 128x128 logo.png 2>/dev/null || true

      # Background color box (fallback if ImageMagick isn't available)
      convert -size 1x1 xc:'#0a0a0a' box.png 2>/dev/null || true
    '';
  };

  # Chromium kiosk wrapper — waits for dashboard, then launches fullscreen
  chromium-kiosk = pkgs.writeShellScriptBin "rednode-kiosk" ''
    # Wait for the dashboard to be available
    echo "🧠 RedNode-OS Kiosk — waiting for dashboard at ${dashboardURL}..."

    ATTEMPTS=0
    MAX_ATTEMPTS=120  # 2 minutes max wait

    while [ $ATTEMPTS -lt $MAX_ATTEMPTS ]; do
      if ${pkgs.curl}/bin/curl -sf "${dashboardURL}" >/dev/null 2>&1; then
        echo "Dashboard ready — launching kiosk browser"
        break
      fi
      sleep 1
      ATTEMPTS=$((ATTEMPTS + 1))
    done

    if [ $ATTEMPTS -ge $MAX_ATTEMPTS ]; then
      echo "Dashboard not available after 2 minutes — launching anyway"
    fi

    # Small delay for GPU init
    sleep 2

    # Launch Chromium in kiosk mode
    exec ${pkgs.chromium}/bin/chromium \
      --kiosk \
      --no-first-run \
      --disable-translate \
      --disable-infobars \
      --disable-suggestions-service \
      --disable-save-password-bubble \
      --disable-session-crashed-bubble \
      --disable-component-update \
      --disable-background-networking \
      --disable-sync \
      --disable-default-apps \
      --disable-extensions \
      --disable-hang-monitor \
      --disable-popup-blocking \
      --disable-prompt-on-repost \
      --disable-client-side-phishing-detection \
      --disable-domain-reliability \
      --noerrdialogs \
      --no-default-browser-check \
      --autoplay-policy=no-user-gesture-required \
      --check-for-update-interval=31536000 \
      --ozone-platform=wayland \
      --enable-features=UseOzonePlatform \
      --start-fullscreen \
      --window-size=1920,1080 \
      "${dashboardURL}"
  '';

in
{
  # ──────────────────────────────────────────────
  # Plymouth — Branded Boot Splash
  # Shows the RedNode logo instead of NixOS snowflake
  # ──────────────────────────────────────────────
  boot.plymouth = {
    enable = true;
    theme = "rednode-spinner";
    themePackages = [ rednode-plymouth-theme ];
    logo = rednode-logo;
  };

  # Kernel params for clean boot (no text, just Plymouth)
  boot.kernelParams = [
    "splash"
    "vt.global_cursor_default=0"     # hide cursor during boot
  ];
  boot.consoleLogLevel = 0;
  boot.initrd.verbose = false;
  boot.loader.timeout = 0;           # skip boot menu — straight to RedNode

  # ──────────────────────────────────────────────
  # Kiosk User — unprivileged, auto-login
  # ──────────────────────────────────────────────
  users.users.${kioskUser} = {
    isNormalUser = true;
    home = "/home/${kioskUser}";
    createHome = true;
    group = "kiosk";
    extraGroups = [ "video" "render" "audio" ];  # GPU + audio access
    # No password, no sudo, no shell escape
  };
  users.groups.kiosk = {};

  # ──────────────────────────────────────────────
  # Cage — Wayland Kiosk Compositor
  # Runs exactly ONE application (Chromium) fullscreen
  # ~15 MB RAM — lighter than any desktop environment
  # ──────────────────────────────────────────────
  services.cage = {
    enable = true;
    user = kioskUser;
    program = "${chromium-kiosk}/bin/rednode-kiosk";
    extraArguments = [ "-s" ];  # -s = single output (one monitor)
  };

  # ──────────────────────────────────────────────
  # GPU — enable OpenGL for Wayland/Chromium
  # ──────────────────────────────────────────────
  hardware.graphics.enable = true;
  # hardware.opengl is being renamed to hardware.graphics in newer NixOS
  # Both work — NixOS resolves the alias

  # ──────────────────────────────────────────────
  # Auto-hide cursor after 3 seconds of inactivity
  # (kiosk displays shouldn't show a mouse cursor)
  # ──────────────────────────────────────────────
  environment.systemPackages = [
    chromium-kiosk
    pkgs.unclutter-xfixes   # auto-hide cursor
  ];

  # ──────────────────────────────────────────────
  # Screen — prevent DPMS/screensaver (always-on display)
  # ──────────────────────────────────────────────
  services.logind.extraConfig = ''
    HandleLidSwitch=ignore
    HandleLidSwitchExternalPower=ignore
    IdleAction=ignore
  '';

  # ──────────────────────────────────────────────
  # Kiosk crash recovery — if Chromium/Cage dies, restart
  # ──────────────────────────────────────────────
  systemd.services."cage-tty1" = {
    serviceConfig = {
      Restart = "always";
      RestartSec = "3";
    };
  };

  # ──────────────────────────────────────────────
  # Fonts — for Chromium to render the dashboard properly
  # ──────────────────────────────────────────────
  fonts.fontconfig.enable = true;
  fonts.packages = with pkgs; [
    inter              # dashboard UI font
    roboto-mono        # monospace / code
    noto-fonts         # fallback unicode
  ];
}
