{
  lib,
  config,
  pkgs,
  ...
}: let
  inherit (lib) mkIf mkMerge mkOption types;
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
  # TODO(Vanta_1): fix these up into proper types
  configType = types.anything;
  launcherType = types.anything;
in {
  options.programs.sherlock = with types; {
    enable = lib.mkEnableOption "Manage sherlock & config files with home-manager module." // {default = false;};
    package = lib.mkPackageOption pkgs "sherlock" {
      default = ["sherlock-launcher"];
    };

    settings = mkOption {
      description = "Sherlock settings, seperated by config file.";
      default = {};
      type = submodule {
        options = {
          aliases = mkOption {
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
            default = {};
            type = nullOr (attrsOf aliasType);
          };
          config = mkOption {
            description = ''
              `config.json` in Nix syntax.
            '';
            default = {};
            type = nullOr (attrsOf configType);
          };
          ignore = mkOption {
            description = "'sherlockignore' file contents.";
            default = "";
            type = nullOr lines;
          };
          launchers = mkOption {
            description = "'fallback.json' in Nix syntax. See ```settings.aliases``` for a similar example.";
            default = [];
            type = nullOr (listOf launcherType);
          };
          style = mkOption {
            description = "CSS content for Sherlock UI styling, written to 'main.css'";
            default = "";
            type = nullOr lines;
          };
        };
      };
    };
  };

  config = mkIf cfg.enable (mkMerge [
    {
      home.packages = [cfg.package];
    }
    (mkIf (cfg.settings != null) (mkMerge [
      (mkIf (cfg.settings.aliases != null) {
        xdg.configFile."sherlock/sherlock_alias.json".text = builtins.toJSON cfg.settings.aliases;
      })
      (mkIf (cfg.settings.config != null) {
        xdg.configFile."sherlock/config.json".text = builtins.toJSON cfg.settings.config;
      })
      (mkIf (cfg.settings.ignore != null) {
        xdg.configFile."sherlock/sherlockignore".text = cfg.settings.ignore;
      })
      (mkIf (cfg.settings.launchers != null) {
        xdg.configFile."sherlock/fallback.json".text = builtins.toJSON cfg.settings.launchers;
      })
      (mkIf (cfg.settings.style != null) {
        xdg.configFile."sherlock/main.css".text = cfg.settings.style;
      })
    ]))
  ]);
}
