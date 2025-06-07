mod file;
mod path;
mod symlink;
pub mod xattr;

use crate::utils;
use std::fs::File;
use std::io::{self, Read};

/// Creates a blob file, and returns it in write-only mode.
pub fn create_blob(name: &str, digest: &str) -> Result<File, io::Error> {
  ensure_blob_dir_exists(digest)?;
  ensure_container_dir_exists(name)?;

  let blob_path = path::get(name, digest, path::FileType::Blob)?;
  let symlink_path = path::get(name, digest, path::FileType::BlobLink)?;

  // If the file already exists, create a symlink to it, and return an error.
  if file::try_read(&blob_path).is_ok() {
    symlink::create_relative_symlink(&symlink_path, &blob_path)?;
    return Err(io::Error::new(
      io::ErrorKind::AlreadyExists,
      format!("Blob already exists: {}", digest),
    ));
  }

  let file = file::try_create(&blob_path)?;
  symlink::create_relative_symlink(&symlink_path, &blob_path)?;
  Ok(file)
}

/// Mounts a blob file.
pub fn mount_blob(name: &str, digest: &str) -> Result<(), io::Error> {
  ensure_blob_dir_exists(digest)?;
  ensure_container_dir_exists(name)?;

  let blob_path = path::get(name, digest, path::FileType::Blob)?;
  let symlink_path = path::get(name, digest, path::FileType::BlobLink)?;

  // If the file does not exist, return an error.
  let file = file::try_read(&blob_path)?;
  drop(file);

  symlink::create_relative_symlink(&symlink_path, &blob_path)?;
  Ok(())
}

/// Deletes a blob file.
pub fn delete_blob(name: &str, digest: &str) -> Result<(), io::Error> {
  let symlink_path = path::get(name, digest, path::FileType::BlobLink)?;
  std::fs::remove_file(&symlink_path)?;
  Ok(())
}

/// Opens a blob file in read-only mode.
pub fn get_blob(name: &str, digest: &str) -> Result<File, io::Error> {
  let symlink_path = path::get(name, digest, path::FileType::BlobLink)?;
  let symlink = file::try_read(&symlink_path)?;
  Ok(symlink)
}

/// Creates a manifest file and optionally a tag symlink, and returns the
/// manifest file with write access. If the tag exists, it is overwritten.
pub fn create_manifest(name: &str, digest: &str, tag: Option<&str>) -> Result<File, io::Error> {
  ensure_container_dir_exists(name)?;

  let file_path = path::get(name, digest, path::FileType::Manifest)?;
  let tag_file_path = match tag {
    Some(tag) => Some(path::get(name, tag, path::FileType::Tag)?),
    None => None,
  };

  let file = match file::try_create(&file_path) {
    Ok(f) => Ok(f),
    Err(e) => match e.kind() {
      // If the file already exists, we should still create the tag before we
      // forward the error.
      io::ErrorKind::AlreadyExists => Err(e),
      // Otherwise error out.
      _ => return Err(e),
    },
  };

  if let Some(tag_file_path) = tag_file_path {
    symlink::create_relative_symlink(&tag_file_path, &file_path)?;
  }

  file
}

/// Deletes a manifest file and all tags that link to it.
pub fn delete_manifest(name: &str, reference: &str) -> Result<(), io::Error> {
  let reference_type = match utils::verify_reference(reference) {
    Ok(r) => r,
    Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid reference")),
  };

  let file_path = match reference_type {
    utils::Reference::Tag(_) => path::get(name, reference, path::FileType::Tag)?,
    utils::Reference::Sha256(_) => path::get(name, reference, path::FileType::Manifest)?,
  };
  file::delete(&file_path)?;

  if let utils::Reference::Sha256(_) = reference_type {
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
    utils::Reference::Sha256(_) => path::get(name, reference, path::FileType::Manifest)?,
  };

  let file = file::try_read(&file_path)?;
  Ok(file)
}

/// Lists all the tags in a given namespace.
pub fn get_tags(name: &str) -> Result<Vec<String>, io::Error> {
  if !utils::is_safe_name(name) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe name: {}", name)));
  }
  let container_dir = path::container_dir(name)?;
  let entries = std::fs::read_dir(container_dir)?;

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
pub fn commit_hunk(name: &str, reference: &str, digest: &str) -> Result<(), io::Error> {
  ensure_container_dir_exists(name)?;
  ensure_blob_dir_exists(digest)?;

  let hunk_path = path::get(name, reference, path::FileType::Hunk)?;
  let blob_path = path::get(name, digest, path::FileType::Blob)?;
  let symlink_path = path::get(name, digest, path::FileType::BlobLink)?;

  let mut file = file::try_read(&hunk_path)?;
  let mut buf = Vec::new();
  file.read_to_end(&mut buf)?;

  if utils::verify_blob(&buf, digest).is_err() {
    return Err(io::Error::new(
      io::ErrorKind::InvalidData,
      format!("Digest mismatch: {}", reference),
    ));
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
pub fn ensure_blob_dir_exists(digest: &str) -> Result<(), std::io::Error> {
  let blob_dir = path::blob_dir(digest)?;
  file::create_dir(&blob_dir)?;
  Ok(())
}

/// Ensures that the directory where a container should be stored exists.
pub fn ensure_container_dir_exists(name: &str) -> Result<(), std::io::Error> {
  let container_dir = path::container_dir(name)?;
  file::create_dir(&container_dir)?;
  Ok(())
}
