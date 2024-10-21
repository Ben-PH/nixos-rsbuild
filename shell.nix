{
  pkgs ? import <nixpkgs> {
    overlays = [ ];
    config = { };
  },
}:
pkgs.mkShell {
  packages = with pkgs; [
    cargo
    rustc
  ];
  env = {
    # RUST_BACKTRACE = "full";
  };
  shelHook = ''

  '';
}
