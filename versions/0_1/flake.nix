{
  inputs =
    {
      holochain.url = "github:holochain/holochain/holochain-0.1.5-beta-rc.0";
      holochain.flake = false;
      lair.url = "github:holochain/lair/lair_keystore-v0.2.3";
      lair.flake = false;
      launcher.url = "github:holochain/launcher/holochain-0.1";
      launcher.flake = false;
      scaffolding.url = "github:holochain/scaffolding/holochain-0.1";
      scaffolding.flake = false;
    };

  outputs = _: { };
}
