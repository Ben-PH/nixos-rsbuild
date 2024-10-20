{
  pkgs,
  config,
  lib,
  ...
}:
{
  options.programs.nixos-rsbuild.enable = lib.mkEnableOption "";

  config = lib.mkIf config.programs.nixos-rsbuild.enable {
    environment.systemPackages = [
      (pkgs.callPackage ./package.nix { })
    ];
  };
}
