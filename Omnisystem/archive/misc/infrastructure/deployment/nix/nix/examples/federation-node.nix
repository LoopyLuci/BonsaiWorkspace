# Multi-Region Federation Node Configuration
# Deployed in multiple regions with federated governance and council participation
# Usage: nixos-rebuild switch -I nixos-config=./federation-node.nix

{ config, lib, pkgs, ... }:

{
  imports = [
    # For actual use, import from flake:
    # ../modules/omnisystem.nix
    # ../modules/autonomic.nix
    # ../modules/governance.nix
  ];

  # System identification (region-aware)
  networking = {
    hostName = "omnisystem-federation-eu-01";
    networkmanager.enable = true;
  };

  # Time zone (regional)
  time.timeZone = "Europe/Berlin";

  # Locale
  i18n.defaultLocale = "en_US.UTF-8";

  # Omnisystem configuration – Federation Node (governance enabled)
  services.omnisystem = {
    enable = true;

    # Hosted-full: full integration with local infrastructure
    mode = "hosted-full";

    # Full service set for federation nodes
    services = {
      kernel = true;
      init = true;
      transfer-daemon = true;
      ums = true;
      vfs = true;
      net-stack = true;
      storage = true;
      device-manager = true;
      logger = true;
      config = true;
      service-manager = true;
    };

    # Reasonable resource allocation
    adapter = {
      memory_mb = 2048;
      cpus = 4;
      gpu_passthrough = false;
      debug = false;
    };

    # Federation-level capabilities
    capabilities = [
      "net-access"
      "storage-full"
      "device-network"
      "policy-voting"
      "service-orchestration"
      "global-mesh"
    ];

    # Full autonomic management for production
    autonomic = {
      enable = true;

      healthCheck = {
        enable = true;
        interval = "15s";
      };

      performanceOptimization = {
        enable = true;
        enablePredictiveLoading = false;
      };

      securityAuditing = {
        enable = true;
        scanInterval = "30m";
      };

      failureDetection = {
        enable = true;
        restartPolicy = "immediate";
      };
    };

    # Minimal compatibility (production Linux focus)
    compatibility = {
      posix.enable = true;
      windows.enable = false;
      legacyDrivers.enable = false;
    };

    # Full persistence with survival system
    persistence = {
      stateDirectory = /var/lib/omnisystem;
      survivalSystem = {
        enable = true;
        logPath = /var/log/omnisystem/survival;
      };
    };

    # Full mesh networking
    networking = {
      meshEnabled = true;
      regions = [
        "Europe"
        "Americas"
      ];
      peers = [
        "omnisystem-eu-hub.local:9000"
        "omnisystem-us-hub.local:9000"
        "omnisystem-ap-hub.local:9000"
      ];
    };

    # ============================================
    # GOVERNANCE CONFIGURATION (CRITICAL)
    # ============================================
    governance = {
      # Europe Council BLS public keys
      councilKeys = [
        # Americas Council (5 members)
        "americas-council-member-1-bls-key"
        "americas-council-member-2-bls-key"
        "americas-council-member-3-bls-key"
        "americas-council-member-4-bls-key"
        "americas-council-member-5-bls-key"

        # Europe Council (5 members)
        "europe-council-member-1-bls-key"
        "europe-council-member-2-bls-key"
        "europe-council-member-3-bls-key"
        "europe-council-member-4-bls-key"
        "europe-council-member-5-bls-key"

        # APAC Council (5 members)
        "apac-council-member-1-bls-key"
        "apac-council-member-2-bls-key"
        "apac-council-member-3-bls-key"
        "apac-council-member-4-bls-key"
        "apac-council-member-5-bls-key"

        # Africa Council (3 members)
        "africa-council-member-1-bls-key"
        "africa-council-member-2-bls-key"
        "africa-council-member-3-bls-key"

        # Middle East Council (3 members)
        "middleeast-council-member-1-bls-key"
        "middleeast-council-member-2-bls-key"
        "middleeast-council-member-3-bls-key"

        # Oceania Council (3 members)
        "oceania-council-member-1-bls-key"
        "oceania-council-member-2-bls-key"
        "oceania-council-member-3-bls-key"

        # Global Council Arbiters (3 members)
        "global-arbiter-1-bls-key"
        "global-arbiter-2-bls-key"
        "global-arbiter-3-bls-key"
      ];

      # 5-of-7 councils required for global policy changes
      thresholdSignatures = 5;

      # Check for new governance policies weekly
      policyUpdateFrequency = "7d";
    };
  };

  # Virtualization for federation node
  virtualisation = {
    libvirtd.enable = true;
    qemu.enable = true;
  };

  # Core utilities
  environment.systemPackages = with pkgs; [
    git
    curl
    htop
    iotop
    nethogs
    tmux
  ];

  # SSH with certificate-based auth (governance-signed)
  services.openssh = {
    enable = true;
    permitRootLogin = "no";
    passwordAuthentication = false;
    ports = [ 22 ];
    pubkeyAuthentication = true;
  };

  # Firewall – mesh + governance + standard services
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [
      22      # SSH
      9000    # Omnisystem mesh
      9001    # Governance voting
      9002    # Council sync
    ];
    allowedUDPPorts = [
      9000    # Omnisystem mesh
      9001    # Governance
    ];
  };

  # Monitoring user (for federation observability)
  users.users.omniadmin = {
    isNormalUser = true;
    home = "/home/omniadmin";
    openssh.authorizedKeys.keys = [
      # Federation admin key
    ];
  };

  # Systemd hardening for production
  systemd.services = {
    omnisystem-kernel.serviceConfig = {
      PrivateTmp = true;
      NoNewPrivileges = true;
      ReadWritePaths = [ "/var/lib/omnisystem" "/var/log/omnisystem" ];
    };
  };

  # Logging (persistent, structured)
  services.journald = {
    extraConfig = ''
      Storage=persistent
      MaxRetentionSec=90d
      MaxDiskSize=10G
      Compress=yes
    '';
  };

  # Nix configuration
  nix = {
    package = pkgs.nix;
    settings = {
      experimental-features = [ "nix-command" "flakes" ];
      auto-optimise-store = true;
      gc = {
        automatic = true;
        dates = "weekly";
        options = "--delete-older-than 30d";
      };
    };
  };

  # Security hardening for federation
  security = {
    apparmor.enable = true;
    apparmor.enforceMode = "complain";  # Start in complain mode
    lockKernelModules = true;
    protectKernelLogs = true;
    restrict-eval = false;  # NixOS evaluation
  };

  # System state version
  system.stateVersion = "24.05";

  # ============================================
  # CRITICAL NOTES FOR FEDERATION OPERATORS
  # ============================================
  #
  # 1. Council Keys Management:
  #    - Keys above are EXAMPLES. Replace with actual BLS public keys.
  #    - Rotate keys annually (1-year council terms).
  #    - Never share council private keys (stored securely offline).
  #
  # 2. Governance Voting:
  #    - Policy changes require 5-of-7 councils to sign (BLS threshold).
  #    - Global Arbiters break ties if deadlock occurs.
  #    - All votes recorded in immutable audit trail.
  #
  # 3. Regional Autonomy:
  #    - Each council governs its region independently.
  #    - Global decisions (e.g., kernel updates) require consensus.
  #    - No central authority can override regional councils.
  #
  # 4. Network Security:
  #    - Governance port (9001) should be restricted to council members.
  #    - Consider using VPN overlay for council communications.
  #    - Verify council key authenticity out-of-band before first deployment.
  #
  # 5. Disaster Recovery:
  #    - Survival System logs all state changes to immutable journal.
  #    - In case of compromise, can replay logs to recover state.
  #    - Council approval required for state rollback.
}
