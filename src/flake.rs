use crate::{cmd::SubCommand, run_cmd};
use camino::Utf8PathBuf;
use std::{
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::Command as CliCommand,
};

mod attribute;
pub use attribute::FlakeAttr;

const DEFAULT_FLAKE_NIX: &str = "/etc/nixos/flake.nix";
/// Destructured `<flake_dir>[#attribute]`
#[derive(Debug, Clone)]
pub struct FlakeRefInput {
    /// Pre-`#` component.
    /// Path to the dir where a flake.nix will be searched
    pub source: Option<PathBuf>,
    /// Post-`#` component
    pub output_selector: Option<FlakeAttr>,
}
#[derive(Debug, Clone)]
pub struct FlakeRef {
    /// Pre-`#` component.
    /// Path to the dir where a flake.nix will be searched
    pub source: PathBuf,
    /// Post-`#` component
    pub output_selector: Option<FlakeAttr>,
}

impl FlakeRefInput {
    /// nixos-rsbuild will flakebuild, unless explicitly stated with the --no-flake flag
    ///
    /// # No path stated in flake-ref
    /// - sets `dirname (realpath /etc/nixos/flake.nix)` for path (i.e. the dir containing the sym-link
    /// # No Attribute stated in the flake-ref
    /// - sets attr to `nixosConfigurations.<hostname>.config.system.build.toplevel`
    /// - Attempts to derive `<hostname>` from content of `/proc/sys/kernel/hostname`
    /// - Falls back to `default` for the `<hostname>`
    ///
    /// # Error
    /// - No `/etc/nixos/flake.nix` present
    ///
    /// TODO: Error-out if derived hostname is not present in `nixosConfigurations`
    pub fn init_flake_Ref(mut self) -> Result<FlakeRef, Self> {
        log::debug!("initialisation unimplemented");
        Err(self)
    }

    /// where `realpath /etc/nixos/flake.nix` resolves to a `flake.nix` file, provides the path to
    /// the directory
    pub fn canoned_default_dir() -> Option<PathBuf> {
            match std::fs::canonicalize("/etc/nixos/flake.nix") {
                Ok(c) => {
                    if c.is_dir() {
                        log::error!("Canonical path from default flake.nix should resolve to a flake.nix. Resolved to: {}", c.display());
                        return None;
                    } else if c
                        .file_name()
                        .expect("somehow symlink of default flake.nix resolved to `..`")
                        != OsStr::new("flake.nix")
                    {
                        log::error!("Canonical path from default flake.nix resolved to file other than `flake.nix`: {}", c.display());
                        return None;
                    }
                    return Some(c);
                }
                Err(e) => {
                    log::error!("Error canonicalising default flake-path: {}", e);
                    return None;
                }
            }
    }
    /// Resolves symbolic links in flakeref path:
    /// path is not a dir: None
    /// path does not contain flake.nix: None
    /// contained flake.nix is a regular file: returned path matches current
    /// cantained flake.nix sym-links to anything other than regular file named `flake.nix`: None
    /// contained flake.nix links to regular file named `flake.nix`: said files parent directory
    pub fn canoned_dir(&self) -> Option<PathBuf> {
        let Some(input_path) = &self.source else {
            // get dir of default flake, resolving sym-links as needed
            return Self::canoned_default_dir();
        };

        if !input_path.is_dir() {
            log::error!("Expected directory, got file: {}", input_path.display());
            return None;
        }

        let flake_loc = input_path.join("flake.nix");
        let flake_exists = match std::fs::exists(&flake_loc)  {
            Ok(exists) => exists,
            Err(e) => {
                log::error!("Error when checking for existence of flake at {}: {}", flake_loc.display(), e);
                return None;
            }
        };
    
        if !flake_exists {
            log::error!("flake-path must be a directory containing `flake.nix`.");
            return None;
        }

        let canoned_path = match std::fs::canonicalize(&flake_loc) {
            Ok(path) => path,
            Err(e) => {
                log::error!("Could not canonicalise path {}: {}", flake_loc.display(), e);
                return None;
            }
        };
        if canoned_path.is_dir() {
                log::error!("Sym-link from {} must resolve to `flake.nix`. Resolved to a directory: {}", flake_loc.display(), canoned_path.display());
                return None;
        }
        if canoned_path.file_name() != Some(OsStr::new("flake.nix")) {
                log::error!("Sym-link from {} must resolve to a `flake.nix`. Resolved to: {}", flake_loc.display(), canoned_path.display());
                return None;
        }

        let res = canoned_path.parent().map(Path::to_path_buf);
        if res.is_none() {
            log::error!("Could not resolve directory from {}", canoned_path.display())
        }

        res
    }
}

/// Takes a string and maps it to a flake-ref
impl TryFrom<&str> for FlakeRefInput {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // no '#'? we just have the source, no selected attr
        let Some(fst_hash) = value.find('#') else {
            return Ok(FlakeRefInput {
                source: Some(Path::new(value).to_path_buf()),
                output_selector: None,
            });
        };

        // split "foo#path.to.bar" into ["foo", "path.to.bar"]
        let (path, hsh_attr) = value.split_at(fst_hash);
        let stripped_attr = &hsh_attr[1..];

        // parse "bar" into `FlakeAttr(["path","to", "bar"])`
        let Ok(attr) = FlakeAttr::try_from(stripped_attr.to_string()) else {
            return Err(value.to_string());
        };

        // jobs-done!
        Ok(FlakeRefInput {
            source: Some(PathBuf::from(path)),
            output_selector: Some(attr),
        })
    }
}

pub fn flake_build_config(sub_cmd: &SubCommand, args: &[&str]) -> io::Result<Utf8PathBuf> {
    log::info!("Building in flake mode.");
    if !sub_cmd.building_attr()
        && !matches!(
            sub_cmd,
            SubCommand::Switch { .. }
                | SubCommand::Boot { .. }
                | SubCommand::Test { .. }
                | SubCommand::DryActivate { .. }
        )
    {
        log::trace!("Not building attre: just run nix build {}", args.join(" "));
        // nix flake build, e.g. nixos-rebuild build --flake .#username
        log::trace!("flake args: {:?}", sub_cmd.inner_args().unwrap().flake);
        let mut cmd = CliCommand::new("nom");
        cmd.arg("build").args(args);
        let _ = run_cmd(&mut cmd);
        // let _sym_link_to_result = std::fs::canonicalize(todo!("tmpDir joined with /result"))?;
    } else if !sub_cmd.inner_args().is_some_and(|a| a.build_host) {
        let mut cmd = CliCommand::new("nom");
        cmd.arg("build").args(args).arg("--out-link");
        // .arg(todo!("tmpDir joined with /result"))
        let _ = run_cmd(&mut cmd);
        // let _sym_link_to_result = std::fs::canonicalize(todo!("tmpDir joined with /result"))?;
    } else {
        let (attr, _args) = (args[0], &args[1..]);
        // TODO: bring in FlakeArgs pass-through
        let mut cmd = CliCommand::new("nom");
        cmd.args(["eval", "--raw", &format!("{}.drv", attr)]);
        let drv = run_cmd(&mut cmd)?.stdout;
        if !String::from_utf8(drv).is_ok_and(|s| Path::new(&s).exists()) {
            eprintln!("nix eval failed");
            std::process::exit(1);
        }
    }
    todo!()
}
