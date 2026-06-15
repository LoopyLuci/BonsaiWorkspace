# nix/modules/usos-co-os.nix — Run USOS as a co-OS alongside NixOS
{ config, lib, pkgs, ... }:

let
  cfg = config.services.usos-co-os;
  inherit (lib) mkEnableOption mkOption mkIf types;
in {
  options.services.usos-co-os = {
    enable = mkEnableOption "USOS Co-OS (run as KVM guest alongside NixOS)";

    package = mkOption {
      type = types.package;
      description = "USOS VM package to use";
      default = pkgs.usos-vm;
    };

    memory = mkOption {
      type = types.str;
      default = "2G";
      description = "RAM allocated to USOS VM (e.g., '2G', '4096M')";
    };

    cpuCores = mkOption {
      type = types.int;
      default = 2;
      description = "Number of CPU cores assigned to USOS";
    };

    kvmEnabled = mkOption {
      type = types.bool;
      default = true;
      description = "Enable hardware acceleration (KVM)";
    };

    sharedFolder = mkOption {
      type = types.path;
      default = "/var/lib/usos-shared";
      description = "Host directory shared with USOS via 9p";
    };

    sshPort = mkOption {
      type = types.port;
      default = 2222;
      description = "SSH port forwarded from guest port 22";
    };

    autoStart = mkOption {
      type = types.bool;
      default = true;
      description = "Start USOS VM on boot";
    };
  };

  config = mkIf cfg.enable {
    # Ensure QEMU/KVM support
    virtualisation.libvirtd.enable = true;
    virtualisation.kvm.enable = cfg.kvmEnabled;

    # Create usos system user
    users.users.usos = {
      isSystemUser = true;
      group = "usos";
      extraGroups = [ "kvm" "libvirtd" ];
      home = "/var/lib/usos";
      createHome = true;
    };
    users.groups.usos = { };

    # Shared folder and state directories
    systemd.tmpfiles.rules = [
      "d ${cfg.sharedFolder} 0755 usos usos -"
      "d /var/lib/usos 0755 usos usos -"
      "d /var/log/usos 0755 usos usos -"
    ];

    # Systemd service to run USOS VM
    systemd.services.usos-co-os = {
      description = "USOS Co-OS Virtual Machine";
      after = [ "network.target" "libvirtd.service" ];
      wantedBy = mkIf cfg.autoStart [ "multi-user.target" ];
      requires = [ "libvirtd.service" ];

      environment = {
        USOS_MEMORY = cfg.memory;
        USOS_CPUS = toString cfg.cpuCores;
        USOS_SHARE_DIR = cfg.sharedFolder;
        USOS_KVM = if cfg.kvmEnabled then "true" else "false";
      };

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/usos-vm";
        Restart = "on-failure";
        RestartSec = "5s";
        User = "usos";
        Group = "usos";
        StandardOutput = "journal";
        StandardError = "journal";
        LimitMEMLOCK = "infinity";
        LimitNOFILE = "1024000";
      };

      # Graceful shutdown
      ExecStop = "${pkgs.killall}/bin/killall -TERM qemu-system-x86_64 || true";
      TimeoutStopSec = "30s";
    };

    # Helper script to SSH into USOS
    environment.systemPackages = [
      (pkgs.writeShellScriptBin "usos-ssh" ''
        exec ${pkgs.openssh}/bin/ssh -p ${toString cfg.sshPort} root@127.0.0.1 "$@"
      '')

      # Convenience script to access shared folder
      (pkgs.writeShellScriptBin "usos-share" ''
        exec ${pkgs.coreutils}/bin/ls -la "${cfg.sharedFolder}"
      '')

      cfg.package
    ];

    # Firewall rules for SSH forwarding (if firewall is enabled)
    networking.firewall.allowedTCPPorts = mkIf config.networking.firewall.enable [
      cfg.sshPort
    ];

    # Journal retention for USOS logs
    services.journald.extraConfig = ''
      Storage=persistent
      MaxRetentionSec=30day
    '';
  };
}
