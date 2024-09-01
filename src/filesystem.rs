use crate::utils;
use std::fs::{DirBuilder, File};
use std::io;

const DATA_DIR: &str = "data";

fn prepare_container(name: &str) -> Result<String, io::Error> {
  if !utils::is_safe_name(name) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe name: {}", name)));
  }

  let container_directory = format!("{DATA_DIR}/containers/{name}");
  DirBuilder::new().recursive(true).create(&container_directory)?;

  Ok(container_directory)
}

fn prepare_blob(name: &str, digest: &str) -> Result<(String, String), io::Error> {
  if !utils::is_safe_digest(digest) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe digest: {}", digest)));
  }

  let digest = digest.replace("sha256:", "");
  let prefix = digest.chars().take(2).collect::<String>();
  let blob_directory = format!("{DATA_DIR}/blobs/{prefix}");
  DirBuilder::new().recursive(true).create(&blob_directory)?;

  let container_directory = prepare_container(name)?;

  Ok((container_directory, blob_directory))
}

fn prepare_manifest(name: &str, reference: &str) -> Result<(String, String), io::Error> {
  if !utils::is_safe_reference(reference) {
    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      format!("Unsafe reference: {}", reference),
    ));
  }

  let container_directory = prepare_container(name)?;

  let file_name = match reference.starts_with("sha256:") {
    true => {
      let digest = reference.replace("sha256:", "");
      format!("{}.json", digest)
    }
    false => {
      format!("{}.json", reference)
    }
  };

  Ok((container_directory, file_name))
}

pub fn create_blob_file(name: &str, digest: &str) -> Result<File, io::Error> {
  let (container_directory, blob_directory) = prepare_blob(name, digest)?;

  let original_path = format!(
    "{blob_directory}/{}.blob",
    digest.replace("sha256:", "").chars().skip(2).collect::<String>()
  );
  let file = File::create(&original_path)?;

  let symlink_path = format!("{container_directory}/{}.blob", digest.replace("sha256:", ""));

  let dir_levels = blob_directory.split("/").count() - 1;
  println!("{:?}", dir_levels);

  std::os::unix::fs::symlink(
    format!(
      "{}blobs/{}/{}.blob",
      "../".repeat(dir_levels),
      digest.replace("sha256:", "").chars().take(2).collect::<String>(),
      digest.replace("sha256:", "").chars().skip(2).collect::<String>()
    ),
    symlink_path,
  )?;

  Ok(file)
}

pub fn get_blob_file(name: &str, digest: &str) -> Result<File, io::Error> {
  let container_directory = prepare_container(name)?;

  println!("{container_directory}/{digest}.blob");
  let symlink =
    File::open(format!("{container_directory}/{}.blob", digest.replace("sha256:", "")))?;
  Ok(symlink)
}

pub fn create_manifest_file(name: &str, reference: &str) -> Result<File, io::Error> {
  let (directory, file_name) = prepare_manifest(name, reference)?;
  let file = File::create(format!("{directory}/{file_name}"))?;
  Ok(file)
}

pub fn get_manifest_file(name: &str, reference: &str) -> Result<File, io::Error> {
  let (directory, file_name) = prepare_manifest(name, reference)?;
  let file = File::open(format!("{directory}/{file_name}"))?;
  Ok(file)
}
