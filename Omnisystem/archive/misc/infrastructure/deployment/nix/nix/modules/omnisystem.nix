{ config, pkgs, lib, ... }:

let
  cfg = config.services.omnisystem;
  omnisystemPkgs = import ../packages.nix { inherit pkgs; };

in {
  options.services.omnisystem = with lib; {
    enable = mkEnableOption "Omnisystem sovereign OS integration";

    mode = mkOption {
      type = types.enum [ "hosted-light" "hosted-full" "bare-metal" "library-os" ];
      default = "hosted-light";
      description = "Deployment mode: hosted-light (NixOS systemd), hosted-full (QEMU), bare-metal (standalone), or library-os (linked library)";
    };

    services = mkOption {
      type = types.submodule {
        freeformType = types.attrsOf types.bool;
        options = {
          transfer-daemon = mkEnableOption "TransferDaemon P2P mesh service" // { default = false; };
          ums = mkEnableOption "Universal Module System service" // { default = false; };
          omnicloak = mkEnableOption "OmniCloak secure browser" // { default = false; };
          vfs = mkEnableOption "Virtual filesystem service" // { default = false; };
          net-stack = mkEnableOption "Network stack service" // { default = false; };
          storage = mkEnableOption "Storage service" // { default = false; };
          device-manager = mkEnableOption "Device manager service" // { default = false; };
          display = mkEnableOption "Display/rendering service" // { default = false; };
          compositor = mkEnableOption "Compositor (desktop environment)" // { default = false; };
          logger = mkEnableOption "Logging service" // { default = false; };
          config = mkEnableOption "Configuration service" // { default = false; };
          service-manager = mkEnableOption "Service manager" // { default = false; };
          kernel = mkEnableOption "UOSC kernel" // { default = false; };
          init = mkEnableOption "Init system" // { default = false; };
        };
      };
      default = {};
      description = "Individual service toggles";
    };

    adapter = {
      memory_mb = mkOption {
        type = types.int;
        default = 512;
        description = "Memory allocated to Omnisystem (hosted modes only)";
      };

      cpus = mkOption {
        type = types.int;
        default = 2;
        description = "Virtual CPUs (hosted modes only)";
      };

      gpu_passthrough = mkOption {
        type = types.bool;
        default = false;
        description = "Enable GPU passthrough (hosted-full only)";
      };

      debug = mkOption {
        type = types.bool;
        default = false;
        description = "Enable debug logging";
      };
    };

    capabilities = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "List of capability tokens to grant the system";
      example = [ "net-access" "storage-rw" "device-usb" ];
    };

    governance = {
      councilKeys = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "BLS public keys of regional councils";
      };

      thresholdSignatures = mkOption {
        type = types.int;
        default = 5;
        description = "Number of council signatures required for policy updates";
      };

      policyUpdateFrequency = mkOption {
        type = types.str;
        default = "7d";
        description = "How often to check for governance updates";
      };
    };

    autonomic = {
      enable = mkEnableOption "Autonomous management agents (self-healing, optimization, security)";

      healthCheck = {
        enable = mkEnableOption "Health monitoring agent" // { default = true; };
        interval = mkOption {
          type = types.str;
          default = "10s";
          description = "Health check interval";
        };
      };

      performanceOptimization = {
        enable = mkEnableOption "Performance optimizer agent" // { default = true; };
        enablePredictiveLoading = mkOption {
          type = types.bool;
          default = false;
          description = "Enable AI-powered predictive load optimization (optional)";
        };
      };

      securityAuditing = {
        enable = mkEnableOption "Security auditor agent" // { default = true; };
        scanInterval = mkOption {
          type = types.str;
          default = "1h";
          description = "How often to scan for security anomalies";
        };
      };

      failureDetection = {
        enable = mkEnableOption "Failure detector and recovery agent" // { default = true; };
        restartPolicy = mkOption {
          type = types.enum [ "immediate" "delayed" "manual" ];
          default = "immediate";
          description = "How to handle crashed services";
        };
      };
    };

    compatibility = {
      posix = {
        enable = mkEnableOption "POSIX compatibility layer" // { default = true; };
      };

      windows = {
        enable = mkEnableOption "Windows ABI emulation" // { default = false; };
      };

      legacyDrivers = {
        enable = mkEnableOption "Legacy driver wrappers" // { default = false; };
      };
    };

    persistence = {
      stateDirectory = mkOption {
        type = types.path;
        default = /var/lib/omnisystem;
        description = "Where to store persistent state";
      };

      survivalSystem = {
        enable = mkEnableOption "Transaction logging and automatic recovery" // { default = true; };
        logPath = mkOption {
          type = types.path;
          default = /var/log/omnisystem/survival;
          description = "Where to store survival logs";
        };
      };
    };

    networking = {
      meshEnabled = mkEnableOption "P2P mesh networking" // { default = true; };

      regions = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Regional affiliations for federation (e.g., ['Americas', 'Europe'])";
        example = [ "Americas" "Europe" ];
      };

      peers = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "Bootstrap peer addresses";
        example = [ "peer1.omnisystem.io:9000" "peer2.omnisystem.io:9000" ];
      };
    };
  };

  config = lib.mkIf cfg.enable {
    # Hosted-light: services run as NixOS systemd units
    systemd.services = lib.mkIf (cfg.mode == "hosted-light") {
      omnisystem-kernel = lib.mkIf cfg.services.kernel {
        description = "UOSC Omnisystem Kernel";
        wantedBy = [ "multi-user.target" ];
        after = [ "network.target" ];
        serviceConfig = {
          Type = "forking";
          ExecStart = "${omnisystemPkgs.kernel}/bin/uosc-kernel";
          Restart = "on-failure";
          RestartSec = 5;
        };
      };

      omnisystem-transfer-daemon = lib.mkIf cfg.services.transfer-daemon {
        description = "TransferDaemon P2P Mesh";
        wantedBy = [ "multi-user.target" ];
        after = [ "network.target" "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${omnisystemPkgs.transfer-daemon}/bin/transfer-daemon";
          Restart = "on-failure";
          Environment = [
            "OMNISYSTEM_MODE=hosted-light"
            "OMNISYSTEM_DEBUG=${lib.boolToString cfg.adapter.debug}"
          ] ++ (lib.optionals (cfg.networking.regions != [])
            ["OMNISYSTEM_REGIONS=${lib.concatStringsSep "," cfg.networking.regions}"]
          );
        };
      };

      omnisystem-ums = lib.mkIf cfg.services.ums {
        description = "Universal Module System";
        wantedBy = [ "multi-user.target" ];
        after = [ "network.target" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${omnisystemPkgs.ums}/bin/ums";
          Restart = "on-failure";
        };
      };

      omnisystem-omnicloak = lib.mkIf cfg.services.omnicloak {
        description = "OmniCloak Secure Browser";
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${omnisystemPkgs.omnicloak}/bin/omnicloak";
          Restart = "on-failure";
        };
      };

      omnisystem-health-monitor = lib.mkIf cfg.autonomic.healthCheck.enable {
        description = "Omnisystem Health Monitor Agent";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${omnisystemPkgs.autonomic-agents}/bin/health-monitor";
          StartLimitInterval = 60;
          StartLimitBurst = 3;
        };
      };

      omnisystem-failure-detector = lib.mkIf cfg.autonomic.failureDetection.enable {
        description = "Omnisystem Failure Detector Agent";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${omnisystemPkgs.autonomic-agents}/bin/failure-detector";
          StartLimitInterval = 60;
          StartLimitBurst = 3;
        };
      };

      omnisystem-security-auditor = lib.mkIf cfg.autonomic.securityAuditing.enable {
        description = "Omnisystem Security Auditor Agent";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${omnisystemPkgs.autonomic-agents}/bin/security-auditor";
          Environment = "SCAN_INTERVAL=${cfg.autonomic.securityAuditing.scanInterval}";
        };
      };
    };

    # Hosted-full: QEMU VM with virtio devices
    virtualisation = lib.mkIf (cfg.mode == "hosted-full") {
      qemu.enable = true;
    };

    # Persistence directories
    systemd.tmpfiles.rules = lib.optionals cfg.persistence.survivalSystem.enable [
      "d '${cfg.persistence.stateDirectory}' 0700 - - -"
      "d '${cfg.persistence.stateDirectory}/modules' 0700 - - -"
      "d '${cfg.persistence.survivalSystem.logPath}' 0700 - - -"
    ];

    # Packages
    environment.systemPackages = lib.optionals (cfg.mode == "hosted-light") [
      omnisystemPkgs.omnisystem
      omnisystemPkgs.build-cli
    ] ++ lib.optionals cfg.services.omnicloak [
      omnisystemPkgs.omnicloak
    ];

    # Enable security features based on threat model
    security = {
      apparmor = {
        enable = lib.mkDefault true;
      };

      lockKernelModules = lib.mkDefault true;

      protectKernelLogs = lib.mkDefault true;
    };

    # Kernel hardening (hosted-light on Linux)
    boot = lib.mkIf (cfg.mode == "hosted-light" && pkgs.stdenv.isLinux) {
      kernelParams = [
        "slub_debug=P"
        "init_on_free=1"
        "init_on_alloc=1"
      ];
    };

    # Logging
    services.journald = lib.mkIf cfg.autonomic.enable {
      extraConfig = ''
        MaxRetentionSec=30d
        Storage=persistent
      '';
    };

    # Simple service orchestration for hosted-light
    systemd.services = lib.mkIf (cfg.mode == "hosted-light") (
      let
        makeServiceDependency = serviceName: {
          "omnisystem-${serviceName}" = lib.mkIf (lib.attrByPath [serviceName] false cfg.services) {
            wantedBy = [ "multi-user.target" ];
            after = [ "omnisystem-kernel.service" ];
            serviceConfig = {
              Type = "simple";
              ExecStart = "${omnisystemPkgs."${serviceName}" or pkgs.coreutils}/bin/${serviceName}";
              Restart = "on-failure";
              RestartSec = 5;
              StandardOutput = "journal";
              StandardError = "journal";
            };
          };
        };
      in
        lib.mkMerge [
          (makeServiceDependency "vfs")
          (makeServiceDependency "storage")
          (makeServiceDependency "net-stack")
          (makeServiceDependency "device-manager")
          (makeServiceDependency "display")
          (makeServiceDependency "compositor")
          (makeServiceDependency "logger")
          (makeServiceDependency "config")
        ]
    );

    # Networking for mesh
    networking = lib.mkIf cfg.networking.meshEnabled {
      firewall.allowedTCPPorts = [ 9000 ];
      firewall.allowedUDPPorts = [ 9000 ];
    };

    # State preservation
    system.activationScripts.omnisystem = lib.mkIf cfg.persistence.survivalSystem.enable ''
      mkdir -p '${cfg.persistence.stateDirectory}'
      mkdir -p '${cfg.persistence.stateDirectory}/modules'
      mkdir -p '${cfg.persistence.survivalSystem.logPath}'
      chmod 0700 '${cfg.persistence.stateDirectory}'
    '';

    # Warnings for unsupported configurations
    warnings = (lib.optional (cfg.mode == "bare-metal")
      "Omnisystem bare-metal mode requires custom ISO build (not automatic in NixOS)")
    ++ (lib.optional (cfg.mode == "hosted-full")
      "Omnisystem hosted-full mode requires QEMU setup");
  };

  meta = {
    doc = ./omnisystem.md;
    maintainers = with lib.maintainers; [];
  };
}
