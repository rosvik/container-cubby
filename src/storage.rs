use crate::utils;
use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{self, Read};

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
      let digest = reference.replace("sha256:", "sha256@");
      format!("{}.json", digest)
    }
    false => {
      format!("{}.json", reference)
    }
  };

  Ok((container_directory, file_name))
}

fn prepare_hunk(name: &str, reference: &str) -> Result<String, io::Error> {
  if !utils::is_safe_reference(reference) {
    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      format!("Unsafe reference: {}", reference),
    ));
  }

  let container_directory = prepare_container(name)?;

  let file_name = format!("{reference}.hunk");
  let file_path = format!("{container_directory}/{file_name}");

  Ok(file_path)
}

pub fn create_blob_file(name: &str, digest: &str) -> Result<File, io::Error> {
  let (container_directory, blob_directory) = prepare_blob(name, digest)?;

  let file_path = format!(
    "{blob_directory}/{}.blob",
    digest.replace("sha256:", "").chars().skip(2).collect::<String>()
  );
  let file = File::create(&file_path)?;

  let symlink_path = format!("{container_directory}/{}.blob", digest.replace("sha256:", ""));
  utils::create_relative_symlink(&symlink_path, &file_path)?;

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

pub fn get_tags(name: &str) -> Result<Vec<String>, io::Error> {
  if !utils::is_safe_name(name) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe name: {}", name)));
  }
  let container_directory = format!("{DATA_DIR}/containers/{name}");
  let entries = std::fs::read_dir(container_directory)?;

  let mut tags = Vec::new();
  for entry in entries {
    let file_name = entry?.file_name().into_string().unwrap();
    if file_name.ends_with(".json") && !file_name.starts_with("sha256@") {
      tags.push(file_name.chars().take(file_name.len() - 5).collect::<String>());
    }
  }
  Ok(tags)
}

/// Opens a file in write-only mode. This function will create a file if it
/// does not exist, and will truncate it if it does.
pub fn create_hunk_file(name: &str, reference: &str) -> Result<File, io::Error> {
  println!("get_hunk_file, name: {}, reference: {}", name, reference);
  let file_path = prepare_hunk(name, reference)?;
  let file = File::create(file_path)?;
  Ok(file)
}

pub fn append_hunk_file(name: &str, reference: &str) -> Result<File, io::Error> {
  let file_path = prepare_hunk(name, reference)?;
  let file = OpenOptions::new().append(true).open(file_path)?;
  Ok(file)
}

/// Opens a file in read-only mode.
pub fn open_hunk_file(name: &str, reference: &str) -> Result<File, io::Error> {
  let file_path = prepare_hunk(name, reference)?;

  let file = File::open(file_path)?;
  Ok(file)
}

pub fn commit_hunk(name: &str, reference: &str, digest: &str) -> Result<(), io::Error> {
  let hunk_path = prepare_hunk(name, reference)?;

  let mut file = File::open(&hunk_path)?;
  let mut buf = Vec::new();
  file.read_to_end(&mut buf)?;

  if utils::verify_blob(&buf, digest).is_err() {
    return Err(io::Error::new(
      io::ErrorKind::InvalidData,
      format!("Digest mismatch: {}", reference),
    ));
  }

  let (container_directory, blob_directory) = prepare_blob(name, digest)?;

  let blob_path = format!(
    "{blob_directory}/{}.blob",
    digest.replace("sha256:", "").chars().skip(2).collect::<String>()
  );
  std::fs::rename(&hunk_path, &blob_path)?;

  let symlink_path = format!("{container_directory}/{}.blob", digest.replace("sha256:", ""));
  utils::create_relative_symlink(&symlink_path, &blob_path)?;

  Ok(())
}
