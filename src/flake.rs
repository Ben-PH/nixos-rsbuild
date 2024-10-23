use crate::{cmd::SubCommand, run_cmd};
use camino::{Utf8Path, Utf8PathBuf};
use flake_path::FlakeDir;
use std::{
    ffi::OsStr,
    fmt::Display,
    io::{self, ErrorKind},
    path::Path,
    process::Command as CliCommand,
};

mod attribute;
mod flake_path;
pub use attribute::FlakeAttr;

/// Destructured `<flake_dir>[#attribute]`
#[derive(Debug, Clone)]
pub struct FlakeRefInput {
    /// Pre-`#` component.
    /// Path to the dir where a flake.nix will be searched
    pub source: Utf8PathBuf,
    /// Post-`#` component
    pub output_selector: Option<FlakeAttr>,
}

impl Display for FlakeRefInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)?;
        if let Some(attr_path) = &self.output_selector {
            if attr_path.len() > 0 {
                write!(f, "#{}", attr_path)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FlakeRef {
    /// Pre-`#` component.
    /// Path to the dir where a flake.nix will be searched
    pub source: flake_path::FlakeDir<Utf8PathBuf>,
    /// Post-`#` component
    pub output_selector: Option<FlakeAttr>,
}

impl Display for FlakeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)?;
        if let Some(output) = &self.output_selector {
            if output.len() > 0 {
                write!(f, "#{}", output)?;
            }
        }
        Ok(())
    }
}

impl FlakeRef {
    pub fn build(&self, out_dir: Option<&Utf8Path>) -> io::Result<Utf8PathBuf> {
        log::info!("Building in flake mode.");

        if let Some(out_dir) = out_dir {
            if !out_dir.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("requested out dir not a directory: {}", out_dir),
                ));
            }
        }

        let out_dir = out_dir.unwrap_or(Utf8Path::new(".")).join("result");
        let mut cmd = CliCommand::new("nom");
        cmd.args([
            "build",
            self.to_string().as_str(),
            "--out-link",
            out_dir.as_str(),
        ]);
        run_cmd(&mut cmd);
        let path = std::fs::canonicalize(out_dir)?;
        Utf8PathBuf::from_path_buf(path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("invalud utf in canonicalised path {}", e.display()),
            )
        })
    }
}

// impl Display for FlakeRef {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}#{}", self.source.as_ref(), self.output_selector)
//     }
// }

impl Default for FlakeRefInput {
    fn default() -> Self {
        Self {
            source: Utf8PathBuf::from(crate::utils::DEFAULT_FILE_DIR),
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
            output_selector: Some(attr),
        })
    }

    /// where `realpath /etc/nixos/flake.nix` resolves to a `flake.nix` file, provides the path to
    /// the directory
    pub fn canoned_default_dir() -> io::Result<Utf8PathBuf> {
        let dir =
            Utf8PathBuf::from_path_buf(std::fs::canonicalize(crate::utils::DEFAULT_FLAKE_NIX)?)
                .map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("Canonicalised path {} not valid Utf8", e.display()),
                    )
                })?;
        if dir.is_dir() {
            Err(io::Error::new(ErrorKind::Other, format!("Canonical path from default flake.nix should resolve to a flake.nix. Resolved to: {}", dir)))
        } else if dir
            .file_name()
            .expect("somehow symlink of default flake.nix resolved to `..`")
            != OsStr::new("flake.nix")
        {
            Err(io::Error::new(ErrorKind::Other, format!("Canonical path from default flake.nix resolved to file other than `flake.nix`: {}", dir)))
        } else {
            Ok(dir)
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
                source: Utf8PathBuf::from(value),
                output_selector: None,
            });
        };

        // split "foo#path.to.bar" into ["foo", "path.to.bar"]
        let (path, hsh_attr) = value.split_at(fst_hash);
        let stripped_attr = &hsh_attr[1..];

        // parse "bar" into `FlakeAttr(["path","to", "bar"])`
        let attr = FlakeAttr::try_from(stripped_attr.to_string()).map_err(|e| value.to_string())?;

        // jobs-done!
        Ok(FlakeRefInput {
            source: Utf8PathBuf::from(path),
            output_selector: Some(attr),
        })
    }
}
