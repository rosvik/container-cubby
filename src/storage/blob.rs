use crate::{
  storage::{self, file, path, symlink},
  utils,
};
use std::io;

/// The binary form of content that is stored by a registry, addressable by a
/// digest
pub struct Blob {
  pub name: String,
  pub digest: String,
}

impl Blob {
  pub fn new(name: String, digest: String) -> Result<Self, io::Error> {
    if !utils::is_safe_digest(&digest) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid digest"));
    }
    if !utils::is_safe_name(&name) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid name"));
    }
    Ok(Self { name, digest })
  }

  /// Creates a symlink from the container directory to the blob directory.
  ///
  /// It will ensure directories exist before creating the symlink.
  /// It will error if the blob does not exist.
  pub fn mount(&self) -> Result<(), io::Error> {
    storage::ensure_blob_dir_exists(&self.digest)?;
    storage::ensure_container_dir_exists(&self.name)?;

    let blob_path = path::get(&self.name, &self.digest, path::FileType::Blob)?;
    let symlink_path = path::get(&self.name, &self.digest, path::FileType::BlobLink)?;

    // If the file does not exist, return an error.
    let file = file::try_read(&blob_path)?;
    drop(file);

    symlink::create_relative_symlink(&symlink_path, &blob_path)?;
    Ok(())
  }

  /// Deletes the symlink from the container directory to the blob directory.
  ///
  /// It will error if the symlink does not exist.
  pub fn unmount(&self) -> Result<(), io::Error> {
    let symlink_path = path::get(&self.name, &self.digest, path::FileType::BlobLink)?;
    file::delete(&symlink_path)?;
    Ok(())
  }
}
