{ config, pkgs, lib, ... }:

# Governance Module – Federated Council Integration
# Optional: add for full governance features with BLS voting and policy enforcement

let
  cfg = config.services.omnisystem;

in {
  config = lib.mkIf (cfg.enable && cfg.governance.councilKeys != []) {
    systemd.services = {
      omnisystem-governance-sync = {
        description = "Omnisystem Governance Policy Synchronization";
        wantedBy = [ "multi-user.target" ];
        after = [ "network-online.target" "omnisystem-transfer-daemon.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/governance-sync";
          Restart = "on-failure";
          RestartSec = 30;
          StandardOutput = "journal";
          StandardError = "journal";
        };
      };

      omnisystem-bls-verifier = {
        description = "Omnisystem BLS Threshold Signature Verifier";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/bls-verifier";
          Restart = "on-failure";
          StandardOutput = "journal";
          StandardError = "journal";
        };
      };

      omnisystem-council-agent = {
        description = "Omnisystem Regional Council Agent";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" "omnisystem-transfer-daemon.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/council-agent";
          Restart = "on-failure";
          RestartSec = 10;
          StandardOutput = "journal";
          StandardError = "journal";
          Environment = [
            "COUNCIL_THRESHOLD=${toString cfg.governance.thresholdSignatures}"
            "POLICY_UPDATE_FREQ=${cfg.governance.policyUpdateFrequency}"
          ];
        };
      };

      omnisystem-audit-logger = {
        description = "Omnisystem Governance Audit Trail";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-governance-sync.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/audit-logger";
          Restart = "on-failure";
          StandardOutput = "journal";
          StandardError = "journal";
          StateDirectory = "omnisystem/governance";
        };
      };
    };

    # Council key configuration
    environment.etc."omnisystem/councils.conf".text = lib.optionalString (cfg.governance.councilKeys != [])
      (builtins.toJSON {
        version = "1.0";
        councils = {
          americas = {
            keys = lib.optional (lib.any (k: lib.hasPrefix "americas-" k) cfg.governance.councilKeys)
              (lib.filter (k: lib.hasPrefix "americas-" k) cfg.governance.councilKeys);
          };
          europe = {
            keys = lib.optional (lib.any (k: lib.hasPrefix "europe-" k) cfg.governance.councilKeys)
              (lib.filter (k: lib.hasPrefix "europe-" k) cfg.governance.councilKeys);
          };
          apac = {
            keys = lib.optional (lib.any (k: lib.hasPrefix "apac-" k) cfg.governance.councilKeys)
              (lib.filter (k: lib.hasPrefix "apac-" k) cfg.governance.councilKeys);
          };
          africa = {
            keys = lib.optional (lib.any (k: lib.hasPrefix "africa-" k) cfg.governance.councilKeys)
              (lib.filter (k: lib.hasPrefix "africa-" k) cfg.governance.councilKeys);
          };
          middleeast = {
            keys = lib.optional (lib.any (k: lib.hasPrefix "middleeast-" k) cfg.governance.councilKeys)
              (lib.filter (k: lib.hasPrefix "middleeast-" k) cfg.governance.councilKeys);
          };
          oceania = {
            keys = lib.optional (lib.any (k: lib.hasPrefix "oceania-" k) cfg.governance.councilKeys)
              (lib.filter (k: lib.hasPrefix "oceania-" k) cfg.governance.councilKeys);
          };
          global = {
            arbiters = 3;
          };
        };
        thresholdSignatures = cfg.governance.thresholdSignatures;
        policyUpdateFrequency = cfg.governance.policyUpdateFrequency;
        allowedOperations = [
          "AddService"
          "RemoveService"
          "UpdateCapabilities"
          "PatchKernel"
          "ScaleCluster"
          "MigrateWorkload"
          "UpdateSecurityPolicy"
          "ManageResources"
        ];
      });

    # Governance timer (periodic sync)
    systemd.timers.omnisystem-governance-audit = {
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnBootSec = "5min";
        OnUnitActiveSec = cfg.governance.policyUpdateFrequency;
        AccuracySec = "1min";
        Persistent = true;
      };
      unitConfig.Description = "Periodic governance audit and policy verification";
    };

    # Immutable audit trail (journald)
    services.journald = {
      extraConfig = ''
        # Governance audit trail
        [Journal]
        Seal=yes
        Storage=persistent
        RateLimitInterval=0
        RateLimitBurst=0
      '';
    };

    # State directories for governance
    systemd.tmpfiles.rules = [
      "d /var/lib/omnisystem/governance 0700 root root -"
      "d /var/lib/omnisystem/governance/policies 0700 root root -"
      "d /var/log/omnisystem/governance 0700 root root -"
    ];

    # Prevent unsigned policy changes
    systemd.services.omnisystem-policy-gate = {
      description = "Omnisystem Policy Gate (Unsigned changes blocked)";
      wantedBy = [ "multi-user.target" ];
      after = [ "omnisystem-bls-verifier.service" ];
      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkgs.omnisystem}/bin/policy-gate";
        Restart = "on-failure";
        StandardOutput = "journal";
        StandardError = "journal";
      };
    };
  };
}
