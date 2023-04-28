{
  # used for debugging

  # useful commands:
  # show which tag holochain points to:
  # git tag --points-at $(nix flake metadata . --json | jq --raw-output '.locks.nodes.holochain.locked.rev') | grep holochain-

  description = "Template for Holochain app development with custom versions";

  inputs.holochain-flake.url = "github:holochain/holochain/pr_versions_0_2?dir=versions/0_2";
  # inputs.nixpkgs.follows = "holochain-flake/nixpkgs";

  outputs = inputs @ { ... }:
    inputs.holochain-flake.inputs.flake-parts.lib.mkFlake { inherit inputs; }
      {
        systems = builtins.attrNames inputs.holochain-flake.devShells;
        perSystem =
          { inputs'
          , config
          , pkgs
          , system
          , ...
          }: {
            devShells.default = pkgs.mkShell {
              inputsFrom = [ inputs'.holochain-flake.devShells.holonix ];
              packages = with pkgs; [
                # more packages go here
              ];
            };
          };
      };
}
