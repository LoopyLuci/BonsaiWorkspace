# Developer Workstation Configuration
# Full Omnisystem integration with all services, autonomic management, and compatibility layers
# Usage: nixos-rebuild switch -I nixos-config=./developer-workstation.nix

{ config, lib, pkgs, ... }:

{
  imports = [
    # For actual use, import from flake:
    # ../modules/omnisystem.nix
    # ../modules/autonomic.nix
    # ../modules/compatibility.nix
  ];

  # System identification
  networking = {
    hostName = "omnisystem-dev";
    networkmanager.enable = true;
  };

  # Time zone
  time.timeZone = "UTC";

  # Locale
  i18n.defaultLocale = "en_US.UTF-8";

  # Display server
  services.xserver = {
    enable = true;
    desktopManager.gnome.enable = true;
    displayManager.gdm.enable = true;
  };

  # Sound
  sound.enable = true;
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    pulse.enable = true;
  };

  # Omnisystem configuration – Developer workstation (full integration)
  services.omnisystem = {
    enable = true;

    # Hosted-full: QEMU VM with seamless integration
    mode = "hosted-full";

    # Enable all services
    services = {
      kernel = true;
      init = true;
      transfer-daemon = true;
      ums = true;
      omnicloak = true;
      vfs = true;
      net-stack = true;
      storage = true;
      device-manager = true;
      display = true;
      compositor = true;
      logger = true;
      config = true;
      service-manager = true;
    };

    # Adapter configuration: 4 CPUs, 4GB RAM, GPU passthrough
    adapter = {
      memory_mb = 4096;
      cpus = 4;
      gpu_passthrough = true;
      debug = false;
    };

    # All capabilities (developer has full access)
    capabilities = [
      "net-access"
      "storage-full"
      "device-usb"
      "device-pci"
      "graphics"
      "audio"
      "kvm"
    ];

    # Autonomic management enabled
    autonomic = {
      enable = true;

      healthCheck = {
        enable = true;
        interval = "10s";
      };

      performanceOptimization = {
        enable = true;
        enablePredictiveLoading = true;  # Use AI for smart prefetch
      };

      securityAuditing = {
        enable = true;
        scanInterval = "1h";
      };

      failureDetection = {
        enable = true;
        restartPolicy = "immediate";
      };
    };

    # Compatibility layers
    compatibility = {
      posix.enable = true;
      windows.enable = false;
      legacyDrivers.enable = false;
    };

    # Persistence
    persistence = {
      stateDirectory = /var/lib/omnisystem;
      survivalSystem = {
        enable = true;
        logPath = /var/log/omnisystem/survival;
      };
    };

    # P2P mesh networking
    networking = {
      meshEnabled = true;
      regions = [ "Americas" ];
      peers = [
        "omnisystem-node1.local:9000"
        "omnisystem-node2.local:9000"
      ];
    };

    # Optional: Regional council integration
    governance = {
      councilKeys = [ ];  # Leave empty for standalone
      thresholdSignatures = 5;
      policyUpdateFrequency = "7d";
    };
  };

  # Virtualization support
  virtualisation = {
    libvirtd.enable = true;
    qemu.enable = true;
  };

  # Development tools
  environment.systemPackages = with pkgs; [
    git
    vim
    neovim
    gcc
    cmake
    rustup
    cargo
    nodejs
    python3
    docker
    kubectl
    nix
    direnv
  ];

  # SSH
  services.openssh = {
    enable = true;
    permitRootLogin = "no";
    passwordAuthentication = false;
  };

  # Firewall
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 80 443 9000 ];
    allowedUDPPorts = [ 9000 ];
  };

  # User account
  users.users.dev = {
    isNormalUser = true;
    home = "/home/dev";
    extraGroups = [ "wheel" "docker" "libvirtd" ];
    openssh.authorizedKeys.keys = [
      # Add your SSH public key here
    ];
  };

  # Nix configuration
  nix = {
    package = pkgs.nix;
    settings = {
      experimental-features = [ "nix-command" "flakes" ];
      auto-optimise-store = true;
    };
  };

  # System state version
  system.stateVersion = "24.05";
}
