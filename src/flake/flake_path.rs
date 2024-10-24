use std::{ffi::OsStr, fmt::Display, io};

use camino::{Utf8Path, Utf8PathBuf};

#[derive(Debug, Clone)]
pub struct FlakeDir<T: AsRef<Utf8Path>> {
    canoned_dir: T,
}

impl<T: AsRef<Utf8Path>> Display for FlakeDir<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canoned_dir.as_ref())
    }
}
impl<T: AsRef<Utf8Path>> AsRef<Utf8Path> for FlakeDir<T> {
    fn as_ref(&self) -> &Utf8Path {
        self.canoned_dir.as_ref()
    }
}

impl FlakeDir<Utf8PathBuf> {
    /// we don't use std-lib here because `impl<T: AsRef<Utf8Path>> TryFrom<T> for ...` is not doable
    /// without some esoteric magic.
    ///
    /// Resolves symbolic links in flakeref path:
    /// path is not a dir: None
    /// path does not contain flake.nix: None
    /// contained flake.nix is a regular file: returned path matches current
    /// cantained flake.nix sym-links to anything other than regular file named `flake.nix`: None
    /// contained flake.nix links to regular file named `flake.nix`: said files parent directory
    pub fn try_from_path<T: AsRef<Utf8Path>>(value: T) -> io::Result<Self> {
        if !value.as_ref().is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Is not a dir: {}", value.as_ref()),
            ));
        }

        let flake_loc = value.as_ref().join("flake.nix");
        let flake_exists = std::fs::exists(&flake_loc).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Error when checking for existence of flake at {}: {}",
                    flake_loc, e
                ),
            )
        })?;

        if !flake_exists {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "flake-path must be a directory containing `flake.nix`.",
            ));
        }

        let canoned_path = std::fs::canonicalize(&flake_loc).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Could not canonicalise path {}: {}", flake_loc, e),
            )
        })?;

        if canoned_path.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Sym-link from {} must resolve to `flake.nix`. Resolved to a directory: {}",
                    flake_loc,
                    canoned_path.display()
                ),
            ));
        }
        if canoned_path.file_name() != Some(OsStr::new("flake.nix")) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Sym-link from {} must resolve to a `flake.nix`. Resolved to: {}",
                    flake_loc,
                    canoned_path.display()
                ),
            ));
        }

        let res = canoned_path.parent().ok_or(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Could not resolve to directory from {}",
                canoned_path.display()
            ),
        ))?;
        let res = Utf8PathBuf::from_path_buf(res.to_path_buf()).map_err(|_e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Invalid utf8: {}", res.display()),
            )
        })?;

        Ok(Self { canoned_dir: res })
    }
}
