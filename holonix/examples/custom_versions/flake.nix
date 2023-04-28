{
  description = "Template for Holochain app development with custom versions";

  inputs.holonix.url = "github:holochain/holochain?dir=holonix/versions/0_2";

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

            packages = {
              inherit (inputs'.holochain-flake.packages) lair-keystore;
            };
          };
      };
}
