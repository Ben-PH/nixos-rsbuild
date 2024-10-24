use std::{
    fs::File,
    io::{self, BufRead},
    path::Path,
};

pub const DEFAULT_FILE_DIR: &str = "/etc/nixos";
pub const DEFAULT_FLAKE_NIX: &str = "/etc/nixos/flake.nix";

/// Reads the first line of a file. Useful for files such as `/proc/sys/kernel/hostname`
///
/// # Errors
///
/// This function will return a file-open or `read_line` error
pub fn read_fst_line(file_path: &Path) -> io::Result<String> {
    let mut reader = io::BufReader::new(File::open(file_path)?);
    let mut line_buf = String::new();
    reader.read_line(&mut line_buf)?;
    Ok(line_buf)
}
