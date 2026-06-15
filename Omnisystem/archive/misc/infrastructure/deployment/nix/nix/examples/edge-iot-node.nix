# Edge IoT Node Configuration
# Minimal footprint (128MB RAM) for Raspberry Pi and similar edge devices
# Usage: nixos-rebuild switch -I nixos-config=./edge-iot-node.nix

{ config, lib, pkgs, ... }:

{
  imports = [
    # For actual use, import from flake:
    # ../modules/omnisystem.nix
  ];

  # System identification
  networking = {
    hostName = "omnisystem-edge-01";
    useDHCP = true;
  };

  # Time zone
  time.timeZone = "UTC";

  # Minimal locale
  i18n.defaultLocale = "en_US.UTF-8";

  # Omnisystem configuration – IoT Edge Node
  services.omnisystem = {
    enable = true;

    # Hosted-light: minimal footprint on Linux host
    mode = "hosted-light";

    # Minimal service set
    services = {
      kernel = true;
      transfer-daemon = true;
      device-manager = true;
      logger = false;  # Disable logging to save space
    };

    # Low resource allocation
    adapter = {
      memory_mb = 128;    # Minimal: 128MB
      cpus = 1;
      debug = false;
    };

    # Limited capabilities (IoT sensor)
    capabilities = [
      "net-access"
      "device-gpio"
      "device-uart"
      "device-i2c"
      "sensor-read"
    ];

    # Autonomic management on minimal level
    autonomic = {
      enable = true;

      healthCheck = {
        enable = true;
        interval = "30s";  # Less frequent on edge
      };

      performanceOptimization.enable = false;
      securityAuditing.enable = false;
      failureDetection = {
        enable = true;
        restartPolicy = "delayed";  # Gentle restart
      };
    };

    # No compatibility layers (embedded Linux focus)
    compatibility = {
      posix.enable = true;
      windows.enable = false;
      legacyDrivers.enable = false;
    };

    # Minimal persistence
    persistence = {
      stateDirectory = /var/lib/omnisystem;
      survivalSystem = {
        enable = false;  # Disable for space
      };
    };

    # Mesh networking for edge devices
    networking = {
      meshEnabled = true;
      regions = [ "Edge" ];
      peers = [
        "omnisystem-hub.local:9000"
      ];
    };

    governance = {
      councilKeys = [ ];  # Standalone edge nodes
      thresholdSignatures = 0;
      policyUpdateFrequency = "30d";
    };
  };

  # Minimize rootfs
  environment.noXlibs = true;

  # Essential system packages only
  environment.systemPackages = with pkgs; [
    busybox
    curl
    htop
  ];

  # SSH for remote management
  services.openssh = {
    enable = true;
    permitRootLogin = "no";
    passwordAuthentication = false;
    ports = [ 22 ];
  };

  # Firewall
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 9000 ];
    allowedUDPPorts = [ 9000 ];
  };

  # User for remote management
  users.users.edge = {
    isNormalUser = true;
    home = "/home/edge";
    openssh.authorizedKeys.keys = [
      # Add SSH key for remote management
    ];
  };

  # Root read-only (safer on edge devices)
  fileSystems = {
    "/" = {
      fsType = "ext4";
      options = [ "ro" "noatime" ];
    };
    "/var" = {
      fsType = "tmpfs";
      options = [ "size=32M" "mode=755" ];
    };
    "/tmp" = {
      fsType = "tmpfs";
      options = [ "size=16M" "mode=1777" ];
    };
  };

  # Disable unnecessary services
  services.udisks2.enable = false;
  services.fstrim.enable = false;
  documentation.enable = false;
  documentation.man.enable = false;

  # Nix configuration
  nix = {
    package = pkgs.nix;
    settings = {
      auto-optimise-store = true;
      gc = {
        automatic = true;
        dates = "weekly";
        options = "--delete-older-than 30d";
      };
    };
  };

  # Systemd optimization for IoT
  systemd = {
    services = {
      # Reduce verbosity for edge
      systemd-journald.serviceConfig.StandardOutput = "null";
    };
  };

  # System state version
  system.stateVersion = "24.05";
}
