use std::{ffi::OsString, io};

/// Contains ordered collection of an attribute path.
///
/// e.g. when using `--flake /path/to/dir#fizz.buzz`, this will be [fizz, buzz] internally
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct FlakeAttr {
    pub attr_path: Vec<String>,
}

impl FlakeAttr {
    /// Prepends `nixosConfigurations` if not already. sets hostname to a `hostname` read, falling
    /// back to default.
    /// ```
    /// use nixos_rsbuild::flake::FlakeAttr;
    /// let mut attr = FlakeAttr{attr_path: vec!["foo".into()]};
    /// assert_eq!("foo", attr.to_string());
    /// attr.set_config().unwrap();
    /// assert_eq!("nixosConfigurations.foo", attr.to_string());
    /// ```
    pub fn set_config(&mut self) -> io::Result<()> {
        log::info!("trying to prepend nixosConfigurations");
        if self.attr_path.first().map(String::as_str) != Some("nixosConfigurations") {
            self.attr_path.insert(0, "nixosConfigurations".to_string());
        }
        if self.attr_path.len() > 1 {
            return Ok(());
        }

        let machine_name: String = hostname::get()
            .unwrap_or(OsString::from("default"))
            .into_string()
            .map_err(|_os| {
                io::Error::new(io::ErrorKind::Other, "Could not read utf8-valid hostname")
            })?;
        log::info!("pushing {} attr", machine_name);
        self.attr_path.push(machine_name);
        log::trace!("Flake attr: {}", self);
        Ok(())
    }

    /// appends the attribute path `.config.system.build.toplivel`, i.e. the path used when running
    /// the standard build: `nixosConfigurations.<hostname>.config....`
    /// ```
    /// use nixos_rsbuild::flake::FlakeAttr;
    /// let mut attr = FlakeAttr{attr_path: vec!["foo".into()]};
    /// assert_eq!("foo", attr.to_string());
    /// attr.route_to_toplevel();
    /// assert_eq!("foo.config.system.build.toplevel", attr.to_string());
    /// ```
    pub fn route_to_toplevel(&mut self) {
        self.attr_path
            .extend_from_slice(&["config", "system", "build", "toplevel"].map(String::from));
    }
    pub fn len(&self) -> usize {
        self.attr_path.len()
    }
    pub fn is_empty(&self) -> bool {
        self.attr_path.is_empty()
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

    /// ```
    /// use nixos_rsbuild::flake::FlakeAttr;
    /// let mut attr = FlakeAttr::try_from("foo.bar".to_string()).unwrap();
    /// assert_eq!("foo.bar", attr.to_string());
    /// let mut attr = FlakeAttr::try_from("#foo.bar".to_string()).unwrap_err();
    /// assert_eq!("#foo.bar", attr);
    /// ```
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn set_config() {
        let hostname = hostname::get()
            .unwrap_or(OsString::from("default"))
            .into_string()
            .unwrap();
        let mut attr = FlakeAttr::try_from(hostname.clone()).unwrap();
        assert_eq!(hostname, attr.to_string());
        attr.set_config().unwrap();
        assert_eq!(
            format!("nixosConfigurations.{}", hostname),
            attr.to_string()
        );
        attr.route_to_toplevel();
        assert_eq!(
            format!(
                "nixosConfigurations.{}.config.system.build.toplevel",
                hostname
            ),
            attr.to_string()
        );
    }
    #[test]
    fn try_from() {
        assert!(FlakeAttr::try_from("fizz.bu\"".to_string()).is_err());
        assert!(FlakeAttr::try_from("fizz.bu#".to_string()).is_err());
        assert_eq!(
            FlakeAttr::try_from("fizz.bu".to_string()).unwrap(),
            FlakeAttr {
                attr_path: vec!["fizz".to_string(), "bu".to_string()]
            }
        );
        assert!(FlakeAttr::try_from("".to_string()).is_err());
    }

    // TODO: test try_default, when you can/cannot get hostname
}
