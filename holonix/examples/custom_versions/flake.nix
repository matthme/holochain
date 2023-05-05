{
  description = "Template for Holochain app development that uses a specific versions set";

  inputs = {
    holochain-flake.url = "github:holochain/holochain";

    holochain-versions.url = "github:holochain/holochain?dir=versions/0_2";
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
