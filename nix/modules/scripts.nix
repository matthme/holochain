{ self, lib, ... }: {
  perSystem = { config, self', inputs', pkgs, ... }: {
    # Definitions like this are entirely equivalent to the ones
    # you may have directly in flake.nix.
    packages = {
      scripts-ci-cachix-helper =
        let
          pathPrefix = lib.makeBinPath (with pkgs; [ cachix ]);
        in
        pkgs.writeShellScript "scripts-ci-cachix-helper" ''
            #! /usr/bin/env nix-shell
            set -euo pipefail

          export PATH=${pathPrefix}:$PATH

          export PATHS_PREBUILD_FILE="''${HOME}/.store-path-pre-build"

          case ''${1} in
            setup)
              if [[ -n ''${CACHIX_AUTH_TOKEN:-} ]]; then
                  echo Using CACHIX_AUTH_TOKEN
                  cachix --verbose authtoken ''${CACHIX_AUTH_TOKEN}
              fi
              cachix --verbose use -m user-nixconf ''${CACHIX_NAME:?}
              nix path-info --all > "''${PATHS_PREBUILD_FILE}"
              ;;

            push)
              comm -13 <(sort "''${PATHS_PREBUILD_FILE}" | grep -v '\.drv$') <(nix path-info --all | grep -v '\.drv$' | sort) | cachix --verbose push ''${CACHIX_NAME:?}
              ;;
          esac
        '';

      scripts-repo-flake-update = pkgs.writeShellScriptBin "scripts-repo-flake-update" ''
        set -xeuo pipefail
        trap "cd $PWD" EXIT

        cd versions/0_1
        nix flake update
        cd ../../

        nix flake lock --update-input versions --override-input versions "git+file:.?dir=versions/0_1&ref=$(git rev-parse HEAD)"

        if [[ $(${pkgs.git}/bin/git diff -- versions/0_1/flake.lock | grep -E '^[+-]\s+"' | grep -v lastModified --count) -eq 0 ]]; then
          echo got no actual source changes, reverting modifications..;
          ${pkgs.git}/bin/git checkout flake.lock versions/0_1/flake.lock
        fi

        git commit versions/0_1/flake.lock -m "updating versions/0_1 flake"

        # replace the URL of the versions flake with the github URL
        nix eval --impure --json --expr "
          let
            lib = (import ${pkgs.path} {}).lib;
            lock = builtins.fromJSON (builtins.readFile ./flake.lock);
            lock_updated = lib.recursiveUpdate lock { nodes.versions.locked.url = "github:holochain/holochain?dir=versions/0_1"; };
          in lock_updated
        " | ${pkgs.jq}/bin/jq --raw-output . > flake.lock.new

        mv flake.lock{.new,}

        if [[ $(${pkgs.git}/bin/git diff -- flake.lock | grep -E '^[+-]\s+"' | grep -v lastModified --count) -eq 0 ]]; then
          echo got no actual source changes, reverting modifications..;
          ${pkgs.git}/bin/git checkout flake.lock versions/*/flake.lock
        fi

        git commit flake.lock -m "updating toplevel flake"
      '';
    };
  };
}
