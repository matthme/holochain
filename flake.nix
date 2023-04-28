{
  description =
    "Holochain is an open-source framework to develop peer-to-peer applications with high levels of security, reliability, and performance.";

  inputs = {
    "0_1".url = "path:./versions/0_1";
    "0_2".url = "path:./versions/0_2";
  };

  # refer to flake-parts docs https://flake.parts/
  outputs = _: {};
}
