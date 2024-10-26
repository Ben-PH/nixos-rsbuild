use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Serialize, Serializer};
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{self, ErrorKind},
    path::Path,
    process::Command,
};

const GEN_DIR: &str = "/nix/var/nix/profiles";

#[derive(Debug, Serialize, Eq, PartialEq, Copy, Clone)]
pub struct GenNumber {
    #[serde(flatten)]
    pub num: u32,
}
#[derive(Debug, Serialize)]
pub struct NixosVersion(pub String);

impl PartialOrd for GenNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GenNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.num.cmp(&other.num)
    }
}

impl From<u32> for GenNumber {
    fn from(num: u32) -> Self {
        Self { num }
    }
}

impl TryFrom<&Path> for GenNumber {
    type Error = io::Error;

    /// e.g. /nix/var/nix/profiles/system-14-link -> 14
    fn try_from(gen_link: &Path) -> Result<Self, Self::Error> {
        let base = gen_link
            .file_name()
            .ok_or(io::Error::new(ErrorKind::NotFound, "invalid dir: `..`"))?
            .to_str()
            .ok_or(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Invalid Utf8 at {}", gen_link.display()),
            ))?;

        if !base.starts_with("system-") || !base.ends_with("-link") {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "file in {} must follow format 'system-X-link': {}",
                    gen_link.display(),
                    base
                ),
            ));
        }
        let base_2 = base.trim_end_matches("-link");
        let base_3 = base_2.trim_start_matches("system-");
        let res = base_3.parse::<u32>().map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidInput,
                format!("Failed conversion to u32: {}", e),
            )
        })?;
        Ok(Self { num: res })
    }
}

#[derive(Debug, Serialize)]
pub struct GenerationMeta {
    build_time: DateTime<Utc>,
    nixos_version: NixosVersion,
    kernel_version: Version,
    cfg_revision: Option<String>,
    specialisations: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct NumberedGenMeta {
    #[serde(flatten)]
    num: GenNumber,
    #[serde(flatten)]
    desc: GenerationMeta,
}
#[derive(Debug)]
pub struct GenDescTable {
    current: GenNumber,
    desc: BTreeMap<GenNumber, GenerationMeta>,
}
impl Serialize for GenDescTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser_desc = serde_json::to_value(&self.desc).unwrap();
        if let Some(entry) = ser_desc.get_mut(self.current.num.to_string()) {
            // Step 3: Add `"current": true` to the selected entry
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("current".to_string(), json!(true));
            }
        }
        ser_desc.serialize(serializer)
    }
}

impl From<(u32, GenerationMeta)> for NumberedGenMeta {
    fn from(value: (u32, GenerationMeta)) -> Self {
        Self {
            num: value.0.into(),
            desc: value.1,
        }
    }
}

/// Takes a file path to a generations dir. Typically `/nix/var/nix/profiles/system-x-link`, but
/// its canonicalised path can be used as well
impl TryFrom<&Path> for GenerationMeta {
    type Error = io::Error;

    fn try_from(gen_dir: &Path) -> Result<Self, Self::Error> {
        let _cannoned_dir = file_utils::CanonedStorePath::try_from(gen_dir)?;
        let gen_number = GenNumber::try_from(gen_dir)?;
        log::trace!("gen-number {}", gen_number.num);

        let build_time = file_utils::creation_time(gen_dir)?;
        log::trace!("creation-time {}", build_time);

        let nixos_ver = Self::nixos_version(gen_dir)?;
        log::trace!("nix-os version: {:?}", nixos_ver);

        let parsed_kern_ver = Self::kernel_version(gen_dir)?;
        log::trace!("kernel version: {}", parsed_kern_ver);

        let mut cfg_command = Command::new(gen_dir.join("sw/bin/nixos-version"));
        let cfg_command = cfg_command.arg("--configuration-revision");
        let cfg_cmd_res = cfg_command.output()?.stdout;
        let cfg_revision = if !cfg_cmd_res.is_empty() {
            Some(String::from_utf8(cfg_cmd_res).unwrap())
        } else {
            None
        };

        let specialisations = std::fs::read_dir(gen_dir.join("specialisation"))?
            // Filter out the error direntries
            .filter_map(Result::ok)
            // get the path
            .map(|e| e.path())
            // stringify
            .map(|d| format!("{}", d.display()))
            .collect();

        Ok(GenerationMeta {
            build_time,
            nixos_version: NixosVersion(nixos_ver),
            kernel_version: parsed_kern_ver,
            cfg_revision,
            specialisations,
        })
    }
}

impl GenerationMeta {
    /// An iterator over (number, generation-meta) pairs. Usually `.collect::<_>()`ed into an
    /// ordered key/value data struct such as a `BTreeMap`.
    pub fn run_cmd() -> io::Result<impl Iterator<Item = (GenNumber, Self)>> {
        let gen_dir_root = Path::new(GEN_DIR);

        // iterate over each entry in the directory...
        let res = std::fs::read_dir(gen_dir_root)?
            // for each path in the dir-entries iterator...
            .filter_map(|e| e.map(|e| e.path()).ok())
            // keep only the ones that can map to a (number, path) pair
            .filter_map(|e| GenNumber::try_from(e.as_path()).map(|num| (num, e)).ok())
            // keep only the (number, path) pairs that can map to (number, generations-meta) pair
            .filter_map(|(i, v)| GenerationMeta::try_from(v.as_path()).map(|v| (i, v)).ok());
        Ok(res)
    }

    fn nixos_version(gen_dir: &Path) -> io::Result<String> {
        let ver_dir = &gen_dir.join("nixos-version");
        log::trace!("ver-dir: {}", ver_dir.display());
        crate::utils::read_fst_line(ver_dir)
    }

    fn kernel_version(gen_dir: &Path) -> io::Result<Version> {
        // canonicalise
        let mut kern_dir = std::fs::canonicalize(gen_dir.join("kernel"))?;

        // only directories
        if !kern_dir.is_dir() {
            kern_dir = kern_dir.parent().unwrap().to_path_buf();
        }

        // `lib/modules/<kernel-version/`
        let kernel_ver_dir = std::fs::read_dir(kern_dir.join("lib/modules"))?
            .next()
            .ok_or(io::Error::new(
                ErrorKind::NotFound,
                "could not find semverdir",
            ))??;

        semver::Version::parse(&kernel_ver_dir.file_name().into_string().unwrap()).map_err(|e| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!("could not parse semver from dirname: {}", e),
            )
        })
    }
}

// General file utilities
mod file_utils {
    use std::{
        ffi::OsStr,
        io,
        path::{Path, PathBuf},
    };

    use chrono::{DateTime, Utc};

    enum StoreEntryType {
        Directory,
        ExtDrv,
        ExtMissing,
        // ExtOther(OsString),
        ExtOther(()),
    }
    /// <https://nix.dev/manual/nix/2.24/protocols/store-path#store-path-proper>
    /// `/nix/store/<digest>-<name>`
    pub(super) struct CanonedStorePath {
        /// the 32-char string is the base32 encoding of the first 20bytes. We store the decoded
        /// bytes instead of the string.
        /// TODO: actually decode back to the 20 bytes...
        digest: String,
        name: String,
        _entry_type: StoreEntryType,
    }

    impl TryFrom<&Path> for CanonedStorePath {
        type Error = io::Error;

        fn try_from(value: &Path) -> Result<Self, Self::Error> {
            let cannoned = std::fs::canonicalize(value)?;

            // Pull out the file/dirname, sanatising it's not `..`, and is a valid string
            let fname = {
                let fname = cannoned.file_name();
                fname
                    .ok_or(io::Error::new(
                        io::ErrorKind::Other,
                        "canonicalised to `..` for some reason",
                    ))?
                    .to_str()
                    .ok_or(io::Error::new(
                        io::ErrorKind::Other,
                        "canonicalised to `..` for some reason",
                    ))
            }?;

            // verify the format
            if fname.find('-') != Some(32) {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "expected <[char; 32]>-<name>. `-` not found at idx 32",
                ));
            }

            // Now we can destructure the file name into digest, name, and filetype

            let entry_type = if cannoned.is_dir() {
                StoreEntryType::Directory
            } else {
                match cannoned.extension().and_then(OsStr::to_str) {
                    Some("drv") => StoreEntryType::ExtDrv,
                    Some(_d) => StoreEntryType::ExtOther(()),
                    None => StoreEntryType::ExtMissing,
                }
            };

            let (digest, name) = fname.split_at(32);
            let name = name[1..].to_string();
            Ok(Self {
                digest: digest.to_string(),
                name,
                _entry_type: entry_type,
            })
        }
    }

    impl From<&CanonedStorePath> for PathBuf {
        fn from(value: &CanonedStorePath) -> Self {
            PathBuf::from(format!("/nix/store/{}-{}", value.digest, value.name))
        }
    }

    pub(super) fn creation_time(gen_dir: &Path) -> io::Result<DateTime<Utc>> {
        std::fs::metadata(gen_dir)?
            .created()
            .map(DateTime::<Utc>::from)
    }
}
