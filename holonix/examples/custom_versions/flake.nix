{
  description = "Template for Holochain app development that uses a specific versions set";

  inputs = {
    holochain-versions.url = "github:holochain/holochain/experiment_deprecate_component_inputs?dir=versions/0_1";
    holochain-flake.url = "github:holochain/holochain/experiment_deprecate_component_inputs";
    holochain-flake.inputs.versions.follows = "holochain-versions";

    nixpkgs.follows = "holochain-flake/nixpkgs";
    flake-parts.follows = "holochain-flake/flake-parts";
  };

  outputs = inputs @ { ... }:
    inputs.flake-parts.lib.mkFlake { inherit inputs; }
      {
        flake.inputs = inputs;
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
