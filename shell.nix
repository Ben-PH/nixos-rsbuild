{ pkgs ? import <nixpkgs> { overlays=[]; config={}; }
}:

pkgs.mkShell {
  packages = with pkgs; [
    cargo rustc
    rust-analyzer
    rustfmt
    clippy
    # pkg-config
    # openssl
  ];

  env = {
    # RUST_BACKTRACE = "full";
    # RUST_SRC_PATH = "";
  };

  shellHook = ''
    echo "Welcome to nixos-rsbuild dev shell!" 1>&2
  '';
}
