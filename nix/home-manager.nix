{
  self,
  lib,
  config,
  pkgs,
  ...
}: let
  inherit (lib) mkIf mkOption types;
  cfg = config.programs.sherlock;
in {
  options.programs.sherlock = with types; {
    enable = lib.mkEnableOption "Manage sherlock & config files with home-manager module." // {default = false;};

    settings = mkOption {
      description = "Sherlock settings, seperated by config file.";
      default = {};
      type = submodule {
        options = {
          aliases = mkOption {
            default = null;
            description = "'sherlock_alias.json'";
            type = nullOr (attrsOf submodule {
              options = {
                name = mkOption {
                  type = str;
                };
                icon = mkOption {
                  type = str;
                };
                exec = mkOption {
                  type = str;
                };
                keywords = mkOption {
                  type = str;
                };
              };
            });
          };
          ignore = mkOption {
            default = "";
            description = "'sherlockignore' file contents.";
            type = lines;
          };
        };
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [self.packages.${pkgs.system}.default];

    # sherlock expects all these files to exist
    xdg.configFile."sherlock/sherlock_alias.json".text =
      if cfg.settings.aliases != null
      then builtins.toJSON cfg.settings.aliases
      else "{}";

    xdg.configFile."sherlock/sherlockignore".text = cfg.settings.ignore;
  };
}
