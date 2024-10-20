{
  lib,
  rustPlatform,
  rev ? null,
  project ? (lib.importTOML ./Cargo.toml).package,
}:

rustPlatform.buildRustPackage rec {
  pname = project.name;
  version = project.version + lib.optionalString (rev != null) "-${rev}";

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

  patchPhase = ''
    substituteInPlace Cargo.toml --replace 'version = "${project.version}"' 'version = "${version}"'
  '';

  meta = {
    description = "A slightly opinionated RIIR of the nixos-rebuild CLI-tool";
    longDescription = project.description;
    homepage = "https://github.com/Ben-PH/nixos-rsbuild";
    maintainers = with lib.maintainers; [
      "benphawke@gmail.com"
      confus
    ];
    license = lib.licenses.mit;
    mainProgram = "nixos-rsbuild";
  };
}
