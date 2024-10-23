use std::{ffi::OsString, io, path::Path};

/// Contains ordered collection of an attribute path.
///
/// e.g. when using `--flake /path/to/dir#fizz.buzz`, this will be [fizz, buzz] internally
#[derive(Debug, Clone)]
pub struct FlakeAttr {
    attr_path: Vec<String>,
}

impl FlakeAttr {
    /// Prepends `nixosConfigurations` if not already. sets hostname to a `hostname` read, falling
    /// back to default.
    fn set_config(&mut self) {
        if self.attr_path.first().map(String::as_str) != Some("nixosConfigurations") {
            self.attr_path.insert(0, "nixosConfigurations".to_string());
        }
        if self.attr_path.len() > 1 {
            return;
        }

        let machine_name = crate::utils::read_fst_line(Path::new("/proc/sys/kernel/hostname"))
            .unwrap_or("default".to_string());
        self.attr_path.push(machine_name);
        log::trace!("Flake attr: {}", self);
    }

    /// appends the attribute path `.config.system.build.toplivel`, i.e. the path used when running
    /// the standard build: `nixosConfigurations.<hostname>.config....`
    pub fn route_to_toplevel(&mut self) {
        self.attr_path
            .extend_from_slice(&["config", "system", "build", "toplevel"].map(String::from));
    }
    pub fn len(&self) -> usize {
        self.attr_path.len()
    }
    pub fn try_default() -> io::Result<Self> {
        let attr = hostname::get()?.into_string().map_err(|_| {
            io::Error::new(
                io::ErrorKind::Other,
                "hostname read gave non-utf8 result".to_string(),
            )
        })?;
        Ok(Self {
            attr_path: vec!["nixosConfigurations".to_string(), attr],
        })
    }
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
