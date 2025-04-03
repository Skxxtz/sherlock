self: {
  lib,
  config,
  pkgs,
  ...
}: let
  inherit (lib) mkIf mkOption types;
  cfg = config.programs.sherlock;
  aliasType = with types;
    submodule {
      options = {
        name = mkOption {
          type = str;
          default = "";
        };
        icon = mkOption {
          type = str;
          default = "";
        };
        exec = mkOption {
          type = str;
          default = "";
        };
        keywords = mkOption {
          type = str;
          default = "";
        };
      };
    };
  # TODO(Vanta_1): fix this up into a proper type
  launcherType = types.anything;
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
            description = ''
              'sherlock_alias.json' file contents in Nix syntax, e.g.

              ```nix
              aliases.<name> = { name = "example"; };
              ```

              Would become:

              ```json
              "<name>": {
                "name": "example"
              }
              ```
            '';
            type = nullOr (attrsOf aliasType);
          };
          ignore = mkOption {
            default = "";
            description = "'sherlockignore' file contents.";
            type = lines;
          };
          launchers = mkOption {
            default = null;
            description = "'fallback.json' in Nix syntax. See ```settings.aliases``` for a similar example.";
            type = nullOr (listOf launcherType);
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

    xdg.configFile."sherlock/fallback.json".text =
      if cfg.settings.launchers != null
      then builtins.toJSON cfg.settings.launchers
      else "[]";
  };
}
