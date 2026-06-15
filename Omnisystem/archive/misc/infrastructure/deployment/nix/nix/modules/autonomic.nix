{ config, pkgs, lib, ... }:

# Autonomic Management Module – Self-healing, optimization, security
# Optional: add to nixosConfigurations modules list for full autonomic features

let
  cfg = config.services.omnisystem;

in {
  config = lib.mkIf (cfg.enable && cfg.autonomic.enable) {
    systemd.services = {
      omnisystem-performance-optimizer = lib.mkIf cfg.autonomic.performanceOptimization.enable {
        description = "Omnisystem Performance Optimizer Agent";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/performance-optimizer";
          Restart = "on-failure";
          RestartSec = 5;
          StandardOutput = "journal";
          StandardError = "journal";
          Environment = lib.optionals cfg.autonomic.performanceOptimization.enablePredictiveLoading
            ["ENABLE_PREDICTIVE_LOAD=1"];
        };
      };

      omnisystem-resource-scaler = {
        description = "Omnisystem Resource Scaler Agent";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/resource-scaler";
          Restart = "on-failure";
          RestartSec = 5;
          StandardOutput = "journal";
          StandardError = "journal";
        };
      };

      omnisystem-policy-enforcer = {
        description = "Omnisystem Policy Enforcer Agent";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/policy-enforcer";
          Restart = "on-failure";
          RestartSec = 5;
          StandardOutput = "journal";
          StandardError = "journal";
        };
      };
    };

    # Governance integration (if councils configured)
    systemd.services = lib.mkIf (cfg.governance.councilKeys != []) {
      omnisystem-governance-daemon = {
        description = "Omnisystem Governance Policy Sync";
        wantedBy = [ "multi-user.target" ];
        after = [ "network-online.target" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/governance-daemon";
          Restart = "on-failure";
          RestartSec = 60;
          StandardOutput = "journal";
          StandardError = "journal";
          Environment = [
            "POLICY_UPDATE_FREQUENCY=${cfg.governance.policyUpdateFrequency}"
            "THRESHOLD_SIGNATURES=${toString cfg.governance.thresholdSignatures}"
          ];
        };
      };
    };

    # Survival System (transaction logging + recovery)
    systemd.services = lib.mkIf cfg.persistence.survivalSystem.enable {
      omnisystem-survival-system = {
        description = "Omnisystem Survival System (Recovery Log)";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/survival-system";
          Restart = "on-failure";
          RestartSec = 5;
          StandardOutput = "journal";
          StandardError = "journal";
          StateDirectory = "omnisystem/survival";
        };
      };
    };

    # Timer-based health audits
    systemd.timers = {
      omnisystem-health-audit = lib.mkIf cfg.autonomic.healthCheck.enable {
        wantedBy = [ "timers.target" ];
        timerConfig = {
          OnBootSec = cfg.autonomic.healthCheck.interval;
          OnUnitActiveSec = cfg.autonomic.healthCheck.interval;
          AccuracySec = "1min";
        };
        unitConfig = {
          Description = "Periodic Omnisystem Health Audit";
        };
      };

      omnisystem-security-scan = lib.mkIf cfg.autonomic.securityAuditing.enable {
        wantedBy = [ "timers.target" ];
        timerConfig = {
          OnBootSec = "10min";
          OnUnitActiveSec = cfg.autonomic.securityAuditing.scanInterval;
          AccuracySec = "5min";
        };
        unitConfig = {
          Description = "Periodic Omnisystem Security Scan";
        };
      };
    };

    # Logging for autonomic operations
    services.rsyslog = {
      extraConfig = ''
        if $programname startswith "omnisystem-" then /var/log/omnisystem.log
        & ~
      '';
    };

    environment.etc."omnisystem/governance.conf".text = lib.optionalString (cfg.governance.councilKeys != [])
      (builtins.toJSON {
        councils = cfg.governance.councilKeys;
        threshold = cfg.governance.thresholdSignatures;
        updateFrequency = cfg.governance.policyUpdateFrequency;
      });
  };
}
