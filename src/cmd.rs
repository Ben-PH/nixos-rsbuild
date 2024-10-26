use std::io::{self, ErrorKind};

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use tempdir::TempDir;

use crate::flake::FlakeRefInput;

mod parsers;
mod stashed;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: SubCommand,
}

// TODO: split build and non-build commands
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

/// Commands that involve building: switch, test, boot, etc..
#[derive(Subcommand, Debug, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum BuildSubComms {
    /// Build a config: Activate it, add it to boot-menu as default.
    Switch,
    /// Build a config, add it to boot-menu as default. Will not be activated.
    Boot,
    /// Build and activate a config. Will not be added to boot-menu
    Test,
    /// Build and sym-link only: No activation, or boot menu changes. Symlinks to configuration through `./result`.
    ///
    /// This is additional long-help
    /// This should not be shown under `-h`
    /// Only to be shown with `--help`
    /// the short help will also be shown under `--help`
    Build,
    /// Build config only. Show (possibly incomplete) list of changes that its activation
    DryActivate,
    /// No-op, but shows the build/download ops performed by an actual build
    DryBuild,
    BuildVm,
    BuildVmWithBootloader,
}

impl BuildSubComms {
    /// Builds a config, capturing a sym-link. Follows up with a call to `switch-to-configuration`
    /// as appropriate.
    pub fn run_build(&self, args: AllArgs) -> io::Result<()> {
        log::trace!("Constructing configuration");
        let use_td = args.res_dir.is_none();
        let full_flake = args.flake.init_flake_ref(self)?;
        let res_dir = args.res_dir.unwrap_or(
            Utf8PathBuf::from_path_buf(TempDir::new("nixrsbuild-")?.into_path()).unwrap(),
        );
        log::trace!("Result link directory: {}", res_dir);

        full_flake.run_nix_build(res_dir.as_path())?;
        if matches!(
            self,
            BuildSubComms::Switch
                | BuildSubComms::Boot
                | BuildSubComms::Test
                | BuildSubComms::DryActivate
        ) {
            let switch_bin = res_dir.join("result/bin/switch-to-configuration");
            let out_link = std::fs::canonicalize(switch_bin).unwrap();

            let local_arch = std::env::var_os("LOCALE_ARCHIVE").unwrap_or_default();
            let task_str = self.to_string();
            let _ = cmd_lib::spawn!(
                sudo nu -c "env -i LOCALE_ARCHIVE=$local_arch $out_link $task_str"
            )?
            .wait();
        }

        if use_td {
            let sys_td = std::env::temp_dir();
            assert!(std::fs::exists(&sys_td).unwrap());
            assert!(res_dir.starts_with(sys_td));
            assert!(
                res_dir.file_name().unwrap().starts_with("nixrsbuild-"),
                "{}",
                res_dir.file_name().unwrap()
            );
            let _ = std::fs::remove_dir_all(res_dir);
        }

        Ok(())
    }
}

/// Commands such as `repl`, `list-generations`, etc
#[derive(Subcommand, Debug)]
pub enum UtilSubCommand {
    ListGenerations {
        #[clap(long)]
        /// Outputs generations in json format
        json: bool,
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
    #[arg(value_parser = parsers::flake_parse, default_value_t = FlakeRefInput::try_default().unwrap())]
    pub flake: FlakeRefInput,
    #[clap(long)]
    pub no_flake: bool,

    /// Used to select an attrubite other than the default
    #[clap(long)]
    pub attr: Option<String>,
    #[clap(long)]
    /// For this build, sets the input file.
    pub res_dir: Option<Utf8PathBuf>,
    // whet `--target-host` or `--build-host`, make this one availabel
    // #[clap(long, short = 's')]
    // use_substitutes: bool,
    // todo: parse as valid nix file
    #[clap(long)]
    #[arg(value_parser = nix_file_exists)]
    /// For this build, sets the input file.
    pub file: Option<Utf8PathBuf>,
    #[clap(long = "profile_name")]
    #[arg(default_value_t = Utf8PathBuf::from(String::from("/nix/var/nix/profiles/system")), value_parser = parsers::profile_name_parse)]
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
