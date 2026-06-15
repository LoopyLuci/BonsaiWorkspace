# nix/modules/bonsai-services.nix — NixOS module to run Bonsai services
{ config, lib, pkgs, ... }:

let
  cfg = config.services.bonsai-services;
  inherit (lib) mkEnableOption mkOption mkIf types concatStringsSep;
in {
  options.services.bonsai-services = {
    enable = mkEnableOption "Bonsai Ecosystem services";

    mcp-server = {
      enable = mkEnableOption "Bonsai MCP Server";
      port = mkOption {
        type = types.port;
        default = 11425;
        description = "Port for MCP server";
      };
      listen = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = "Listen address";
      };
    };

    bmf-server = {
      enable = mkEnableOption "Bonsai Messaging Fabric (SMTP/IMAP)";
      smtpPort = mkOption {
        type = types.port;
        default = 25;
      };
      imapPort = mkOption {
        type = types.port;
        default = 143;
      };
    };
  };

  config = mkIf cfg.enable {
    # Create bonsai system user
    users.users.bonsai = {
      isSystemUser = true;
      group = "bonsai";
      home = "/var/lib/bonsai";
      createHome = true;
      shell = pkgs.nologin;
    };
    users.groups.bonsai = { };

    # Ensure data directory
    systemd.tmpfiles.rules = [
      "d /var/lib/bonsai 0755 bonsai bonsai -"
      "d /var/log/bonsai 0755 bonsai bonsai -"
    ];

    # MCP Server systemd unit
    systemd.services.bonsai-mcp-server = mkIf cfg.mcp-server.enable {
      description = "Bonsai MCP Server";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.bonsai-mcp-server}/bin/bonsai-mcp-server --listen ${cfg.mcp-server.listen} --port ${toString cfg.mcp-server.port}";
        Restart = "on-failure";
        RestartSec = "10s";
        User = "bonsai";
        Group = "bonsai";
        StandardOutput = "journal";
        StandardError = "journal";
      };
    };

    # BMF Server systemd unit
    systemd.services.bonsai-bmf-server = mkIf cfg.bmf-server.enable {
      description = "Bonsai Messaging Fabric (SMTP/IMAP)";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${pkgs.bonsai-bmf-server}/bin/bmf-server --smtp-port ${toString cfg.bmf-server.smtpPort} --imap-port ${toString cfg.bmf-server.imapPort}";
        Restart = "on-failure";
        RestartSec = "10s";
        User = "bonsai";
        Group = "bonsai";
        StandardOutput = "journal";
        StandardError = "journal";
      };
    };

    # Install Bonsai CLI globally
    environment.systemPackages = [
      pkgs.bonsai-cli
    ];
  };
}
