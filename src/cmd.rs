use std::io::{self, ErrorKind};

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};

use crate::flake::FlakeRefInput;

mod parsers;
mod stashed;
/// Implementations for carrying out the various tasks
mod handlers;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// Entry point into the cli application.
pub struct Cli {
    #[command(subcommand)]
    pub command: SubCommand,
}

/// Foobarbaz
#[derive(Subcommand, Debug)]
pub enum SubCommand {
    Builders {
        #[command(subcommand)]
        task: BuildSubComms,
        #[clap(flatten)]
        arg: AllArgs,
    },
    Util {
        #[command(subcommand)]
        task: UtilSubCommand,
    },
}

/// Build-oriented tasks. See its `-h`/`--help` for more info.
#[derive(Subcommand, Debug, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum BuildSubComms {
    /// Build and... Activate. Start using it, but won't be part of next reboot.
    Test,
    /// Build and... BootMenu. Will be default config on next reboot.
    Boot,
    /// Build and... Activate + BootMenu. Analagous to `test + boot` runs
    Switch,
    /// Build and... Nothing. Makes sym-link to nix-store entry: `./result` by default
    ///
    /// Use `--res-dir` to override default directory in which the `result` symlink will be placed
    Build,
    /// Build config only. Show (possibly incomplete) list of changes that its activation
    DryActivate,
    /// No-op, but shows the build/download ops performed by an actual build
    DryBuild,
    /// Un-tested. Use at own risk. 
    BuildVm,
    /// Un-tested. Use at own risk. 
    BuildVmWithBootloader,
}

/// Tools-oriented tasks. See its `-h`/`--help` for more info.
#[derive(Subcommand, Debug)]
pub enum UtilSubCommand {

    /// Output available generations.
    ListGenerations {
        #[clap(long)]
        /// Outputs generations in json format
        json: bool,
    },
    // /// Opens `configuration.nix` in default editor.
    // Edit {
    //     #[clap(flatten)]
    //     all: AllArgs,
    // },

    // /// Opens the configuration using `nix repl`
    // Repl {
    //     #[clap(flatten)]
    //     all: AllArgs,
    // },
}

#[derive(Args, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct AllArgs {
    // #[clap(long)]
    // /// Unimplemented
    // ///
    // /// upgrade root-users "nixos" channel, and channels containing `.update-on-nixos-rebuild`
    // /// marker file in base-dir
    // pub upgrade: bool,
    
    // #[clap(long)]
    // #[arg(conflicts_with("upgrade"))]
    // /// Unimplemented
    // ///
    // /// --upgrade, but ALL of root-users channels
    // pub upgrade_all: bool,

    // #[clap(long)]
    // /// (Re)Installs boot loader to device specified by relevant config options.
    // pub install_bootloader: bool,

    // #[clap(long, short)]
    // /// Uses currently installed version of Nix.
    // ///
    // /// Normal behavior is to first build the `nixUnstable` attribute in `Nix-pkgs`, and use that.
    // /// This is required when ``NixOS`` modules use features not provided by the currently installed
    // /// version of Nix.
    // pub no_build_nix: bool,

    
    #[clap(long, conflicts_with_all(["file"]))] //, "attr", "no_flake"]))]
    #[arg(value_parser = parsers::flake_parse, default_value_t = FlakeRefInput::try_default().unwrap())]
    #[arg(name = "FLAK_REF")]
    /// Explicitly define the flake path: Typically `.#<hostname>`
    pub flake: FlakeRefInput,

    // #[clap(long)]
    // pub no_flake: bool,

    // #[clap(long)]
    // /// Used to select an attrubite other than the default
    // pub attr: Option<String>,

    #[clap(long)]
    /// For this build, sets the input file.
    pub res_dir: Option<Utf8PathBuf>,

    // #[clap(long, short = 's')]
    // when `--target-host` or `--build-host`, make this one availabel
    // use_substitutes: bool,

    #[clap(long)]
    #[arg(value_parser = nix_file_exists)]
    /// For this build, sets the input file.
    pub file: Option<Utf8PathBuf>,

    // #[clap(long = "profile_name")]
    // #[arg(default_value_t = Utf8PathBuf::from(String::from("/nix/var/nix/profiles/system")), value_parser = parsers::profile_name_parse)]
    // /// For this build, sets profile directory to `/nix/var/nix/profiles/system-profiles/$profile-name`
    // pub profile_path: Utf8PathBuf,

    // #[clap(long)]
    // #[arg(conflicts_with_all(["no_build_nix", "flake", "no_flake", "attr", "file"]))]
    // pub rollback: bool,

    // #[clap(long)]
    // pub build_host: bool,

    // #[clap(long)]
    // pub fast: bool,
}

fn nix_file_exists(path: &str) -> io::Result<Utf8PathBuf> {
    let path = Utf8PathBuf::from(path);
    if !path.exists() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("File does not exist: {}", path.as_str()),
        ));
    }
    if !path.is_file() {
        return Err(io::Error::new(
            ErrorKind::Other,
            format!("Not a file: {}", path.as_str()),
        ));
    }
    if !matches!(path.extension(), Some("nix")) {
        return Err(io::Error::new(
            ErrorKind::Other,
            "Requires '.nix' extension".to_string(),
        ));
    }
    Ok(path)
}
