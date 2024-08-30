use crate::utils;
use std::fs::{DirBuilder, File};
use std::io;

const DATA_DIR: &str = "data";

fn prepare_blob(digest: &str) -> Result<(String, String), io::Error> {
  if !utils::is_safe_digest(digest) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe digest: {}", digest)));
  }

  let digest = digest.replace("sha256:", "");
  let prefix = digest.chars().take(2).collect::<String>();
  let directory = format!("{DATA_DIR}/blobs/{prefix}/");
  DirBuilder::new().recursive(true).create(&directory)?;

  let rest = digest.chars().skip(2).collect::<String>();
  let file_name = format!("{rest}.blob");

  Ok((directory, file_name))
}

fn prepare_manifest(name: &str, reference: &str) -> Result<(String, String), io::Error> {
  if !utils::is_safe_name(name) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe name: {}", name)));
  }
  if !utils::is_safe_reference(reference) {
    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      format!("Unsafe reference: {}", reference),
    ));
  }

  let directory = format!("{DATA_DIR}/manifests/{name}/");
  DirBuilder::new().recursive(true).create(&directory)?;

  let file_name = match reference.starts_with("sha256:") {
    true => {
      let digest = reference.replace("sha256:", "");
      format!("{}.json", digest)
    }
    false => {
      format!("{}.json", reference)
    }
  };

  Ok((directory, file_name))
}

pub fn create_blob_file(digest: &str) -> Result<File, io::Error> {
  let (directory, file_name) = prepare_blob(digest)?;
  let file = File::create(format!("{directory}/{file_name}"))?;
  Ok(file)
}

pub fn get_blob_file(digest: &str) -> Result<File, io::Error> {
  let (directory, file_name) = prepare_blob(digest)?;
  let file = File::open(format!("{directory}/{file_name}"))?;
  Ok(file)
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
