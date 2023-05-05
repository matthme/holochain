{ inputs, self, lib, ... }: {
  perSystem = { config, self', inputs', pkgs, ... }:
    let
      holonix = pkgs.mkShell {
        inputsFrom = [ self'.devShells.rustDev ];
        packages = holonixPackages ++ [ hn-introspect ];
        shellHook = ''
          echo Holochain development shell spawned. Type 'exit' to leave.
          export PS1='\n\[\033[1;34m\][holonix:\w]\$\[\033[0m\] '
        '';
      };
      holonixPackages = with self'.packages; [ holochain lair-keystore hc-launch hc-scaffold ];
      versionsFileText = builtins.concatStringsSep "\n"
        (
          builtins.map
            (package: ''
              echo ${package.pname} \($(${package}/bin/${package.pname} -V)\): ${package.src.rev or "na"}'')
            holonixPackages
        );
      hn-introspect =
        pkgs.writeShellScriptBin "hn-introspect" versionsFileText;

      versionsInputSpecified = (builtins.pathExists "${inputs.versions.outPath}/flake.nix") || (builtins.readFile inputs.versions.outPath != "");
    in
    {
      devShells.holonix = holonix;

      packages = {
        inherit hn-introspect;
      };
    };
}
