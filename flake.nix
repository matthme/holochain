{
  inputs =
    {
      holochain-flake.url = "github:holochain/holochain";
      holochain-flake.inputs.holochain.url = "github:holochain/holochain/holochain-0.2.0";
      holochain-flake.inputs.lair.url = "github:holochain/lair/lair_keystore-v0.2.4";
      holochain-flake.inputs.launcher.url = "github:holochain/launcher/holochain-0.2";
      holochain-flake.inputs.scaffolding.url = "github:holochain/scaffolding/holochain-0.2";

      nixpkgs.follows = "holochain-flake/nixpkgs";
    };

  outputs = inputs: inputs.holochain-flake.outputs;
}
