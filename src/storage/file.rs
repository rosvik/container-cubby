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

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Read, Write};
  use tempfile::{tempdir, NamedTempFile};

  #[test]
  fn test_try_create() {
    let tmp_dir = tempdir().unwrap();
    let tmp_dir_path = String::from(tmp_dir.path().to_str().unwrap());
    let file_path = format!("{}/test.txt", tmp_dir_path);

    let file = try_create(&file_path);
    assert!(file.is_ok());

    let mut file = file.unwrap();
    let result = file.write(b"Hello, world!");
    assert!(result.is_ok());
  }

  #[test]
  fn test_try_append() {
    let tmp_file = NamedTempFile::new().unwrap();
    let tmp_file_path = String::from(tmp_file.path().to_str().unwrap());

    let mut file = try_append(&tmp_file_path).unwrap();
    let result = file.write(b"Hello, world!");
    assert!(result.is_ok());
  }

  #[test]
  fn test_try_read() {
    let tmp_dir = tempdir().unwrap();
    let tmp_dir_path = String::from(tmp_dir.path().to_str().unwrap());
    let file_path = format!("{}/test.txt", tmp_dir_path);
    let mut file = try_create(&file_path).unwrap();
    file.write_all(b"Hello, world!").unwrap();

    let file = try_read(&file_path);
    assert!(file.is_ok());

    let mut buf = Vec::new();
    file.unwrap().read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"Hello, world!");
  }
}
