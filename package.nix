{
  lib,
  rustPlatform,
  rev ? null,
  project ? (lib.importTOML ./Cargo.toml).package,
}:

rustPlatform.buildRustPackage {
  pname = project.name;
  version = project.version + lib.optionalString (rev != null) rev;

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

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
