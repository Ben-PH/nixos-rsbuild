use std::path::{Path, PathBuf};

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};

use crate::flake::FlakeRef;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: SubCommand,
}
const _DEFAULT_CONFG_NIX: &str = "/etc/nixos/configuration.nix";

// TODO: split build and non-build commands
#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Build a config: Activate it, add it to boot-menu as default.
    Switch {
        #[clap(long, short = 'c')]
        specialisation: Option<String>,
        #[clap(flatten)]
        all: AllArgs,
        #[clap(flatten)]
        rb: RbFlag,
        // #[clap(flatten)]
        // flake_args: Option<FlakeBuildArgs>,
    },
    /// Build a config, add it to boot-menu as default. Will not be activated.
    Boot {
        #[clap(flatten)]
        all: AllArgs,
        #[clap(flatten)]
        rb: RbFlag,
        // #[clap(flatten)]
        // flake_args: Option<FlakeBuildArgs>,
    },
    /// Build and activate a config. Will not be added to boot-menu
    Test {
        #[clap(long, short = 'c')]
        specialisation: Option<String>,
        #[clap(flatten)]
        all: AllArgs,
        #[clap(flatten)]
        rb: RbFlag,
        // #[clap(flatten)]
        // flake_args: Option<FlakeBuildArgs>,
    },
    /// Build and sym-link only: No activation, or boot menu changes. Symlinks to configuration through `./result`.
    ///
    /// This is additional long-help
    /// This should not be shown under `-h`
    /// Only to be shown with `--help`
    /// the short help will also be shown under `--help`
    Build {
        #[clap(flatten)]
        all: AllArgs,
        #[clap(flatten)]
        rb: RbFlag,
        // #[clap(flatten)]
        // flake_args: Option<FlakeBuildArgs>,
    },
    /// Build config only. Show (possibly incomplete) list of changes that its activation
    DryActivate {
        #[clap(flatten)]
        all: AllArgs,
        // #[clap(flatten)]
        // flake_args: Option<FlakeBuildArgs>,
    },
    /// No-op, but shows the build/download ops performed by an actual build
    DryBuild {
        #[clap(flatten)]
        all: AllArgs,
        // #[clap(flatten)]
        // flake_args: Option<FlakeBuildArgs>,
    },
    /// Opens `configuration.nix` in default editor.
    Edit {
        #[clap(flatten)]
        all: AllArgs,
    },
    /// Opens the configuration using `nix repl`
    Repl {
        #[clap(flatten)]
        all: AllArgs,
    },
    BuildVm {
        #[clap(flatten)]
        all: AllArgs,
        // #[clap(flatten)]
        // flake_args: Option<FlakeBuildArgs>,
    },
    BuildVmWithBootloader {
        #[clap(flatten)]
        all: AllArgs,
        // #[clap(flatten)]
        // flake_args: Option<FlakeBuildArgs>,
    },
    /// Shows available generations. By default, similarly to boot-loader menu. Json output also
    /// available
    ListGenerations {
        #[clap(long)]
        /// Outputs generations in json format
        json: bool,
    },
}

#[derive(Args, Debug)]
pub struct RbFlag {
    #[clap(long)]
    rollback: bool,
}
#[derive(Args, Debug)]
struct SpecArg {
    specialisation: Option<String>,
}
#[derive(Args, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct AllArgs {
    /// upgrade root-users "nixos" channel, and channels containing `.update-on-nixos-rebuild`
    /// marker file in base-dir
    #[clap(long)]
    pub upgrade: bool,
    /// --upgrade, but ALL of root-users channels
    #[clap(long)]
    #[arg(conflicts_with("upgrade"))]
    pub upgrade_all: bool,
    /// (Re)Installs boot loader to device specified by relevant config options.
    #[clap(long)]
    pub install_bootloader: bool,
    /// Uses currently installed version of Nix.
    ///
    /// Normal behavior is to first build the `nixUnstable` attribute in `Nix-pkgs`, and use that.
    /// This is required when ``NixOS`` modules use features not provided by the currently installed
    /// version of Nix.
    #[clap(long, short)]
    pub no_build_nix: bool,

    /// TODO: if both flake and no-flake are unset, set flake to /etc/nixos/flake.nix, but only if
    /// that file exists...
    #[clap(long, conflicts_with_all(["file", "attr", "no_flake"]))]
    #[arg(value_parser = flake_parse)]
    pub flake: Option<FlakeRef>,
    #[clap(long)]
    pub no_flake: bool,

    /// Used to select an attrubite other than the default
    #[clap(long)]
    pub attr: Option<String>,
    // whet `--target-host` or `--build-host`, make this one availabel
    // #[clap(long, short = 's')]
    // use_substitutes: bool,
    // todo: parse as valid nix file
    #[clap(long)]
    #[arg(value_parser = file_exists)]
    /// For this build, sets the input file.
    pub file: Option<Utf8PathBuf>,
    #[clap(long = "profile_name")]
    #[arg(default_value_t = Utf8PathBuf::from(String::from("/nix/var/nix/profiles/system")), value_parser = profile_name_parse)]
    //Utf8PathBuf::from_path_buf(PathBuf::from("/nix/var/nix/profiles/system")).unwrap())]
    /// For this build, sets profile directory to `/nix/var/nix/profiles/system-profiles/$profile-name`
    pub profile_path: Utf8PathBuf,
    #[clap(long)]
    // #[arg(conflicts_with_all(["no_build_nix", "flake", "no_flake", "attr", "file"]))]
    // pub rollback: bool,
    #[clap(long)]
    pub build_host: bool,
    #[clap(long)]
    pub fast: bool,
}

// TODO: this is needed for bringing in a value parser. if you can access `try_from` directly, do
// that instead
fn flake_parse(val: &str) -> Result<FlakeRef, String> {
    FlakeRef::try_from(val)
}

#[derive(Args, Debug)]
#[allow(clippy::struct_excessive_bools)]
struct FlakeBuildArgs {
    #[clap(long)]
    recreate_lock_file: bool,
    #[clap(long)]
    no_update_lock_file: bool,
    #[clap(long)]
    no_write_lock_file: bool,
    #[clap(long)]
    no_registries: bool,
    #[clap(long)]
    commit_lock_file: bool,
    #[clap(long)]
    update_input: Option<String>,
    #[clap(long)]
    #[arg(num_args(2))]
    override_input: Option<String>,
    #[clap(long)]
    impure: bool,
}

impl AllArgs {
    pub fn building_attribute(&self) -> bool {
        self.file.is_some() || self.attr.is_some()
    }
}

fn profile_name_parse(prof_name: &str) -> Result<Utf8PathBuf, String> {
    let root_str = "/nix/var/nix/profiles";
    let prof_root = Path::new(root_str);
    let mut path_buff = PathBuf::from(prof_root);
    path_buff.push("system-profiles");
    path_buff.push(prof_name);
    Utf8PathBuf::from_path_buf(path_buff)
        .map_err(|_| format!("Cannot construct utf8-path from {}/{}", root_str, prof_name))
}
fn file_exists(path: &str) -> Result<Utf8PathBuf, String> {
    let path = Utf8PathBuf::from_path_buf(PathBuf::from(path)).map_err(|_| path)?;
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.as_str()));
    }
    if !path.is_file() {
        return Err(format!("Not a file: {}", path.as_str()));
    }
    if !matches!(path.extension(), Some("nix")) {
        return Err("Requires '.nix' extension".to_string());
    }
    Ok(path)
}

impl SubCommand {
    /// all but list-gens contains `AllArgs`.
    /// If this is already known not to be a ``ListGenerations`` run, you can unwrap this no problem.
    /// ...And that should be a flag to clean up the arg architecture, no?
    fn inner_args_mut(&mut self) -> Option<&mut AllArgs> {
        match self {
            SubCommand::Switch { all, .. }
            | SubCommand::Boot { all, .. }
            | SubCommand::Test { all, .. }
            | SubCommand::Build { all, .. }
            | SubCommand::DryActivate { all }
            | SubCommand::DryBuild { all }
            | SubCommand::Edit { all }
            | SubCommand::Repl { all }
            | SubCommand::BuildVm { all }
            | SubCommand::BuildVmWithBootloader { all } => Some(all),
            SubCommand::ListGenerations { .. } => None,
        }
    }
    pub fn inner_args(&self) -> Option<&AllArgs> {
        match self {
            SubCommand::Switch { all, .. }
            | SubCommand::Boot { all, .. }
            | SubCommand::Test { all, .. }
            | SubCommand::Build { all, .. }
            | SubCommand::DryActivate { all }
            | SubCommand::DryBuild { all }
            | SubCommand::Edit { all }
            | SubCommand::Repl { all }
            | SubCommand::BuildVm { all }
            | SubCommand::BuildVmWithBootloader { all } => Some(all),
            SubCommand::ListGenerations { .. } => None,
        }
    }
    pub fn building_attr(&self) -> bool {
        self.inner_args().is_some_and(AllArgs::building_attribute)
    }
    pub fn _rollback(&self) -> bool {
        matches!(
            self,
            SubCommand::Switch {
                rb: RbFlag { rollback: true },
                ..
            } | SubCommand::Boot {
                rb: RbFlag { rollback: true },
                ..
            } | SubCommand::Test {
                rb: RbFlag { rollback: true },
                ..
            } | SubCommand::Build {
                rb: RbFlag { rollback: true },
                ..
            }
        )
    }
    pub fn can_run(&self) -> bool {
        matches!(
            self,
            Self::Switch { .. } | Self::Boot { .. } | Self::Test { .. }
        )
    }
    /// If there's a /etc/nixos/flake.nix (or canonicalised version), sets the flake path if not
    /// set already, derives a default attribute.
    pub fn try_init_to_default_flake(&mut self) {
        // Verify we are happy to flake...
        let Some(AllArgs {
            ref mut flake,
            no_flake: false,
            ..
        }) = self.inner_args_mut()
        else {
            log::trace!("no-flake set: not attempting to set flake");
            return;
        };
        log::trace!("Happy to flake...");

        // Verify the flake isn't already set
        let None = flake else {
            log::trace!("flake already set: not updating");
            return;
        };
        log::trace!("no flake set: attempting to derive default...");

        // Happy to flake, no flake set: Map in the flake if it exists
        let Some(path) = FlakeRef::canonned_default_dir() else {
            log::trace!("Could not find a flake: no flake set");
            return;
        };
        log::trace!("Derived flake path: {}", path.display());
        // We pulled out a default flake, now let's get its attr
        let attr = Some(crate::flake::FlakeAttr::default());
        let new_flake = FlakeRef {
            source: path,
            output_selector: attr,
        };
        log::trace!("Setting new flake: {:?}", new_flake);

        *flake = Some(new_flake);
    }
    fn build_nix(&self) -> bool {
        todo!()
    }
}
