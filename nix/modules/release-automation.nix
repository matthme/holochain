# { self, inputs, lib, ... }@flake: {
#   perSystem = { config, self', inputs', system, pkgs, ... }: {
#     packages = {
#       release-automation =
#         pkgs.callPackage ../../crates/release-automation/default.nix {
#           crate2nixSrc = inputs.crate2nix;
#         };

#       release-automation-regenerate-readme =
#         pkgs.writeShellScriptBin "release-automation-regenerate-readme" ''
#           set -x
#           ${pkgs.cargo-readme}/bin/cargo-readme readme --project-root=crates/release-automation/ --output=README.md;
#         '';
#     };
#   };
# }

# Definitions can be imported from a separate file like this one

{ self, inputs, lib, ... }@flake: {
  perSystem = { config, self', inputs', system, pkgs, ... }:
    let

      rustToolchain = config.rust.mkRust {
        track = "stable";
        version = "latest";
      };
      craneLib = inputs.crane.lib.${system}.overrideToolchain rustToolchain;

      commonArgs = {
        pname = "release-automation";
        version = "workspace";
        src = flake.config.srcCleanedReleaseAutomationRepo;

        cargoExtraArgs = "--all-targets";

        buildInputs = (with pkgs; [ openssl ])
          ++ (lib.optionals pkgs.stdenv.isDarwin
          (with pkgs.darwin.apple_sdk_11_0.frameworks; [
            AppKit
            CoreFoundation
            CoreServices
            Security
          ]));

        nativeBuildInputs = (with pkgs;
          [ perl pkg-config ])
        ++ lib.optionals pkgs.stdenv.isDarwin
          (with pkgs; [ xcbuild libiconv ]);
      };

      # derivation building all dependencies
      deps = craneLib.buildDepsOnly (commonArgs // {
        doCheck = false;
      });

      # derivation with the main crates
      package = craneLib.buildPackage (commonArgs // {
        cargoArtifacts = deps;
        doCheck = false;
      });

      tests = craneLib.cargoNextest (commonArgs // {
        pname = "${commonArgs.pname}-tests";
        __noChroot = pkgs.stdenv.isLinux;

        cargoArtifacts = deps;

        buildInputs = commonArgs.buildInputs ++ [ pkgs.cacert ];
        nativeBuildInputs = commonArgs.nativeBuildInputs ++
          [
            package

            rustToolchain
            pkgs.gitFull
            pkgs.coreutils
          ];

        cargoNextestExtraArgs =
          let
            nextestToml = builtins.toFile "nextest.toml" ''
              [profile.default]
              retries = { backoff = "exponential", count = 3, delay = "1s", jitter = true }
              status-level = "skip"
              final-status-level = "flaky"

            '';
          in
          '' \
            --config-file=${nextestToml} \
          '' + builtins.getEnv "NEXTEST_EXTRA_ARGS";

        dontPatchELF = true;
        dontFixup = true;
        installPhase = ''
          mkdir -p $out
          cp -vL target/.rustc_info.json $out/
        '';
      });

      packagePath = lib.makeBinPath [ package ];

    in
    {
      packages = {
        release-automation = package;

        build-release-automation-tests = tests;

        # check the state of the repository
        # TODO: to get the actual .git repo we could be something like this:
        # using a dummy input like this:
        # ```nix
        #     repo-git.url = "file+file:/dev/null";
        #     repo-git.flake = false;
        # ```
        # and then when i run the test derivations that rely on that input, i can temporarily lock that input to a local path like this:
        # ```
        # tmpgit=$(mktemp -d)
        # git clone --bare --single-branch . $tmpgit
        # nix flake lock --update-input repo-git --override-input repo-git "path:$tmpdir"
        # rm -rf $tmpgit
        # ```
        build-release-automation-tests-repo = pkgs.runCommand
          "release-automation-tests-repo"
          {
            __noChroot = pkgs.stdenv.isLinux;
            nativeBuildInputs = self'.packages.holochain.nativeBuildInputs ++ [
              pkgs.coreutils
              pkgs.gitFull
            ];
            buildInputs = self'.packages.holochain.buildInputs ++ [
              pkgs.cacert
            ];
          } ''
          set -euo pipefail

          export HOME="$(mktemp -d)"
          export TEST_WORKSPACE="''${HOME:?}/src"

          git config --global user.email "ci@holochain.org"
          git config --global user.name "CI"

          git clone --single-branch ${inputs.dummy} ''${TEST_WORKSPACE:?}
          cd ''${TEST_WORKSPACE:?}
          git status

          ${self'.packages.scripts-release-automation-prepare}/bin/scripts-release-automation-prepare ''${TEST_WORKSPACE:?}

          set +e
          git clean -ffdx
          mv ''${TEST_WORKSPACE:?} $out
          echo use "nix-store --realise $out" to retrieve the result.
        '';

        build-release-automation-detect-missing-release-headings-repo = pkgs.runCommand
          "release-automation-detect-missing-release-headings-repo"
          {
            nativeBuildInputs = self'.packages.holochain.nativeBuildInputs ++ [
              package

              pkgs.coreutils
              pkgs.gitFull
            ];
            buildInputs = self'.packages.holochain.buildInputs ++ [
            ];
          } ''

          set -euo pipefail

          export HOME="$(mktemp -d)"
          export TEST_WORKSPACE="''${HOME:?}/src"

          cp -r --no-preserve=mode,ownership ${flake.config.srcCleanedRepoWithChangelogs} ''${TEST_WORKSPACE:?}
          cd ''${TEST_WORKSPACE:?}

          git init
          git switch -c main
          git add .
          git config --global user.email "you@example.com"
          git config --global user.name "Your Name"
          git commit -am "main"

          release-automation \
              --workspace-path=$PWD \
              --log-level=debug \
              crate detect-missing-releaseheadings

          touch $out
        '';
      };
    };
}
