use std::{
    fs::File,
    io::{self, BufRead},
    path::Path,
};

pub fn read_fst_line(file_path: &Path) -> io::Result<String> {
    let mut reader = io::BufReader::new(File::open(file_path)?);
    let mut line_buf = String::new();
    reader.read_line(&mut line_buf)?;
    Ok(line_buf)
}
