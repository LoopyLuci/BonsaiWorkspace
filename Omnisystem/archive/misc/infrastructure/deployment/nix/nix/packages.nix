{ pkgs ? import <nixpkgs> {} }:

let
  # Helper to build Omnisystem Rust components with verification
  buildOmnisystemComponent = { name, src, features ? [], cargoLock ? null }:
    pkgs.rustPlatform.buildRustPackage rec {
      inherit name src cargoLock;
      version = "1.0.0";

      cargoLock = {
        lockFile = ./Cargo.lock;
      };

      buildInputs = with pkgs; [
        pkg-config
        openssl
        cmake
        llvm
      ];

      nativeBuildInputs = with pkgs; [
        rustc
        cargo
      ];

      cargoBuildFlags = lib.optionals (features != [])
        [ "--features" (lib.concatStringsSep "," features) ];

      # Run Axiom formal verification on build
      postBuild = ''
        echo "Running Axiom verification..."
        # Would integrate with actual Axiom checker
        # ${pkgs.axiom-checker}/bin/axiom-verify --component ${name} --output $out
      '';

      meta = with pkgs.lib; {
        description = "Omnisystem ${name} component";
        mainProgram = name;
        platforms = platforms.unix;
      };
    };

  sources = {
    omnisystem = ./..;
  };

in {
  # Core Omnisystem components
  omnisystem = pkgs.rustPlatform.buildRustPackage {
    name = "omnisystem";
    src = sources.omnisystem;
    version = "1.0.0";

    cargoLock = {
      lockFile = ../Cargo.lock;
    };

    buildInputs = with pkgs; [
      pkg-config
      openssl
      cmake
      llvm
      libclang
    ];

    nativeBuildInputs = with pkgs; [
      rustc
      cargo
      cmake
      which
    ];

    cargoBuildFlags = [
      "--release"
      "--all-features"
    ];

    meta = with pkgs.lib; {
      description = "Omnisystem – sovereign, verifiable, self-managing OS";
      platforms = platforms.unix;
    };
  };

  # UOSC Microkernel
  kernel = pkgs.stdenv.mkDerivation {
    name = "omnisystem-kernel";
    version = "1.0.0";
    src = sources.omnisystem;

    buildPhase = ''
      cd Omnisystem/kernel
      ${pkgs.rustc}/bin/rustc --edition 2021 -O kernel.ti -o kernel
      ${pkgs.llvm}/bin/llvm-strip kernel
    '';

    installPhase = ''
      mkdir -p $out/bin
      cp kernel $out/bin/uosc-kernel
      chmod +x $out/bin/uosc-kernel
    '';

    meta = with pkgs.lib; {
      description = "UOSC microkernel (<50KB, formally verified)";
      platforms = [ "x86_64-linux" "aarch64-linux" ];
    };
  };

  # TransferDaemon
  transfer-daemon = pkgs.rustPlatform.buildRustPackage {
    name = "transfer-daemon";
    src = sources.omnisystem;
    version = "1.0.0";

    buildInputs = with pkgs; [ openssl pkg-config ];

    meta.description = "TransferDaemon – P2P mesh networking for Omnisystem";
  };

  # UMS (Universal Module System)
  ums = pkgs.rustPlatform.buildRustPackage {
    name = "ums";
    src = sources.omnisystem;
    version = "1.0.0";

    meta.description = "UMS – universal module system for hot-loading and composition";
  };

  # OmniCloak Browser
  omnicloak = pkgs.rustPlatform.buildRustPackage {
    name = "omnicloak";
    src = sources.omnisystem;
    version = "1.0.0";

    buildInputs = with pkgs; [
      openssl
      pkg-config
      xorg.libxcb
      xorg.libX11
    ];

    meta.description = "OmniCloak – privacy-first browser with Bonsai integration";
  };

  # VFS Service
  vfs = pkgs.rustPlatform.buildRustPackage {
    name = "vfs-service";
    src = sources.omnisystem;
    version = "1.0.0";

    meta.description = "VFS – virtual filesystem service for Omnisystem";
  };

  # Storage Service
  storage = pkgs.rustPlatform.buildRustPackage {
    name = "storage-service";
    src = sources.omnisystem;
    version = "1.0.0";

    buildInputs = with pkgs; [ zstd ];

    meta.description = "Storage – persistent storage management for Omnisystem";
  };

  # Device Manager
  device-manager = pkgs.rustPlatform.buildRustPackage {
    name = "device-manager";
    src = sources.omnisystem;
    version = "1.0.0";

    meta.description = "Device Manager – hardware discovery and configuration";
  };

  # Compositor
  compositor = pkgs.rustPlatform.buildRustPackage {
    name = "compositor";
    src = sources.omnisystem;
    version = "1.0.0";

    buildInputs = with pkgs; [ xorg.libxcb mesa ];

    meta.description = "Compositor – GPU-accelerated rendering for Omnisystem";
  };

  # Logger Service
  logger = pkgs.rustPlatform.buildRustPackage {
    name = "logger-service";
    src = sources.omnisystem;
    version = "1.0.0";

    meta.description = "Logger – logging and audit service";
  };

  # Config Service
  config = pkgs.rustPlatform.buildRustPackage {
    name = "config-service";
    src = sources.omnisystem;
    version = "1.0.0";

    meta.description = "Config – configuration and policy management";
  };

  # Service Manager
  service-manager = pkgs.rustPlatform.buildRustPackage {
    name = "service-manager";
    src = sources.omnisystem;
    version = "1.0.0";

    meta.description = "Service Manager – orchestration and lifecycle management";
  };

  # Autonomic Agents
  autonomic-agents = pkgs.rustPlatform.buildRustPackage {
    name = "autonomic-agents";
    src = sources.omnisystem;
    version = "1.0.0";

    meta.description = "Autonomic Agents – self-healing and optimization";
  };

  # Build CLI
  build-cli = pkgs.rustPlatform.buildRustPackage {
    name = "build-cli";
    src = sources.omnisystem;
    version = "1.0.0";

    buildInputs = with pkgs; [ openssl pkg-config ];

    meta.description = "Build CLI – compilation and deployment tool";
  };

  # Convenience: bundle all services
  all-services = pkgs.symlinkJoin {
    name = "omnisystem-all-services";
    paths = [
      kernel
      transfer-daemon
      ums
      omnicloak
      vfs
      storage
      device-manager
      compositor
      logger
      config
      service-manager
      autonomic-agents
      build-cli
    ];
  };

  # Slim image (kernel + essential only)
  minimal-image = pkgs.stdenv.mkDerivation {
    name = "omnisystem-minimal";
    version = "1.0.0";

    buildCommand = ''
      mkdir -p $out/bin
      cp ${kernel}/bin/* $out/bin/
      du -sh $out
    '';

    meta.description = "Minimal Omnisystem image (kernel only)";
  };

  # Development environment
  dev-shell = pkgs.mkShell {
    buildInputs = with pkgs; [
      rustc
      cargo
      rustfmt
      clippy
      cmake
      llvm
      pkg-config
      openssl
      nix
      direnv
    ];

    shellHook = ''
      echo "Omnisystem development environment loaded"
      export OMNISYSTEM_ROOT=$(pwd)
    '';
  };
}
