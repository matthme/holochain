{
  inputs =
    {
      holochain = {
        url = "github:holochain/holochain/main-0.1";
        flake = false;
      };

      lair = {
        url = "github:holochain/lair/main";
        flake = false;
      };

      # holochain_cli_launch
      launcher = {
        url = "github:holochain/launcher/holochain-0.1";
        flake = false;
      };

      # holochain_scaffolding_cli
      scaffolding = {
        url = "github:holochain/scaffolding/holochain-0.1";
        flake = false;
      };
    };

  outputs = { ... }: { };
}
