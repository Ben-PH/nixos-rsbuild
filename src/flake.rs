use crate::{cmd::SubCommand, run_cmd};
use camino::Utf8PathBuf;
use flake_path::FlakeDir;
use std::{
    ffi::OsStr,
    fmt::Display,
    io,
    path::{Path, PathBuf},
    process::Command as CliCommand,
};

mod attribute;
mod flake_path;
pub use attribute::FlakeAttr;

const DEFAULT_FILE_DIR: &str = "/etc/nixos";
const DEFAULT_FLAKE_NIX: &str = "/etc/nixos/flake.nix";

/// Destructured `<flake_dir>[#attribute]`
#[derive(Debug, Clone)]
pub struct FlakeRefInput {
    /// Pre-`#` component.
    /// Path to the dir where a flake.nix will be searched
    pub source: PathBuf,
    /// Post-`#` component
    pub output_selector: Option<FlakeAttr>,
}

impl Display for FlakeRefInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let attr_path = self
            .output_selector
            .as_ref()
            .map(|a| format!("#{}", a))
            .unwrap_or_default();
        write!(f, "{}{}", self.source.display(), attr_path)
    }
}

#[derive(Debug, Clone)]
pub struct FlakeRef {
    /// Pre-`#` component.
    /// Path to the dir where a flake.nix will be searched
    pub source: flake_path::FlakeDir<PathBuf>,
    /// Post-`#` component
    pub output_selector: FlakeAttr,
}

impl Display for FlakeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.source.as_ref().display(), self.output_selector)
    }
}

impl Default for FlakeRefInput {
    fn default() -> Self {
        Self {
            source: PathBuf::from(DEFAULT_FILE_DIR),
            output_selector: Some(FlakeAttr::default()),
        }
    }
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
    pub fn init_flake_ref(&self) -> io::Result<FlakeRef> {
        let path = FlakeDir::try_from_path(&self.source)?;
        let mut attr = self.output_selector.clone().unwrap_or_default();
        attr.route_to_toplevel();

        Ok(FlakeRef {
            source: path,
            output_selector: attr,
        })
    }

    /// where `realpath /etc/nixos/flake.nix` resolves to a `flake.nix` file, provides the path to
    /// the directory
    pub fn canoned_default_dir() -> Option<PathBuf> {
        match std::fs::canonicalize(DEFAULT_FLAKE_NIX) {
            Ok(c) => {
                if c.is_dir() {
                    log::error!("Canonical path from default flake.nix should resolve to a flake.nix. Resolved to: {}", c.display());
                    None
                } else if c
                    .file_name()
                    .expect("somehow symlink of default flake.nix resolved to `..`")
                    != OsStr::new("flake.nix")
                {
                    log::error!("Canonical path from default flake.nix resolved to file other than `flake.nix`: {}", c.display());
                    None
                } else {
                    Some(c)
                }
            },
            Err(e) => {
                log::error!("Error canonicalising default flake-path: {}", e);
                None
            }
        }
    }
}

/// Takes a string and maps it to a flake-ref
impl TryFrom<&str> for FlakeRefInput {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // no '#'? we just have the source, no selected attr
        let Some(fst_hash) = value.find('#') else {
            return Ok(FlakeRefInput {
                source: Path::new(value).to_path_buf(),
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
            source: PathBuf::from(path),
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
