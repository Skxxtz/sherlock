# this is mostly from https://fasterthanli.me/series/building-a-rust-service-with-nix/part-10#a-flake-with-derivation
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    ...
  } @ inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      # sherlock currently only supports linux
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem = {system, ...}: let
        name = "sherlock";
        version = "0.1.14";

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

        nativeBuildInputs = with pkgs; [rustToolchain pkg-config wrapGAppsHook];
        buildInputs = with pkgs; [
          dbus
          glib
          gtk4
          gtk4-layer-shell
          gdk-pixbuf
          librsvg
          openssl
          sqlite
          wayland
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

            meta = {mainProgram = "sherlock";};
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
