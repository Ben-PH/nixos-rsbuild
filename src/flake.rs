use crate::{cmd::SubCommand, run_cmd};
use camino::Utf8PathBuf;
use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    process::Command as CliCommand,
};
const NIX_ARGS: [&str; 2] = ["--extra-experimental-features", "'nix-command flakes'"];
const DEFAULT_FLAKE_NIX: &str = "/etc/nixos/flake.nix";
/// Destructured `<flake_dir>[#attribute]`
#[derive(Debug, Clone)]
pub struct FlakeRef {
    /// Pre-`#` component.
    /// Path to the dir where a flake.nix will be searched
    pub source: PathBuf,
    /// Post-`#` component
    pub output_selector: Option<FlakeAttr>,
}

impl FlakeRef {
    /// Checks if /etc/nixos has a flake.nix, or the canonicalised if need be
    pub fn canonned_default_dir() -> Option<PathBuf> {
        // Default path

        // Resolve sym-links
        let can_path = std::fs::canonicalize(DEFAULT_FLAKE_NIX).ok()?;

        // ...and pull out the directory
        if can_path.is_file() {
            return can_path.parent().map(Path::to_path_buf);
        }
        Some(can_path)
    }
    pub fn _canonicalise_path(&mut self) {
        log::trace!("cononicalising path: {}", self.source.display());
        log::warn!(
            "Attempting to canonicalise {}: this is untested",
            self.source.display()
        );
        if let Ok(path) = std::fs::canonicalize(&self.source) {
            if path.is_file() {
                log::error!("Internal error: canonicalised a source to a file");
            }
            if path.is_dir() && !path.eq(&self.source) {
                log::trace!("Updateing flake source: {}", path.display());
                self.source = path;
            }
        }
    }
}

/// Takes a string and maps it to a flake url
impl TryFrom<&str> for FlakeRef {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // no '#'? we just have the source, no selected attr
        let Some(fst_hash) = value.find('#') else {
            return Ok(FlakeRef {
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
        Ok(FlakeRef {
            source: PathBuf::from(path),
            output_selector: Some(attr),
        })
    }
}

/// Contains ordered collection of an attribute path.
///
/// e.g. when using `--flake /path/to/dir#fizz.buzz`, this will be [fizz, buzz] internally
#[derive(Debug, Clone)]
pub struct FlakeAttr {
    attr_path: Vec<String>,
}

/// "" -> Error
/// "contains"double.quote" -> Error
/// "contains#hash" -> Error
/// "foo" -> [foo]
/// "foo.bar" -> [foo, bar]
impl TryFrom<String> for FlakeAttr {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.contains('#') || value.contains('"') || value.is_empty() {
            log::trace!("malformed attr: {}", value);
            return Err(value);
        }
        Ok(FlakeAttr {
            attr_path: value.split('.').map(ToString::to_string).collect(),
        })
    }
}

/// `["flake", "attribute", "path"]` -> "flake.attribute.path"
impl std::fmt::Display for FlakeAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.attr_path.join("."))
    }
}

impl Default for FlakeAttr {
    fn default() -> Self {
        let Ok(attr) = hostname::get()
            .unwrap_or(OsString::from("default"))
            .into_string()
        else {
            eprintln!("Hostname fetch returned invalid unicode");
            std::process::exit(1);
        };
        Self {
            attr_path: vec!["nixosConfigurations".to_string(), attr],
        }
    }
}

pub fn flake_build_config(sub_cmd: &SubCommand, args: &[&str]) -> io::Result<Utf8PathBuf> {
    println!("Building in flake mode.");
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
        let mut cmd = CliCommand::new("nix");
        cmd.args(["--extra-experimental-features", "'nix-command flakes'"])
            .arg("build")
            .args(args);
        let _ = run_cmd(&mut cmd);
        // let _sym_link_to_result = std::fs::canonicalize(todo!("tmpDir joined with /result"))?;
    } else if !sub_cmd.inner_args().is_some_and(|a| a.build_host) {
        let mut cmd = CliCommand::new("nix");
        cmd.args(NIX_ARGS).arg("build").args(args).arg("--out-link");
        // .arg(todo!("tmpDir joined with /result"))
        let _ = run_cmd(&mut cmd);
        // let _sym_link_to_result = std::fs::canonicalize(todo!("tmpDir joined with /result"))?;
    } else {
        let (attr, _args) = (args[0], &args[1..]);
        // TODO: bring in FlakeArgs pass-through
        let mut cmd = CliCommand::new("nix");
        cmd.args(NIX_ARGS)
            .args(["eval", "--raw", &format!("{}.drv", attr)]);
        let drv = run_cmd(&mut cmd)?.stdout;
        if !String::from_utf8(drv).is_ok_and(|s| Path::new(&s).exists()) {
            eprintln!("nix eval failed");
            std::process::exit(1);
        }
    }
    todo!()
}
