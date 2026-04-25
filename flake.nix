{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems =
        fn:
        nixpkgs.lib.genAttrs systems (
          system:
          fn (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
    in
    {
      packages = forAllSystems (pkgs: rec {
        default = sherlock-gpui;
        sherlock-gpui = pkgs.rustPlatform.buildRustPackage {
          pname = "sherlock-gpui";
          version = "0.2.0";
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
          cargoLock.outputHashes = {
            "gpui-0.2.2" = "sha256-uEdle/hvTjzEcV6Hcdo/6TRjJiajCfNO/7sIIqufKMA=";
            "naga-29.0.0" = "sha256-sNPsB8oFhAkaaSQ5Lb+CbwXsJIyOlnn80OZYmtNWS1I=";
            "suite-223b-0.1.0" = "sha256-vxm9nZOVTcOENRBCPretttY4TF8l1SV8IkW5KP9LI3A=";
            "wgpu-29.0.0" = "sha256-sNPsB8oFhAkaaSQ5Lb+CbwXsJIyOlnn80OZYmtNWS1I=";
            "zed-font-kit-0.14.1-zed" = "sha256-rxpumYP0QpHW+4e+J1qo5lEZXfBk1LaL/Y0APkUp9cg=";
            "zed-scap-0.0.8-zed" = "sha256-BihiQHlal/eRsktyf0GI3aSWsUCW7WcICMsC2Xvb7kw=";
          };

          buildFeatures = [ "wayland" ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            clang
            makeWrapper
          ];
          buildInputs = with pkgs; [
            wayland
            libxkbcommon
            vulkan-loader
            openssl
            sqlite
            fontconfig
            freetype
            libx11
            libxcursor
            libxrandr
            libxi
            libxcb
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

          postInstall = ''
            wrapProgram $out/bin/sherlock-gpui \
              --prefix LD_LIBRARY_PATH : "${
                pkgs.lib.makeLibraryPath [
                  pkgs.wayland
                  pkgs.libxkbcommon
                  pkgs.vulkan-loader
                ]
              }"
          '';

          meta.mainProgram = "sherlock-gpui";
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            (rust-bin.nightly."2025-01-15".default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            })
            pkg-config
            cmake
            clang
            wayland
            libxkbcommon
            vulkan-loader
            openssl
            sqlite
            fontconfig
            freetype
            libx11
            libxcursor
            libxrandr
            libxi
            libxcb
          ];
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.vulkan-loader
          ]}";
        };
      });
    };
}
