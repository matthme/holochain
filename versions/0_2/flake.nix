{
  inputs =
    {
      holochain.url = "github:holochain/holochain/develop-0.2";
      holochain.flake = false;
      lair.url = "github:holochain/lair/lair_keystore-v0.2.4";
      lair.flake = false;
      launcher.url = "github:holochain/launcher/holochain-0.2";
      launcher.flake = false;
      scaffolding.url = "github:holochain/scaffolding/holochain-0.2";
      scaffolding.flake = false;
    };

  outputs = _: { };
}
