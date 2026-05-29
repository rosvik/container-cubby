mod file;
mod path;
mod symlink;

pub mod blob;
pub mod manifest;
pub mod prune;
pub mod tag;
pub mod xattr;

use crate::digest::Digest;
use crate::utils;
use std::fs::File;
use std::io::{self, Read};

/// Deletes a manifest file and all tags that link to it.
pub fn delete_manifest(name: &str, reference: &str) -> Result<(), io::Error> {
  let reference_type = match utils::verify_reference(reference) {
    Ok(r) => r,
    Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid reference")),
  };

  let file_path = match reference_type {
    utils::Reference::Tag(_) => path::get(name, reference, path::FileType::Tag)?,
    utils::Reference::Digest(_) => path::get(name, reference, path::FileType::Manifest)?,
  };
  file::delete(&file_path)?;

  if let utils::Reference::Digest(_) = reference_type {
    // Delete tags that point to the deleted manifest
    let container_dir = path::container_dir(name)?;
    symlink::clean_broken_symlinks_in(&container_dir)?;
  }
  Ok(())
}

/// Opens a manifest file in read-only mode.
pub fn get_manifest(name: &str, reference: &str) -> Result<File, io::Error> {
  let reference_type = match utils::verify_reference(reference) {
    Ok(r) => r,
    Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid reference")),
  };

  let file_path = match reference_type {
    utils::Reference::Tag(_) => path::get(name, reference, path::FileType::Tag)?,
    utils::Reference::Digest(_) => path::get(name, reference, path::FileType::Manifest)?,
  };

  let file = file::try_read(&file_path)?;
  Ok(file)
}

/// Lists all the tags in a given namespace.
pub fn get_tags(name: &str) -> Result<Vec<String>, io::Error> {
  if !utils::is_safe_name(name) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe name: {name}")));
  }
  let container_dir = path::container_dir(name)?;
  let entries = file::read_dir(&container_dir)?;

  let mut tags = Vec::new();
  for entry in entries {
    let file_name = entry?.file_name().into_string().unwrap();
    if file_name.ends_with(".json") && !file_name.starts_with("sha256:") {
      tags.push(file_name.chars().take(file_name.len() - 5).collect::<String>());
    }
  }
  Ok(tags)
}

/// Opens a file in write-only mode.
pub fn create_hunk(name: &str, reference: &str) -> Result<File, io::Error> {
  ensure_container_dir_exists(name)?;

  let file_path = path::get(name, reference, path::FileType::Hunk)?;
  let file = file::try_create(&file_path)?;
  Ok(file)
}

/// Opens a file in append-only mode.
pub fn append_hunk(name: &str, reference: &str) -> Result<File, io::Error> {
  ensure_container_dir_exists(name)?;

  let file_path = path::get(name, reference, path::FileType::Hunk)?;
  let file = file::append(&file_path)?;
  Ok(file)
}

/// Opens a file in read-only mode.
pub fn read_hunk(name: &str, reference: &str) -> Result<File, io::Error> {
  let file_path = path::get(name, reference, path::FileType::Hunk)?;
  let file = file::try_read(&file_path)?;
  Ok(file)
}

/// Verifies that a hunk is complete, and converts it into a blob.
pub fn commit_hunk(name: &str, reference: &str, digest: &Digest) -> Result<(), io::Error> {
  ensure_container_dir_exists(name)?;
  ensure_blob_dir_exists(digest)?;

  let hunk_path = path::get(name, reference, path::FileType::Hunk)?;
  let blob_path = path::get(name, digest.to_string().as_str(), path::FileType::Blob)?;
  let symlink_path = path::get(name, digest.to_string().as_str(), path::FileType::BlobLink)?;

  let mut file = file::try_read(&hunk_path)?;
  let mut buf = Vec::new();
  file.read_to_end(&mut buf)?;

  let blob = match blob::Blob::new(name.to_string(), digest.to_string()) {
    Ok(blob) => blob,
    Err(e) => {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid blob: {e:?}")));
    }
  };
  if let Err(e) = blob.verify(&buf) {
    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Digest mismatch: {e:?}")));
  }

  file::rename(&hunk_path, &blob_path)?;

  match symlink::create_relative_symlink(&symlink_path, &blob_path) {
    Ok(_) => (),
    Err(e) => match e.kind() {
      // If the symlink already exists, we can ignore the error and continue.
      io::ErrorKind::AlreadyExists => (),
      _ => return Err(e),
    },
  }

  Ok(())
}

/// Ensures that the directory where a blob should be stored exists.
pub fn ensure_blob_dir_exists(digest: &Digest) -> Result<(), std::io::Error> {
  let blob_dir = path::blob_dir(digest);
  file::create_dir(&blob_dir)?;
  Ok(())
}

/// Ensures that the directory where a container should be stored exists.
pub fn ensure_container_dir_exists(name: &str) -> Result<(), std::io::Error> {
  let container_dir = path::container_dir(name)?;
  file::create_dir(&container_dir)?;
  Ok(())
}
