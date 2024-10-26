use camino::{Utf8Path, Utf8PathBuf};

use crate::flake::FlakeRefInput;

#[allow(unused)]
#[allow(clippy::unnecessary_wraps, reason = "result needed for parser")]
pub(super) fn profile_name_parse(prof_name: &str) -> Result<Utf8PathBuf, String> {
    Ok(Utf8Path::new("/nix/var/nix/profiles/system-profiles").join(prof_name))
}

// TODO: this is needed for bringing in a value parser. if you can access `try_from` directly, do
// that instead
pub(super) fn flake_parse(val: &str) -> Result<FlakeRefInput, String> {
    FlakeRefInput::try_from(val)
}
