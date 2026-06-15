# flake.nix — Complete Bonsai Ecosystem & USOS Co-OS integration for NixOS
{
  description = "Bonsai Ecosystem & USOS Co-OS — run alongside NixOS in real-time parallel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Rust toolchain with necessary targets (bare-metal kernel for USOS)
        rust-toolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [
            "x86_64-unknown-none-elf"
            "x86_64-unknown-linux-gnu"
            "aarch64-unknown-none-elf"
          ];
          extensions = [ "rust-src" "llvm-tools-preview" ];
        };

        # Crane setup for incremental Rust builds
        craneLib = crane.mkLib pkgs;

        # Source filter
        src = craneLib.cleanCargoSource (craneLib.path ../.);

        # Common build inputs
        buildInputs = with pkgs; [
          openssl
          pkg-config
          udev
          vulkan-loader
          wayland
          xorg.libX11
          xorg.libXi
          xorg.libXcursor
          xorg.libXrandr
          libxkbcommon
          llvm
          binutils
        ];

        nativeBuildInputs = with pkgs; [
          rust-toolchain
          clang
          lld
          pkg-config
          cmake
          ninja
        ];

        # Common arguments for cargo builds
        commonArgs = {
          inherit src buildInputs nativeBuildInputs;
          CARGO_PROFILE_RELEASE_LTO = "thin";
          CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16";
          RUSTFLAGS = "-C target-cpu=native -C link-arg=-fuse-ld=lld";
        };

        # ════════════════════════════════════════════════════════════════
        # Build individual USOS & Bonsai crates
        # ════════════════════════════════════════════════════════════════

        # Sentinel Core (bare-metal microkernel)
        sentinel-core = craneLib.buildPackage (commonArgs // {
          pname = "sentinel-core";
          version = "0.1.0";
          cargoExtraArgs = "-p sentinel-core --target x86_64-unknown-none-elf --release";
          doCheck = false;
          postInstall = ''
            mkdir -p $out/bin
            cp target/x86_64-unknown-none-elf/release/sentinel-core $out/bin/
          '';
        });

        # Weave component system
        bonsai-weave = craneLib.buildPackage (commonArgs // {
          pname = "bonsai-weave";
          version = "0.1.0";
          cargoExtraArgs = "-p bonsai-weave --release";
        });

        # Bonsai CLI
        bonsai-cli = craneLib.buildPackage (commonArgs // {
          pname = "bonsai-cli";
          version = "0.1.0";
          cargoExtraArgs = "-p cargo-bace --release";
        });

        # MCP Server
        bonsai-mcp-server = craneLib.buildPackage (commonArgs // {
          pname = "bonsai-mcp-server";
          version = "0.1.0";
          cargoExtraArgs = "-p bonsai-mcp-server --release";
        });

        # BMF Server
        bonsai-bmf-server = craneLib.buildPackage (commonArgs // {
          pname = "bonsai-bmf-server";
          version = "0.1.0";
          cargoExtraArgs = "-p bonsai-bmf-server --release";
        });

        # Bonsai Workspace (Tauri app, builds for Wayland)
        bonsai-workspace = craneLib.buildPackage (commonArgs // {
          pname = "bonsai-workspace";
          version = "0.2.0";
          cargoExtraArgs = "-p bonsai-workspace --release";
          nativeBuildInputs = nativeBuildInputs ++ [ pkgs.nodejs_20 pkgs.pnpm ];
        });

        # ════════════════════════════════════════════════════════════════
        # VM Image Assembly
        # ════════════════════════════════════════════════════════════════

        # USOS kernel image
        usos-kernel = pkgs.callPackage ./packages/usos-kernel.nix {
          inherit sentinel-core;
        };

        # USOS initrd
        usos-initrd = pkgs.callPackage ./packages/usos-initrd.nix {
          inherit bonsai-weave bonsai-workspace;
        };

        # Complete USOS VM package
        usos-vm = pkgs.callPackage ./packages/usos-vm.nix {
          inherit usos-kernel usos-initrd;
        };

      in {
        # ════════════════════════════════════════════════════════════════
        # Public packages for NixOS integration
        # ════════════════════════════════════════════════════════════════

        packages = {
          inherit sentinel-core bonsai-weave bonsai-cli bonsai-mcp-server
                  bonsai-bmf-server bonsai-workspace usos-kernel usos-initrd usos-vm;

          # Convenience package: all Bonsai services
          bonsai-all = pkgs.symlinkJoin {
            name = "bonsai-all";
            paths = [
              bonsai-cli
              bonsai-mcp-server
              bonsai-bmf-server
              bonsai-workspace
            ];
          };

          default = usos-vm;
        };

        # ════════════════════════════════════════════════════════════════
        # NixOS modules
        # ════════════════════════════════════════════════════════════════

        nixosModules = {
          bonsai-services = ./modules/bonsai-services.nix;
          usos-co-os = ./modules/usos-co-os.nix;
          default = ./modules/default.nix;
        };

        # ════════════════════════════════════════════════════════════════
        # Development shell
        # ════════════════════════════════════════════════════════════════

        devShells.default = pkgs.mkShell {
          buildInputs = buildInputs ++ [
            rust-toolchain
            pkgs.cargo-watch
            pkgs.sccache
            pkgs.mold
            pkgs.qemu
            pkgs.libvirt
            pkgs.virt-manager
            pkgs.nodejs_20
            pkgs.pnpm
            pkgs.rustfmt
            pkgs.clippy
          ];

          shellHook = ''
            export RUSTC_WRAPPER=sccache
            export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld"
            export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
            echo "🧬 Bonsai Ecosystem development environment ready."
            echo "   Run: cargo build -p <crate>"
            echo "   VM:  nix build .#usos-vm && ./result/bin/usos-vm"
          '';
        };

        # ════════════════════════════════════════════════════════════════
        # NixOS configuration example
        # ════════════════════════════════════════════════════════════════

        nixosConfigurations.coos = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./modules/bonsai-services.nix
            ./modules/usos-co-os.nix
            {
              # Example: enable both Bonsai services and USOS co-OS
              services.bonsai-services = {
                enable = true;
                mcp-server.enable = true;
                bmf-server.enable = true;
              };

              services.usos-co-os = {
                enable = true;
                memory = "4G";
                cpuCores = 4;
              };
            }
          ];
        };
      }
    );
}
