use chrono::{DateTime, Utc};
use semver::Version;
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

const GEN_DIR: &str = "/nix/var/nix/profiles";
#[derive(Debug, Serialize)]
pub struct GenDesc {
    build_time: DateTime<Utc>,
    nixos_ver: String,
    kernel_ver: Version,
    cfg_revision: Option<String>,
    specialisation: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct NumberedGenDesc {
    num: u32,
    #[serde(flatten)]
    desc: GenDesc,
}
#[derive(Debug)]
pub struct GenDescTable {
    current: u32,
    desc: BTreeMap<u32, GenDesc>,
}
impl Serialize for GenDescTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser_desc = serde_json::to_value(&self.desc).unwrap();
        if let Some(entry) = ser_desc.get_mut(self.current.to_string()) {
            // Step 3: Add `"current": true` to the selected entry
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("current".to_string(), json!(true));
            }
        }
        ser_desc.serialize(serializer)
    }
}

impl From<(u32, GenDesc)> for NumberedGenDesc {
    fn from(value: (u32, GenDesc)) -> Self {
        Self {
            num: value.0,
            desc: value.1,
        }
    }
}
// impl<I: Iterator<Item = &NumberedGenDesc>> From<(u32, I)> for GenDescTable {
//     fn from((current, desc_iter): (u32, I)) -> Self {
//         let desc = desc_iter.map(|d| (d.num, d.desc)).collect::<BTreeMap<_,_>>();
//         Self { current, desc }
//     }
// }

/// e.g. /nix/var/nix/profiles/system-14-link -> 14
fn generation_no_from_dir(gen_dir: &Path) -> Result<u32, String> {
    let Some(base) = gen_dir.file_stem().and_then(|s| s.to_str()) else {
        return Err(format!("no file in {}", gen_dir.display()));
    };
    if !base.starts_with("system-") || !base.ends_with("-link") {
        return Err(format!(
            "file in {} must follow format 'system-X-link': {}",
            gen_dir.display(),
            base
        ));
    }
    let base_2 = base.trim_end_matches("-link");
    let base_3 = base_2.trim_start_matches("system-");
    Ok(base_3.parse().unwrap_or(0))
}

fn read_fst_line(file_path: &Path) -> io::Result<String> {
    log::trace!("reading line from {}", file_path.display());
    let mut reader = io::BufReader::new(File::open(file_path)?);
    let mut line_buf = String::new();
    reader.read_line(&mut line_buf)?;
    Ok(line_buf)
}

/// Takes a file path to a generations dir. Typically `A/nix/var/nix/profiles/system-x-link`, but
/// its canonicalised path can be used as well
pub fn describe_generation(gen_dir: &Path) -> Result<GenDesc, String> {
    let gen_number = generation_no_from_dir(gen_dir)?;
    log::trace!("gen-number {}", gen_number);

    let build_time = std::fs::metadata(gen_dir)
        .map(|md| {
            md.created()
                .map_err(|_| "Platform not supported for getting creation time".to_string())
        })
        .map_err(|_| {
            format!(
                "Cannot get creation time metadata from {}",
                gen_dir.display()
            )
        })?
        .map(DateTime::<Utc>::from)?;
    let dir = &gen_dir.join("nixos-version");
    log::trace!("ver-dir: {}", dir.display());
    let nixos_ver = read_fst_line(dir).map_err(|_| "Could not read ver-dir".to_string())?;
    log::trace!("nix-os version: {:?}", nixos_ver);

    let mut kern_dir = std::fs::canonicalize(gen_dir.join("kernel")).map_err(|_| {
        format!(
            "Could not get canonicalised path to kernel dir from {}",
            gen_dir.display()
        )
    })?;

    if !kern_dir.is_dir() {
        kern_dir = kern_dir.parent().unwrap().to_path_buf();
    }

    let Some(Ok(kernel_ver_dir)) = std::fs::read_dir(kern_dir.join("lib/modules"))
        .map_err(|_| "Could not read ker-ver-dir".to_string())?
        .next()
    else {
        return Err("could not get kvar dir".to_string());
    };

    let kernel_ver_dir: semver::Version =
        semver::Version::parse(&kernel_ver_dir.file_name().into_string().unwrap())
            .map_err(|_| "Could not parse ver-dir to semver".to_string())?;
    log::trace!("kernel version: {}", kernel_ver_dir);

    let mut cfg_command = Command::new(gen_dir.join("sw/bin/nixos-version"));
    let cfg_command = cfg_command.arg("--configuration-revision");
    let cfg_cmd_res = crate::run_cmd(cfg_command)
        .map_err(|_| "Getting cfg revision failed".to_string())?
        .status
        .success();

    let spec_ls = std::fs::read_dir(gen_dir.join("specialisation")).unwrap();
    log::trace!("read dir of specs");
    for ent in spec_ls {
        log::info!("spec: {:?}", ent.unwrap().path().file_name());
    }

    Ok(GenDesc {
        build_time,
        nixos_ver,
        kernel_ver: kernel_ver_dir,
        cfg_revision: None,
        specialisation: None,
    })
}

pub fn list_generations() {
    let gen_dir_root = Path::new(GEN_DIR);
    let read = std::fs::read_dir(gen_dir_root).unwrap();
    let entry_iter = read
        .filter_map(|e| e.map(|e| e.path()).ok())
        .filter_map(|e| generation_no_from_dir(&e).map(|num| (num, e)).ok())
        .collect::<BTreeMap<u32, PathBuf>>();
    let descriptions = entry_iter
        .iter()
        .rev()
        .filter_map(|(i, v)| describe_generation(v).map(|v| (*i, v)).ok())
        .collect::<BTreeMap<_, _>>();
    let table = GenDescTable {
        current: 28,
        desc: descriptions,
    };
    println!("{}", serde_json::to_string_pretty(&table).unwrap());
    // {
    //
    //         match describe_generation(path) {
    //             Ok(d) =>
    //             Err(e) => log::error!("{}", e),
    //         }
    //     }
    log::trace!("DONE");
}
