use std::{ffi::OsString, path::Path};

/// Contains ordered collection of an attribute path.
///
/// e.g. when using `--flake /path/to/dir#fizz.buzz`, this will be [fizz, buzz] internally
#[derive(Debug, Clone)]
pub struct FlakeAttr {
    attr_path: Vec<String>,
}

impl FlakeAttr {
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

    pub fn route_to_toplevel(&mut self) {
        self.attr_path
            .extend_from_slice(&["config", "system", "build", "toplevel"].map(String::from));
    }
    pub fn len(&self) -> usize {
        self.attr_path.len()
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
