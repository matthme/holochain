{
  description = "Template for Holochain app development with custom versions";

  inputs = {
    nixpkgs.follows = "holochain-flake/nixpkgs";

    holochain-flake.url = "github:holochain/holochain/versions-0.2";
  };

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
