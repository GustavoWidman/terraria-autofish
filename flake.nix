{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    devshell.url = "github:numtide/devshell";
  };

  outputs =
    inputs@{
      flake-parts,
      systems,
      devshell,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import systems;
      imports = [
        devshell.flakeModule
        (import ./nix/package.nix inputs)
      ];
      perSystem =
        { pkgs, commonDeps, ... }:
        {
          devshells.default = {
            imports = [
              "${inputs.devshell}/extra/language/c.nix"
            ];

            language.c = {
              includes = commonDeps.buildInputs;
              # libraries = commonDeps.buildInputs;
            };

            devshell = {
              packages = commonDeps.nativeBuildInputs ++ commonDeps.buildInputs;
              motd = "";
            };
          };
        };
    };
}
