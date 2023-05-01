# Definitions can be imported from a separate file like this one

{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let
      # aarch64 only uses 11.0 and x86_64 mixes them
      apple_sdk' = lib.attrsets.optionalAttrs pkgs.stdenv.isDarwin
        (
          if system == "x86_64-darwin"
          then pkgs.darwin.apple_sdk_10_12
          else pkgs.darwin.apple_sdk_11_0
        );

      appleNativeBuildInputs = sdk: with sdk.frameworks; [
        AppKit
        CoreFoundation
        Foundation
        Security
        WebKit
        IOKit
      ];

      go' =
        # there is interference only in this specific case, we assemble a go derivationt that not propagate anything but still has everything available required for our specific use-case
        #
        # the wrapper inherits preconfigured environment variables from the
        # derivation that depends on the propagating go
        if pkgs.stdenv.isDarwin && pkgs.system == "x86_64-darwin" then
          pkgs.darwin.apple_sdk_11_0.stdenv.mkDerivation
            {
              name = "go";

              nativeBuildInputs = with pkgs; [
                makeBinaryWrapper
                go
              ];

              dontBuild = true;
              dontUnpack = true;

              installPhase = ''
                makeWrapper ${pkgs.go}/bin/go $out/bin/go \
                  ${builtins.concatStringsSep " " (
                    builtins.map (var: "--set ${var} \"\$${var}\"") 
                    [
                      "NIX_BINTOOLS_WRAPPER_TARGET_HOST_x86_64_apple_darwin"
                      "NIX_LDFLAGS"
                      "NIX_CFLAGS_COMPILE_FOR_BUILD"
                      "NIX_CFLAGS_COMPILE"

                      # confirmed needed above here

                      # unsure between here
                      # and here

                      # confirmed unneeded below here

                      # "NIX_CC"
                      # "NIX_CC_FOR_BUILD"
                      # "NIX_LDFLAGS_FOR_BUILD"
                      # "NIX_BINTOOLS"
                      # "NIX_CC_WRAPPER_TARGET_HOST_x86_64_apple_darwin"
                      # "NIX_CC_WRAPPER_TARGET_BUILD_x86_64_apple_darwin"
                      # "NIX_ENFORCE_NO_NATIVE"
                      # "NIX_DONT_SET_RPATH"
                      # "NIX_BINTOOLS_FOR_BUILD"
                      # "NIX_DONT_SET_RPATH_FOR_BUILD"
                      # "NIX_NO_SELF_RPATH"
                      # "NIX_IGNORE_LD_THROUGH_GCC"
                      # "NIX_PKG_CONFIG_WRAPPER_TARGET_HOST_x86_64_apple_darwin"
                      # "NIX_COREFOUNDATION_RPATH"
                      # "NIX_BINTOOLS_WRAPPER_TARGET_BUILD_x86_64_apple_darwin"
                    ]
                  )}
              '';
            }
        else pkgs.go;

      rustToolchain = config.rustHelper.mkRust { };
      craneLib = inputs.crane.lib.${system}.overrideToolchain rustToolchain;

      commonArgs = {

        pname = "hc-launch";
        src = inputs.launcher;

        CARGO_PROFILE = "release";

        cargoExtraArgs = "--bin hc-launch";

        # stdenv = if pkgs.stdenv.isDarwin then pkgs.darwin.stdenv else pkgs.stdenv;

        buildInputs = (with pkgs; [
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
          (appleNativeBuildInputs apple_sdk')
        ;

        nativeBuildInputs = (with pkgs; [
          perl
          pkg-config

          # currently needed to build tx5
          go'
        ])
        ++ (lib.optionals pkgs.stdenv.isLinux
          (with pkgs; [
            wrapGAppsHook
          ]))
        ++ (lib.optionals pkgs.stdenv.isDarwin [
          pkgs.xcbuild
          pkgs.libiconv
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

      rustPkgs = config.rustHelper.mkRustPkgs { };

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

        goWrapped = go';
      };
    };
}





