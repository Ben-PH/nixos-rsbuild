use std::{ffi::OsStr, io, path::{Path, PathBuf}};


const DEFAULT_FILE_DIR: &str = "/etc/nixos";
const DEFAULT_FLAKE_NIX: &str = "/etc/nixos/flake.nix";
#[derive(Debug, Clone)]
pub struct FlakeDir<T: AsRef<Path>> {
    canoned_dir: T,
}
impl<T: AsRef<Path>> AsRef<Path> for FlakeDir<T> {
    fn as_ref(&self) -> &Path {
        self.canoned_dir.as_ref()
    }
}

impl FlakeDir<PathBuf> {

    /// we don't use std-lib here because `impl<T: AsRef<Path>> TryFrom<T> for ...` is not doable
    /// without some esoteric magic.
    /// 
    /// Resolves symbolic links in flakeref path:
    /// path is not a dir: None
    /// path does not contain flake.nix: None
    /// contained flake.nix is a regular file: returned path matches current
    /// cantained flake.nix sym-links to anything other than regular file named `flake.nix`: None
    /// contained flake.nix links to regular file named `flake.nix`: said files parent directory
    pub fn try_from_path<T: AsRef<Path>>(value: T) -> Result<Self, io::Error> {
        if !value.as_ref().is_dir() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Is not a dir: {}", value.as_ref().display())));
        }

        let flake_loc = value.as_ref().join("flake.nix");
        let flake_exists = match std::fs::exists(&flake_loc) {
            Ok(exists) => exists,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!(
                    "Error when checking for existence of flake at {}: {}",
                    flake_loc.display(),
                    e
                )));
            }
        };

        if !flake_exists {
            return Err(io::Error::new(io::ErrorKind::Other, "flake-path must be a directory containing `flake.nix`."));
        }

        let canoned_path = match std::fs::canonicalize(&flake_loc) {
            Ok(path) => path,
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, format!("Could not canonicalise path {}: {}", flake_loc.display(), e)));
            }
        };
        if canoned_path.is_dir() {
            return Err(io::Error::new(io::ErrorKind::Other, format!(
                "Sym-link from {} must resolve to `flake.nix`. Resolved to a directory: {}",
                flake_loc.display(),
                canoned_path.display()
            )));
        }
        if canoned_path.file_name() != Some(OsStr::new("flake.nix")) {
            return Err(io::Error::new(io::ErrorKind::Other, format!(
                "Sym-link from {} must resolve to a `flake.nix`. Resolved to: {}",
                flake_loc.display(),
                canoned_path.display()
            )));
        }

        let Some(res) = canoned_path.parent() else {
            return Err(io::Error::new(io::ErrorKind::Other, format!(
                "Could not resolve directory from {}",
                canoned_path.display()
            )));
        };

        Ok(Self{canoned_dir: res.to_path_buf()})

    }
}





