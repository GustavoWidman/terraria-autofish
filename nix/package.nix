{
  crane,
  fenix,
  ...
}:
{
  perSystem =
    {
      pkgs,
      lib,
      system,
      ...
    }:
    let
      toolchain = fenix.packages.${system}.fromToolchainFile {
        file = ../rust-toolchain.toml;
        sha256 = "sha256-K8/aNzEwNFy5A+HIFCFhHbilHJezC3HqOc9YItLeZ7c=";
      };
      craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
      root = ../.;
      commonDeps = {
        nativeBuildInputs = with pkgs; [ pkg-config ];
        buildInputs = with pkgs; [
          libiconv
          openssl
        ];
      };

      args = {
        src = lib.fileset.toSource {
          inherit root;
          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources root)
            # (lib.fileset.fileFilter (file: file.hasExt "md") root)
          ];
        };
        strictDeps = true;
      }
      // commonDeps;

      bin = craneLib.buildPackage (
        args
        // {
          cargoArtifacts = craneLib.buildDepsOnly args;
        }
      );
    in
    {
      checks.terraria-autofish = bin;
      packages.default = bin;
      _module.args.commonDeps = commonDeps;
      devshells.default = {
        packages = [
          toolchain
        ];
      };
    };
}
