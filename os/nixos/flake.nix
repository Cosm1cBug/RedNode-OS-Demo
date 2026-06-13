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
      config.allowUnfree = true; # for nvidia / vscode – remove for fully free build
    };

    # RedNode Core – Rust – CNS + Sentience Engine + Tool Executor
    rednode-core = pkgs.rustPlatform.buildRustPackage {
      pname = "rednode-core";
      version = "0.3.1";
      src = ../../core/rednode-core;
      cargoLock = {
        lockFile = ../../core/rednode-core/Cargo.lock;
        allowBuiltinFetchGit = true;
      };
      nativeBuildInputs = with pkgs; [ pkg-config ];
      buildInputs = with pkgs; [ openssl sqlite ];
      # Build both rednode-core and rednode-init
      # Enable init PID1 + kuzu graph features
      # doCheck = false; # enable after tests are network-free
      doCheck = false;
      meta = with pkgs.lib; {
        description = "RedNode-OS Central Nervous System – Rust – privacy-first autonomous OS";
        license = licenses.mit;
        platforms = platforms.linux;
      };
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
        
        # RedNode CNS – systemd service – becomes PID1 in os-mode
        systemd.services.rednode-core = {
          description = "RedNode CNS – Central Nervous System";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" "postgresql.service" "nats.service" ];
          serviceConfig = {
            Type = "simple";
            ExecStart = "${rednode-core}/bin/rednode-core";
            Restart = "always";
            RestartSec = "2";
            User = "rednote";
          };
          environment = {
            RUST_LOG = "info";
            REDNODE_SENTIENCE = "on";
          };
        };
      }
    ];

  in {
    # NixOS configuration – RedNode-OS – systemd mode (stable)
    nixosConfigurations.rednode = nixpkgs.lib.nixosSystem {
      inherit system;
      modules = rednodeModules;
    };

    # NixOS configuration – RedNode-OS – TRUE OS MODE – rednode-init = PID1
    # nixosConfigurations.rednode-os = nixpkgs.lib.nixosSystem {
    #   inherit system;
    #   modules = rednodeModules ++ [ ./configuration-os.nix ];
    # };

    # ISO – net-install – 1.2 GB – models pull on first boot
    packages.x86_64-linux.iso = nixos-generators.nixosGenerate {
      system = "x86_64-linux";
      format = "iso";
      modules = rednodeModules ++ [
        { isoImage.squashfsCompression = "zstd -Xcompression-level 19"; }
      ];
    };

    # ISO – offline – with pre-seeded Ollama models – ~6.2 GB
    # Uncomment ollama model pre-seed in configuration.nix first
    # packages.x86_64-linux.iso-offline = nixos-generators.nixosGenerate { … }

    # QEMU VM – quick test
    packages.x86_64-linux.vm = self.nixosConfigurations.rednode.config.system.build.vm;

    # Dev shell – for building RedNode-OS
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
