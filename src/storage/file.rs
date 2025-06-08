use crate::env;
use std::fs;
use std::io;

/// Opens a file with read permissions. If the file does not exist, an error is
/// returned.
pub fn try_read(relative_path: &str) -> Result<fs::File, io::Error> {
  let absolute_path = format!("{}/{relative_path}", env::data_dir());
  let file = fs::File::open(absolute_path)?;
  Ok(file)
}

/// Creates and opens a file with write permissions. If the file already exists,
/// an error is returned.
pub fn try_create(relative_path: &str) -> Result<fs::File, io::Error> {
  let absolute_path = format!("{}/{relative_path}", env::data_dir());
  let file = fs::OpenOptions::new().create_new(true).write(true).open(absolute_path)?;
  Ok(file)
}

/// Opens a file with append permissions.
pub fn append(relative_path: &str) -> Result<fs::File, io::Error> {
  let absolute_path = format!("{}/{relative_path}", env::data_dir());
  let file = fs::OpenOptions::new().append(true).create(true).open(absolute_path)?;
  Ok(file)
}

pub fn delete(relative_path: &str) -> Result<(), io::Error> {
  let absolute_path = format!("{}/{relative_path}", env::data_dir());
  fs::remove_file(absolute_path)?;
  Ok(())
}

pub fn rename(relative_path_from: &str, relative_path_to: &str) -> Result<(), io::Error> {
  let absolute_from = format!("{}/{relative_path_from}", env::data_dir());
  let absolute_to = format!("{}/{relative_path_to}", env::data_dir());
  fs::rename(absolute_from, absolute_to)?;
  Ok(())
}

pub fn create_dir(relative_path: &str) -> Result<(), io::Error> {
  let absolute_path = format!("{}/{relative_path}", env::data_dir());
  fs::DirBuilder::new().recursive(true).create(&absolute_path)?;
  Ok(())
}

pub fn read_dir(relative_path: &str) -> Result<fs::ReadDir, io::Error> {
  let absolute_path = format!("{}/{relative_path}", env::data_dir());
  fs::read_dir(absolute_path)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::storage::ensure_container_dir_exists;
  use crate::tests::utils::get_random_namespace;
  use std::io::{Read, Write};

  #[test]
  fn test_try_create() {
    let name: String = get_random_namespace();
    ensure_container_dir_exists(&name).unwrap();
    let container_dir = crate::storage::path::container_dir(&name).unwrap();
    let relative_file_path = format!("{container_dir}/test.txt");

    let file = try_create(&relative_file_path);
    assert!(file.is_ok());

    let mut file = file.unwrap();
    let result = file.write(b"Hello, world!");
    assert!(result.is_ok());
  }

  #[test]
  fn test_append() {
    let name: String = get_random_namespace();
    ensure_container_dir_exists(&name).unwrap();
    let container_dir = crate::storage::path::container_dir(&name).unwrap();
    let relative_file_path = format!("{container_dir}/test.txt");

    let mut file = append(&relative_file_path).unwrap();
    let _ = file.write(b"Hello");

    let mut file = append(&relative_file_path).unwrap();
    let _ = file.write(b" world!");

    let mut file = try_read(&relative_file_path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"Hello world!");
  }

  #[test]
  fn test_try_read() {
    let name: String = get_random_namespace();
    ensure_container_dir_exists(&name).unwrap();
    let container_dir = crate::storage::path::container_dir(&name).unwrap();
    let relative_file_path = format!("{container_dir}/test.txt");

    let mut file = try_create(&relative_file_path).unwrap();
    file.write_all(b"Hello, world!").unwrap();

    let file = try_read(&relative_file_path);
    assert!(file.is_ok());

    let mut buf = Vec::new();
    file.unwrap().read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"Hello, world!");
  }
}
