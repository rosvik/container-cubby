use crate::env;
use crate::utils;
use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{self, Read};
use xattr::FileExt;

/// Creates a blob file, and retuns it in write-only mode. If the file already
/// exists, an error is returned.
pub fn create_blob(name: &str, digest: &str) -> Result<File, io::Error> {
  let blob_path = get_path(name, digest, FileType::Blob)?;
  let symlink_path = get_path(name, digest, FileType::BlobLink)?;

  // If the file already exists, create a symlink to it, and return an
  // AlreadyExists error.
  if File::open(&blob_path).is_ok() {
    utils::create_relative_symlink(&symlink_path, &blob_path)?;
    return Err(io::Error::new(
      io::ErrorKind::AlreadyExists,
      format!("Blob already exists: {}", digest),
    ));
  }

  let file = OpenOptions::new().create_new(true).write(true).open(&blob_path)?;
  utils::create_relative_symlink(&symlink_path, &blob_path)?;
  Ok(file)
}

/// Mounts a blob file. If the file does not exist, an error is returned.
pub fn mount_blob(name: &str, digest: &str) -> Result<(), io::Error> {
  let blob_path = get_path(name, digest, FileType::Blob)?;
  let symlink_path = get_path(name, digest, FileType::BlobLink)?;

  // If the file does not exist, return an error.
  let file = File::open(&blob_path)?;
  drop(file);

  utils::create_relative_symlink(&symlink_path, &blob_path)?;
  Ok(())
}

/// Deletes a blob file. If the file does not exist, an error is returned.
pub fn delete_blob(name: &str, digest: &str) -> Result<(), io::Error> {
  let symlink_path = get_path(name, digest, FileType::BlobLink)?;
  std::fs::remove_file(&symlink_path)?;
  Ok(())
}

/// Opens a blob file in read-only mode. If the file does not exist, an error is
/// returned.
pub fn get_blob(name: &str, digest: &str) -> Result<File, io::Error> {
  let symlink_path = get_path(name, digest, FileType::BlobLink)?;
  let symlink = File::open(symlink_path)?;
  Ok(symlink)
}

/// Creates a manifest file, and retuns it in write-only mode. If the file
/// already exists, an error is returned.
pub fn create_manifest(name: &str, digest: &str, tag: Option<&str>) -> Result<File, io::Error> {
  let file_path = get_path(name, digest, FileType::Manifest)?;
  let tag_file_path = match tag {
    Some(tag) => Some(get_path(name, tag, FileType::Tag)?),
    None => None,
  };

  let file = match OpenOptions::new().create_new(true).write(true).open(&file_path) {
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
    utils::create_relative_symlink(&tag_file_path, &file_path)?;
  }

  file
}

/// Deletes a manifest file and all tags that link to the manifest. If the file
/// does not exist, an error is returned.
pub fn delete_manifest(name: &str, reference: &str) -> Result<(), io::Error> {
  let reference_type = match utils::verify_reference(reference) {
    Ok(r) => r,
    Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid reference")),
  };

  let file_path = match reference_type {
    utils::Reference::Tag(_) => get_path(name, reference, FileType::Tag)?,
    utils::Reference::Sha256(_) => get_path(name, reference, FileType::Manifest)?,
  };
  std::fs::remove_file(file_path)?;

  match reference_type {
    utils::Reference::Tag(_) => {}
    utils::Reference::Sha256(_) => {
      // Delete tags that point to the deleted manifest
      let container_dir = container_dir(name)?;
      utils::clean_broken_symlinks_in(&container_dir)?;
    }
  }

  Ok(())
}

/// Opens a manifest file in read-only mode. If the file does not exist, an error
/// is returned.
pub fn get_manifest(name: &str, reference: &str) -> Result<File, io::Error> {
  let reference_type = match utils::verify_reference(reference) {
    Ok(r) => r,
    Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid reference")),
  };

  let file_path = match reference_type {
    utils::Reference::Tag(_) => get_path(name, reference, FileType::Tag)?,
    utils::Reference::Sha256(_) => get_path(name, reference, FileType::Manifest)?,
  };

  let file = File::open(file_path)?;
  Ok(file)
}

/// Lists all the tags in a given namespace.
pub fn get_tags(name: &str) -> Result<Vec<String>, io::Error> {
  is_safe_name(name)?;
  let container_dir = container_dir(name)?;
  let entries = std::fs::read_dir(container_dir)?;

  let mut tags = Vec::new();
  for entry in entries {
    let file_name = entry?.file_name().into_string().unwrap();
    if file_name.ends_with(".json") && !file_name.starts_with("sha256@") {
      tags.push(file_name.chars().take(file_name.len() - 5).collect::<String>());
    }
  }
  Ok(tags)
}

/// Opens a file in write-only mode. If the file already exist, an error is
/// returned.
pub fn create_hunk(name: &str, reference: &str) -> Result<File, io::Error> {
  let file_path = get_path(name, reference, FileType::Hunk)?;
  let file = OpenOptions::new().create_new(true).write(true).open(file_path)?;
  Ok(file)
}

/// Opens a file in append-only mode. If the file does not exist, an error is
/// returned.
pub fn append_hunk(name: &str, reference: &str) -> Result<File, io::Error> {
  let file_path = get_path(name, reference, FileType::Hunk)?;
  let file = OpenOptions::new().append(true).open(file_path)?;
  Ok(file)
}

/// Opens a file in read-only mode. If the file does not exist, an error is
/// returned.
pub fn read_hunk(name: &str, reference: &str) -> Result<File, io::Error> {
  let file_path = get_path(name, reference, FileType::Hunk)?;
  let file = File::open(file_path)?;
  Ok(file)
}

/// Verifies that a hunk is complete, and converts it into a blob.
pub fn commit_hunk(name: &str, reference: &str, digest: &str) -> Result<(), io::Error> {
  let hunk_path = get_path(name, reference, FileType::Hunk)?;
  let blob_path = get_path(name, digest, FileType::Blob)?;
  let symlink_path = get_path(name, digest, FileType::BlobLink)?;

  let mut file = File::open(&hunk_path)?;
  let mut buf = Vec::new();
  file.read_to_end(&mut buf)?;

  if utils::verify_blob(&buf, digest).is_err() {
    return Err(io::Error::new(
      io::ErrorKind::InvalidData,
      format!("Digest mismatch: {}", reference),
    ));
  }

  std::fs::rename(&hunk_path, &blob_path)?;

  match utils::create_relative_symlink(&symlink_path, &blob_path) {
    Ok(_) => (),
    Err(e) => match e.kind() {
      // If the symlink already exists, we can ignore the error and continue.
      io::ErrorKind::AlreadyExists => (),
      _ => return Err(e),
    },
  }

  Ok(())
}

/// Gets the media type of a file by reading the `mediatype` extended attribute.
pub fn get_xattr_media_type(file: &File) -> Option<String> {
  let bytes = match file.get_xattr("user.mime_type") {
    Ok(bytes) => match bytes {
      Some(bytes) => bytes,
      None => return None,
    },
    Err(e) => {
      println!("Failed to get media type: {:?}", e);
      return None;
    }
  };
  String::from_utf8(bytes).ok()
}

/// Sets the media type of a file by setting the `mediatype` extended attribute.
pub fn set_xattr_media_type(file: &File, media_type: &str) -> Result<(), io::Error> {
  file.set_xattr("user.mime_type", media_type.as_bytes())
}

enum FileType {
  Blob,     // .blob
  BlobLink, // Symlink to Blob
  Hunk,     // .hunk
  Manifest, // .json
  Tag,      // Symlink to Manifest
}
fn get_path(name: &str, reference: &str, file_type: FileType) -> Result<String, io::Error> {
  match file_type {
    FileType::Blob => is_safe_digest(reference)?,
    FileType::BlobLink => is_safe_digest(reference)?,
    FileType::Hunk => is_safe_reference(reference)?,
    FileType::Manifest => is_safe_digest(reference)?,
    FileType::Tag => is_safe_reference(reference)?,
  }

  let reference = match reference.starts_with("sha256:") {
    true => reference.to_string().replace("sha256:", "sha256@"),
    false => reference.to_string(),
  };

  match file_type {
    FileType::Blob => {
      let file_name = reference.replace("sha256@", "").chars().skip(2).collect::<String>();
      Ok(format!("{}/{file_name}.blob", blob_dir(reference)?))
    }
    FileType::BlobLink => Ok(format!("{}/{reference}.blob", container_dir(name)?)),
    FileType::Hunk => Ok(format!("{}/{reference}.hunk", container_dir(name)?)),
    FileType::Manifest => Ok(format!("{}/{reference}.json", container_dir(name)?)),
    FileType::Tag => Ok(format!("{}/{reference}.json", container_dir(name)?)),
  }
}

fn container_dir(name: &str) -> Result<String, io::Error> {
  let data_dir = env::data_dir();
  is_safe_name(name)?;
  let container_dir = format!("{data_dir}/containers/{name}");
  DirBuilder::new().recursive(true).create(&container_dir)?;
  Ok(container_dir)
}
fn blob_dir(digest: String) -> Result<String, io::Error> {
  let data_dir = env::data_dir();
  let prefix = digest.replace("sha256:", "").chars().take(2).collect::<String>();
  let blob_dir = format!("{data_dir}/blobs/{prefix}");
  DirBuilder::new().recursive(true).create(&blob_dir)?;
  Ok(blob_dir)
}

fn is_safe_name(name: &str) -> Result<(), io::Error> {
  match utils::is_safe_name(name) {
    true => Ok(()),
    false => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe name: {}", name))),
  }
}
fn is_safe_digest(digest: &str) -> Result<(), io::Error> {
  match utils::is_safe_digest(digest) {
    true => Ok(()),
    false => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe digest: {}", digest))),
  }
}
fn is_safe_reference(reference: &str) -> Result<(), io::Error> {
  match utils::verify_reference(reference) {
    Ok(_) => Ok(()),
    Err(_) => {
      Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe reference: {}", reference)))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_xattr() {
    let file = tempfile::tempfile_in("/var/tmp").unwrap();
    set_xattr_media_type(&file, "application/vnd.docker.distribution.manifest.v2+json").unwrap();

    let media_type = get_xattr_media_type(&file).unwrap();
    assert_eq!(media_type, "application/vnd.docker.distribution.manifest.v2+json");
  }
}
