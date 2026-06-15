# flake.nix – Complete Omnisystem NixOS Integration
# Enables seamless deployment of Omnisystem at any integration level
# Usage: Add to any flake.nix: inputs.omnisystem.url = "github:LoopyLuci/Omnisystem"

{
  description = "Omnisystem – Sovereign, verifiable, self-managing OS with NixOS integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    let
      allSystems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];

      forAllSystems = nixpkgs.lib.genAttrs allSystems;

      mkPkgsFor = system: nixpkgs.legacyPackages.${system};

    in {
      # Package outputs for each system
      packages = forAllSystems (system:
        let
          pkgs = mkPkgsFor system;
          omnisystem-pkgs = import ./nix/packages.nix { inherit pkgs; };
        in
          omnisystem-pkgs
      );

      # Development shells
      devShells = forAllSystems (system:
        let
          pkgs = mkPkgsFor system;
        in {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              nix
              nixpkgs-fmt
              nil  # Nix language server
              direnv
            ];
          };

          omnisystem = pkgs.mkShell {
            buildInputs = with pkgs; [
              nix
              cargo
              rustc
              cmake
              llvm
              pkg-config
            ];
          };
        }
      );

      # NixOS modules for integration
      nixosModules = {
        default = import ./nix/modules/omnisystem.nix;
        omnisystem = import ./nix/modules/omnisystem.nix;
        autonomic = import ./nix/modules/autonomic.nix;
        compatibility = import ./nix/modules/compatibility.nix;
        governance = import ./nix/modules/governance.nix;
      };

      # Pre-configured NixOS systems
      nixosConfigurations = {
        # Hosted-light: Omnisystem as NixOS systemd services
        omnisystem-hosted-light = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./nix/modules/omnisystem.nix
            {
              services.omnisystem = {
                enable = true;
                mode = "hosted-light";
                services = {
                  transfer-daemon.enable = true;
                  ums.enable = true;
                  omnicloak.enable = true;
                };
                adapter = {
                  memory_mb = 2048;
                  cpus = 2;
                };
              };
            }
          ];
        };

        # Hosted-full: UOSC as KVM guest with seamless integration
        omnisystem-hosted-full = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./nix/modules/omnisystem.nix
            {
              services.omnisystem = {
                enable = true;
                mode = "hosted-full";
                services = {
                  transfer-daemon.enable = true;
                  ums.enable = true;
                  vfs.enable = true;
                  display.enable = true;
                  omnicloak.enable = true;
                };
                adapter = {
                  memory_mb = 4096;
                  cpus = 4;
                  gpu_passthrough = true;
                };
              };
            }
          ];
        };

        # Bare-metal: Standalone UOSC OS
        omnisystem-baremetal = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./nix/modules/omnisystem.nix
            {
              services.omnisystem = {
                enable = true;
                mode = "bare-metal";
                services = {
                  kernel.enable = true;
                  init.enable = true;
                  transfer-daemon.enable = true;
                  ums.enable = true;
                  vfs.enable = true;
                  storage.enable = true;
                  device-manager.enable = true;
                  display.enable = true;
                  compositor.enable = true;
                  omnicloak.enable = true;
                  logger.enable = true;
                  config.enable = true;
                  service-manager.enable = true;
                };
                autonomic.enable = true;
              };
            }
          ];
        };

        # Library OS: UOSC as linked library
        omnisystem-library = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./nix/modules/omnisystem.nix
            {
              services.omnisystem = {
                enable = true;
                mode = "library-os";
                services = {
                  kernel.enable = true;
                };
                adapter = {
                  memory_mb = 64;
                  cpus = 1;
                };
              };
            }
          ];
        };

        # IoT Edge Node: Minimal footprint
        omnisystem-iot = nixpkgs.lib.nixosSystem {
          system = "aarch64-linux";
          modules = [
            ./nix/modules/omnisystem.nix
            {
              services.omnisystem = {
                enable = true;
                mode = "hosted-light";
                services = {
                  kernel.enable = true;
                  transfer-daemon.enable = true;
                  device-manager.enable = true;
                };
                adapter = {
                  memory_mb = 128;
                  cpus = 1;
                };
              };
            }
          ];
        };

        # Developer Workstation: Full integration
        omnisystem-developer = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./nix/modules/omnisystem.nix
            ./nix/modules/autonomic.nix
            ./nix/modules/compatibility.nix
            {
              services.omnisystem = {
                enable = true;
                mode = "hosted-full";
                services = {
                  transfer-daemon.enable = true;
                  ums.enable = true;
                  omnicloak.enable = true;
                  compositor.enable = true;
                  vfs.enable = true;
                };
                autonomic.enable = true;
                compatibility = {
                  posix.enable = true;
                  windows.enable = false;
                };
              };
            }
          ];
        };
      };

      # Build outputs
      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.build-cli}/bin/build";
        };

        deploy = {
          type = "app";
          program = "${self.packages.${system}.build-cli}/bin/build deploy";
        };

        docker = {
          type = "app";
          program = "${self.packages.${system}.build-cli}/bin/build docker";
        };
      });

      # Build images
      images = {
        # Bare-metal bootable ISO
        omnisystem-iso = self.nixosConfigurations.omnisystem-baremetal.config.system.build.isoImage;

        # QEMU disk image for hosted-full
        omnisystem-qemu = self.nixosConfigurations.omnisystem-hosted-full.config.system.build.qemuImage;

        # Docker image
        omnisystem-docker = self.nixosConfigurations.omnisystem-hosted-light.config.system.build.dockerImage;

        # Raspberry Pi image (ARM)
        omnisystem-rpi = self.nixosConfigurations.omnisystem-iot.config.system.build.sdImage;
      };

      # Overlays for nixpkgs
      overlays.default = final: prev: {
        omnisystem = final.callPackage ./nix/packages.nix { };
      };

      # Formatter
      formatter = forAllSystems (system:
        (mkPkgsFor system).nixpkgs-fmt
      );
    };
}
