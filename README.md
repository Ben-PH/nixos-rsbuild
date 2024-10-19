
`nixos-rsbuild` is a (slightly opinionated) rewrite of the `nixos-rebuild` cli utility. The goals are as follows

 - minimal barrier-of-entry to read/write the codebase in a meaningful way
 - provide a pleasant documentation/help UX
 - showcase Rust as an implementation tool in the nix/nixos ecosystem
 - (Initially): Stand-in replacement for most of my build/switch/etc. needs
 - (Stretch): Cover all supported use-cases of `nixos-rebuild`


 #### Opinions

 - (TODO) Will fail if cannot find a `flake.nix` unless `--no-flake` is used
 - Makes more args/flags mutually exclusive. `--upgrade-all` implios `--upgrade`, so providing both will be an error.
 - Aims to be relatively platform agnostic.
 - no `sudo`: can only run as non-root user. in future, will add `--sudo | -s` flag to relevant subcommands in place of `--use-remote-sudo`

#### Usage:

`cargo run -- <args>`

- `cargo run -- -h` show top level help
- `cargo run -- --help` show top level help in long-form
- `cargo run -- <subcommand> -h` show subcommand help
- `cargo run -- <subcommand> --help` show subcommand help in long-form
 
#### For the "Rust Curious/Skeptic"

If you are reading this, you are probably a Nix/NixOS nerd. If you are curious and/or skeptical about Rust, particularly in the Nix/NixOS ecosystem, I _strongly_ encourage you to ask questions, express your skepticism, and other wise promote lively discussion we can all enjoy.

#### Roadmap

NOTE: `<flake-uri>` => `/path/to/dir#flake.attr`

1. [ ] `nixos-rsbuild list-generations [--json]`
2. [ ] `nixos-rebuild boot --flake <flake-uri>` -> `nixos-rs boot <flake-uri>`
3. [ ] `nixos-rebuild boot` -> `nixos-rsbuild boot --config`
4. [ ] `nixos-rebuild test ...` -> `nixos-rsbuild test ...`
5. [ ] `nixos-rebuild switch ...` -> `nixos-rsbuild switch ...`
6. [ ] `nixos-rebuild switch | boot | test ... --use-remote-sudo` -> `nixos-rsbuild <switch | boot | test> -s ...`

#### TODOs

 - [ ] split out things into a library to leverage `cargo doc`
 - [ ] add `no_op` feature so as to only emulate changes in the output
 - [ ] black-box testing
 - [ ] white-box testing
 - [ ] system-level testing
 - [ ] non-local target-aware
 - [ ] non-local builde-aware

