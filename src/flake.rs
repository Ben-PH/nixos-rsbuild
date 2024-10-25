use camino::{Utf8Path, Utf8PathBuf};
use flake_path::FlakeDir;
use std::{
    ffi::OsStr,
    fmt::Display,
    io::{self, ErrorKind},
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
            if !attr_path.is_empty() {
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
            if !output.is_empty() {
                write!(f, "#{}", output)?;
            }
        }
        Ok(())
    }
}

impl FlakeRef {
    pub fn run_nix_build(&self, out_dir: &Utf8Path) -> io::Result<()> {
        log::info!("Building in flake mode.");

        let refstr = self.to_string();
        let resfile = out_dir.join("result");
        cmd_lib::spawn!(nix  build "$refstr" --out-link "$resfile")
            .unwrap()
            .wait()
            .unwrap();
        Ok(())
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

        let mut attr = self
            .output_selector
            .clone()
            .unwrap_or(FlakeAttr::try_default()?);

        attr.set_config()?;
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
    pub fn try_default() -> io::Result<Self> {
        Ok(Self {
            source: Utf8PathBuf::from(crate::utils::DEFAULT_FILE_DIR),
            output_selector: Some(FlakeAttr::try_default()?),
        })
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
        let attr =
            FlakeAttr::try_from(stripped_attr.to_string()).map_err(|_e| value.to_string())?;

        // jobs-done!
        Ok(FlakeRefInput {
            source: Utf8PathBuf::from(path),
            output_selector: Some(attr),
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn flake_ref_display() {
        let mut data = FlakeRef {
            source: FlakeDir {
                canoned_dir: Utf8PathBuf::from("/fizz/buzz"),
            },
            output_selector: None,
        };

        assert_eq!(format!("{}", data), "/fizz/buzz");
        data.output_selector = Some(FlakeAttr { attr_path: vec![] });
        assert_eq!(format!("{}", data), "/fizz/buzz");
        data.output_selector = Some(FlakeAttr {
            attr_path: vec!["foo".into()],
        });
        assert_eq!(format!("{}", data), "/fizz/buzz#foo");
        data.output_selector = Some(FlakeAttr {
            attr_path: vec!["foo".into(), "bar".into()],
        });
        assert_eq!(format!("{}", data), "/fizz/buzz#foo.bar");
        data.source.canoned_dir = Utf8PathBuf::from("/bop/pow/");
        assert_eq!(format!("{}", data), "/bop/pow/#foo.bar");
    }

    #[test]
    fn flake_ref_try_from() {
        let assert = |s| {
            let data = FlakeRefInput::try_from(s).unwrap();
            assert_eq!(data.to_string(), s);
        };

        assert("/fizz/buzz");
        assert("/fizz/buzz/");
        assert("/fizz/buzz#foo");
        assert("/fizz/buzz#foo.bar");
        assert!(FlakeRefInput::try_from("/fizz/buzz#").is_err());
        assert!(FlakeRefInput::try_from("/fizz/buzz#foo#").is_err());
        assert!(FlakeRefInput::try_from(r#"/fizz/buzz#foo""#).is_err());
    }
}
