{
  inputs =
    {
      holochain.url = "github:holochain/holochain/holochain-0.2.0";
      holochain.flake = false;
      lair.url = "github:holochain/lair/lair_keystore-v0.2.4";
      lair.flake = false;
      launcher.url = "github:holochain/launcher/holochain-0.2";
      launcher.flake = false;
      scaffolding.url = "github:holochain/scaffolding/holochain-0.2";
      scaffolding.flake = false;

      holochain-flake.url = "github:holochain/holochain";
      holochain-flake.inputs.holochain.follows = "holochain";
      holochain-flake.inputs.lair.follows = "lair";
      holochain-flake.inputs.launcher.follows = "launcher";
      holochain-flake.inputs.scaffolding.follows = "scaffolding";

      nixpkgs.follows = "holochain-flake/nixpkgs";
      flake-parts.follows = "holochain-flake/flake-parts";
    };

  outputs = inputs: inputs.holochain-flake.outputs;
}
