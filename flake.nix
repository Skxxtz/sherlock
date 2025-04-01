# this is mostly from https://fasterthanli.me/series/building-a-rust-service-with-nix/part-10#a-flake-with-derivation
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    home-manager,
    ...
  } @ inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      # sherlock currently only supports linux
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake.homeManagerModules.default = {
        config,
        lib,
        pkgs,
        ...
      }: let
        cfg = config.programs.sherlock;
      in {
        options.programs.sherlock = {
          enable = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Manage sherlock config files with Nix module.";
          };
        };
        config = lib.mkIf cfg.enable {
          xdg.configFile."sherlock/test".text = "";
        };
      };

      perSystem = {
        system,
        stdenv,
        ...
      }: let
        name = "sherlock";
        version = "0.1.6";

        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import inputs.rust-overlay)];
        };

        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = with nixpkgs;
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              (lib.fileset.maybeMissing ./resources)
            ];
          };

        nativeBuildInputs = with pkgs; [rustToolchain pkg-config];
        buildInputs = with pkgs; [
          dbus.dev
          glib.dev
          gtk4.dev
          gtk4-layer-shell.dev
          openssl.dev
          sqlite.dev
          wayland.dev
        ];
        commonArgs = {
          inherit src buildInputs nativeBuildInputs;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        bin = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;
            pname = "${name}";
            version = "${version}";
          });
      in {
        devShells.default = pkgs.mkShell {
          inputsFrom = [bin];
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
          shellHook = ''
            echo "entering ${name} devshell..."
          '';
        };
        packages = {
          inherit bin;
          default = bin;
        };
      };
    };
}
