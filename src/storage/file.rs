use std::fs::{File, OpenOptions};
use std::io;

/// Opens a file with read permissions. If the file does not exist, an error is
/// returned.
pub fn try_read(path: &str) -> Result<File, io::Error> {
  let file = File::open(path)?;
  Ok(file)
}

/// Creates and opens a file with write permissions. If the file already exists,
/// an error is returned.
pub fn try_create(path: &str) -> Result<File, io::Error> {
  let file = OpenOptions::new().create_new(true).write(true).open(path)?;
  Ok(file)
}

/// Opens a file with append permissions. If the file does not exist, an error
/// is returned.
pub fn try_append(path: &str) -> Result<File, io::Error> {
  let file = OpenOptions::new().append(true).open(path)?;
  Ok(file)
}
