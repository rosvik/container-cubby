use crate::{
  digest::Digest,
  storage::{self, file, path, symlink},
  utils::{is_safe_name, DigestMismatch},
};
use std::{fs::File, io};

/// The binary form of content that is stored by a registry, addressable by a
/// digest
pub struct Blob {
  pub name: String,
  pub digest: Digest,
}

impl Blob {
  pub fn new(name: String, digest: String) -> Result<Self, io::Error> {
    let digest = Digest::from_string(digest.as_str())
      .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid digest: {e}")))?;
    if !is_safe_name(&name) {
      return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid name"));
    }
    Ok(Self { name, digest })
  }

  /// Creates an empty blob file and symlink in the data directory, and returns
  /// it with write access. If the blob already exists, a symlink to the
  /// existing blob is created and an `AlreadyExists` error is returned instead.
  pub fn create(&self) -> Result<File, io::Error> {
    storage::ensure_blob_dir_exists(&self.digest)?;
    storage::ensure_container_dir_exists(&self.name)?;

    let blob_path = path::get(&self.name, &self.digest.to_string(), path::FileType::Blob)?;
    let symlink_path = path::get(&self.name, &self.digest.to_string(), path::FileType::BlobLink)?;

    // If the file already exists, create a symlink to it and return an
    // `AlreadyExists` error.
    if file::try_read(&blob_path).is_ok() {
      symlink::create_relative_symlink(&symlink_path, &blob_path)?;
      return Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("Blob already exists: {}", self.digest),
      ));
    }

    let file = file::try_create(&blob_path)?;
    symlink::create_relative_symlink(&symlink_path, &blob_path)?;
    Ok(file)
  }

  /// Opens the file in read-only mode.
  pub fn read(&self) -> Result<File, io::Error> {
    let symlink_path = path::get(&self.name, &self.digest.to_string(), path::FileType::BlobLink)?;
    let symlink = file::try_read(&symlink_path)?;
    Ok(symlink)
  }

  /// Verifies that the blob data matches its digest.
  pub fn verify(&self, data: &[u8]) -> Result<(), DigestMismatch> {
    let computed_digest = Digest::new(self.digest.algorithm, &data.to_vec());
    if computed_digest != self.digest {
      return Err(DigestMismatch {
        expected: self.digest.clone(),
        computed: computed_digest,
      });
    }
    Ok(())
  }

  /// Creates a symlink from the container directory to the blob directory.
  ///
  /// It will ensure directories exist before creating the symlink.
  /// It will error if the blob does not exist.
  pub fn mount(&self) -> Result<(), io::Error> {
    storage::ensure_blob_dir_exists(&self.digest)?;
    storage::ensure_container_dir_exists(&self.name.to_string())?;

    let blob_path = path::get(&self.name, &self.digest.to_string(), path::FileType::Blob)?;
    let symlink_path = path::get(&self.name, &self.digest.to_string(), path::FileType::BlobLink)?;

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
    let symlink_path = path::get(&self.name, &self.digest.to_string(), path::FileType::BlobLink)?;
    file::delete(&symlink_path)?;
    Ok(())
  }
}
