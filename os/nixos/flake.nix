{
  description = "RedNode-OS – Personal Autonomous Operating System – Privacy-first, self-aware, sentient";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    nixos-generators = {
      url = "github:nix-community/nixos-generators";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, nixos-generators, ... }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      config.allowUnfree = true; # for nvidia – remove for fully free build
    };

    # RedNode Core – Rust – CNS + Sentience Engine + Tool Executor
    rednode-core = pkgs.rustPlatform.buildRustPackage {
      pname = "rednode-core";
      version = "0.8.0";
      src = ../../core/rednode-core;
      cargoLock = {
        lockFile = ../../core/rednode-core/Cargo.lock;
        allowBuiltinFetchGit = true;
      };
      nativeBuildInputs = with pkgs; [ pkg-config ];
      buildInputs = with pkgs; [ openssl sqlite ];
      doCheck = false;
      meta = with pkgs.lib; {
        description = "RedNode-OS Central Nervous System – Rust – privacy-first autonomous OS";
        license = licenses.mit;
        platforms = platforms.linux;
      };
    };

    # RedNode source tree — baked into ISO as a Nix store path
    # This means the source code is ALREADY ON DISK after install — no git clone needed
    rednode-source = pkgs.stdenv.mkDerivation {
      pname = "rednode-source";
      version = "0.8.0";
      src = pkgs.lib.cleanSource ../..;  # entire repo root
      phases = [ "unpackPhase" "installPhase" ];
      installPhase = ''
        mkdir -p $out
        cp -r . $out/
        # Remove build artifacts that shouldn't ship
        rm -rf $out/.git $out/node_modules $out/core/rednode-core/target
      '';
    };

    # RedNode OS – base modules
    rednodeModules = [
      ./configuration.nix
      ./hardware.nix
      ./disk-encryption.nix
      {
        # Override rednode-core package with our local build
        nixpkgs.overlays = [(final: prev: {
          rednode-core = rednode-core;
        })];
        environment.systemPackages = [ rednode-core ];

        # RedNode CNS – systemd service
        systemd.services.rednode-core = {
          description = "RedNode CNS – Central Nervous System";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" "postgresql.service" "nats.service" ];
          serviceConfig = {
            Type = "simple";
            ExecStart = "${rednode-core}/bin/rednode-core";
            Restart = "always";
            RestartSec = "2";
            User = "rednode";
          };
          environment = {
            RUST_LOG = "info";
            REDNODE_SENTIENCE = "on";
          };
        };
      }
    ];

    # Modules for the minimal appliance ISO
    rednodeISOModules = rednodeModules ++ [
      ./minimal.nix
      {
        # Bake the source tree into the installed system
        # On first boot, rednode-deploy copies from Nix store to /var/lib/rednode/source
        # This means: NO INTERNET NEEDED for the source code
        # Only internet needed: Ollama model download (5-9 GB)
        system.extraSystemBuilderCmds = ''
          ln -s ${rednode-source} $out/rednode-source
        '';

        # Tell the self-heal system where the baked-in source lives
        systemd.services.rednode-deploy.environment = {
          REDNODE_BAKED_SOURCE = "${rednode-source}";
        };

        # ISO-specific settings
        isoImage.squashfsCompression = "zstd -Xcompression-level 19";
        isoImage.isoName = "rednode-os-0.8.0-x86_64.iso";
        isoImage.volumeID = "REDNODE-OS";
      }
    ];

  in {
    # ─── System Configurations ───

    # NixOS configuration – RedNode-OS – systemd mode (stable)
    nixosConfigurations.rednode = nixpkgs.lib.nixosSystem {
      inherit system;
      modules = rednodeModules;
    };

    # NixOS configuration – Minimal appliance (for installed system)
    nixosConfigurations.rednode-minimal = nixpkgs.lib.nixosSystem {
      inherit system;
      modules = rednodeModules ++ [ ./minimal.nix ];
    };

    # ─── ISO Images ───

    # ISO – net-install – ~1.2 GB – source baked in, models pull on first boot
    packages.x86_64-linux.iso = nixos-generators.nixosGenerate {
      system = "x86_64-linux";
      format = "iso";
      modules = rednodeISOModules;
    };

    # ISO – offline – source + pre-seeded Ollama models – ~6.2 GB
    # Uncomment ollama model pre-seed in configuration.nix first
    # packages.x86_64-linux.iso-offline = nixos-generators.nixosGenerate {
    #   system = "x86_64-linux";
    #   format = "iso";
    #   modules = rednodeISOModules ++ [{
    #     # Pre-seed models into /var/lib/ollama/models/
    #     # Copy from a machine that has already pulled them
    #   }];
    # };

    # NixOS configuration – RedNode-OS with branded kiosk GUI
    nixosConfigurations.rednode-kiosk = nixpkgs.lib.nixosSystem {
      inherit system;
      modules = rednodeModules ++ [ ./kiosk.nix ];
    };

    # ISO with branded kiosk – for always-on dashboard display
    packages.x86_64-linux.iso-kiosk = nixos-generators.nixosGenerate {
      system = "x86_64-linux";
      format = "iso";
      modules = rednodeISOModules ++ [
        ./kiosk.nix
        { isoImage.isoName = "rednode-os-0.8.0-kiosk-x86_64.iso"; }
      ];
    };

    # ─── VM ───

    # QEMU VM – quick test
    packages.x86_64-linux.vm = self.nixosConfigurations.rednode.config.system.build.vm;

    # ─── Dev Shell ───

    devShells.x86_64-linux.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        rustc cargo clippy rustfmt
        nodejs_22 pnpm
        nats-server postgresql_16
        qdrant
        ollama
        nixos-generators
        minisign  # for ISO signing
        cosign   # alternative
      ];
      shellHook = ''
        echo "🧠 RedNode-OS Dev Shell"
        echo "  cargo run -p rednode-core"
        echo "  pnpm agents"
        echo "  pnpm web"
        echo "  nix build .#iso"
      '';
    };
  };
}
