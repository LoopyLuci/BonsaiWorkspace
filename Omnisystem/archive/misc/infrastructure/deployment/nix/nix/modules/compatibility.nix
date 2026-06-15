{ config, pkgs, lib, ... }:

# Compatibility Module – POSIX, Windows ABI, Legacy Driver Support
# Optional: add to nixosConfigurations modules list for compatibility layers

let
  cfg = config.services.omnisystem;

in {
  config = lib.mkIf (cfg.enable && cfg.compatibility.posix.enable) {
    # POSIX compatibility layer
    environment.systemPackages = [
      pkgs.glibc
      pkgs.gcc
      pkgs.binutils
    ];

    # Set up POSIX environment variables
    environment.sessionVariables = {
      LD_LIBRARY_PATH = lib.mkDefault "${pkgs.glibc}/lib:${pkgs.gcc.cc.lib}/lib";
    };

    # System-wide locale
    i18n = {
      defaultLocale = lib.mkDefault "en_US.UTF-8";
      supportedLocales = lib.mkDefault [ "en_US.UTF-8/UTF-8" ];
    };

    # Enable systemd user services (POSIX service model)
    systemd.user.enable = lib.mkDefault true;
  };

  # Windows ABI emulation (if enabled)
  config = lib.mkIf (cfg.enable && cfg.compatibility.windows.enable) {
    environment.systemPackages = [
      pkgs.wine
      pkgs.winetricks
    ];

    # Windows runtime wrapper
    systemd.services = {
      omnisystem-windows-abi = {
        description = "Omnisystem Windows ABI Emulation";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-kernel.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/windows-abi-shim";
          Restart = "on-failure";
          StandardOutput = "journal";
          StandardError = "journal";
        };
      };
    };

    environment.etc."omnisystem/windows-abi.conf".text = ''
      # Windows ABI compatibility configuration
      [Emulation]
      Enabled = true
      APITranslationMode = capability-based
      FileSystemMapping = /mnt/c -> C:
      NetworkMode = translated
      ThreadModel = 1-to-1

      [Performance]
      OptimizationLevel = 2
      CachingEnabled = true
    '';
  };

  # Legacy driver wrappers (if enabled)
  config = lib.mkIf (cfg.enable && cfg.compatibility.legacyDrivers.enable) {
    systemd.services = {
      omnisystem-legacy-driver-manager = {
        description = "Omnisystem Legacy Driver Manager";
        wantedBy = [ "multi-user.target" ];
        after = [ "omnisystem-device-manager.service" ];
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.omnisystem}/bin/legacy-driver-manager";
          Restart = "on-failure";
          StandardOutput = "journal";
          StandardError = "journal";
        };
      };
    };

    environment.etc."omnisystem/legacy-drivers.conf".text = ''
      # Legacy device driver compatibility

      [Drivers]
      # Map legacy PCI IDs to modern drivers
      [PCI:8086:0D04]
        ModernDriver = integrated-storage
        TranslationLayer = legacy-ahci

      # USB legacy support
      [USB]
        EnableLegacyHID = true
        EnableLegacyHubEmulation = true
    '';
  };
}
