{
  description = "A slightly opinionated RIIR of the nixos-rebuild CLI-tool";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs, ... }:
    let
      rev = toString (self.shortRev or self.dirtyShortRev or self.lastModifiedDate or "unknown");

      # ToDo: Add more when actually testing for those architectures
      supportedSystems = [ "x86_64-linux" ];

      forSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      packages = forSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = self.outputs.packages.${system}.nixos-rsbuild;
          nixos-rsbuild = pkgs.callPackage ./package.nix { inherit rev; };
        }
      );

      devShells = forSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = self.outputs.devShells.${system}.minimal;
          minimal = import ./shell.nix { inherit pkgs; };
        }
      );

      overlays.default = final: prev: {
        nixos-rsbuild = final.callPackage ./package.nix { };
      };

      nixosModules.default = ./module.nix;

      formatter = forSystems (system: nixpkgs.legacyPackages.${system}.nixfmt-rfc-style);
    };
}
