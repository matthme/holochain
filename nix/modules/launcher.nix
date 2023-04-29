# Definitions can be imported from a separate file like this one

{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      rustToolchain = config.rustHelper.mkRust { };
      craneLib = inputs.crane.lib.${system}.overrideToolchain rustToolchain;

      commonArgs = {

        pname = "hc-launch";
        src = inputs.launcher;

        CARGO_PROFILE = "release";

        cargoExtraArgs = "--bin hc-launch";

        buildInputs =
          (with pkgs; [
            openssl
            # glib

            # TODO: remove this once the features have been rearranged to use vendored sqlite
            sqlite
          ])
          ++ (lib.optionals pkgs.stdenv.isLinux
            (with pkgs; [
              webkitgtk.dev
              gdk-pixbuf
              gtk3
            ]))
          ++ lib.optionals pkgs.stdenv.isDarwin
            (
              # because other packages propagate apple_sdk_10_12 frameworks we adhere to that
              (with pkgs.darwin.apple_sdk_10_12.frameworks; [
                AppKit
                Foundation
                Carbon
                CoreFoundation
                CoreServices
                IOKit
                Security
                WebKit
              ])
            )
        ;

        nativeBuildInputs =
          (with pkgs;
          [
            perl
            pkg-config
            (go.overrideAttrs
              (prevAttrs: lib.attrsets.optionalAttrs stdenv.isDarwin {
                depsTargetTargetPropagated = [ ];
              }))
          ])
          ++ (lib.optionals pkgs.stdenv.isLinux
            (with pkgs; [
              wrapGAppsHook
            ]))
          ++ lib.optionals pkgs.stdenv.isDarwin (with pkgs; [
            xcbuild
            libiconv
          ])
        ;

        doCheck = false;
      };

      # derivation building all dependencies
      deps = craneLib.buildDepsOnly (commonArgs // { });

      # derivation with the main crates
      package = craneLib.buildPackage (commonArgs // {
        cargoArtifacts = deps;

        nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
          pkgs.makeBinaryWrapper
        ];

        preFixup = ''
          gappsWrapperArgs+=(
            --set WEBKIT_DISABLE_COMPOSITING_MODE 1
          )

          # without this the DevTools will just display an unparsed HTML file (see https://github.com/tauri-apps/tauri/issues/5711#issuecomment-1336409601)
          gappsWrapperArgs+=(
            --prefix XDG_DATA_DIRS : "${pkgs.shared-mime-info}/share"
          )
        '';
      });

      rustPkgs = config.rustHelper.mkRustPkgs {
        track = "stable";
        version = "1.69.0";
      };

      cargoNix = config.rustHelper.mkCargoNix {
        name = "hc-launch-generated-crate2nix";
        src = inputs.launcher;
        pkgs = rustPkgs;
      };

    in
    {
      packages = {
        hc-launch = package;

        hc-launch-crate2nix =
          config.rustHelper.mkNoIfdPackage
            "hc-launch"
            cargoNix.workspaceMembers.holochain_cli_launch.build
        ;

        _debug-build-crate2nix =
          let
            cargoNix = config.rustHelper.mkCargoNix {
              name = "debug-build";
              src = flake.config.srcCleanedDebugBuild;
              pkgs = rustPkgs;
            };
          in
          cargoNix.allWorkspaceMembers;

        _debug-build =
          craneLib.buildPackage
            (lib.attrsets.recursiveUpdate commonArgs {
              cargoArtifacts = null;

              pname = "debug-build";
              cargoExtraArgs = "";
              CARGO_PROFILE = "release";
              src = flake.config.srcCleanedDebugBuild;
              doCheck = false;
            });
      };
    };
}

