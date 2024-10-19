{
  description = "A slightly opinionated RIIR of the nixos-rebuild CLI-tool";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      forSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" ]; # Add more when actullay testing for those architecutres
      rev = toString (self.shortRev or self.dirtyShortRev or self.lastModifiedDate or "unknown");
    in
    {
      packages = forSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.callPackage ./package.nix { inherit rev; };
        }
      );
    };
}
