use crate::env;
use crate::utils;
use std::fs::DirBuilder;

pub enum FileType {
  Blob,     // .blob
  BlobLink, // Symlink to Blob
  Hunk,     // .hunk
  Manifest, // .json
  Tag,      // Symlink to Manifest
}
pub fn get(name: &str, reference: &str, file_type: FileType) -> Result<String, std::io::Error> {
  let is_safe = match file_type {
    FileType::Blob => utils::is_safe_digest(reference),
    FileType::BlobLink => utils::is_safe_digest(reference),
    FileType::Hunk => utils::is_safe_hunk(reference),
    FileType::Manifest => utils::is_safe_digest(reference),
    FileType::Tag => utils::is_safe_reference(reference),
  };
  if !is_safe {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidInput,
      format!("Unsafe reference: {}", reference),
    ));
  }

  match file_type {
    FileType::Blob => {
      let file_name = reference.replace("sha256:", "").chars().skip(2).collect::<String>();
      Ok(format!("{}/{file_name}.blob", blob_dir(reference)?))
    }
    FileType::BlobLink => Ok(format!("{}/{reference}.blob", container_dir(name)?)),
    FileType::Hunk => Ok(format!("{}/{reference}.hunk", container_dir(name)?)),
    FileType::Manifest => Ok(format!("{}/{reference}.json", container_dir(name)?)),
    FileType::Tag => Ok(format!("{}/{reference}.json", container_dir(name)?)),
  }
}

pub fn container_dir(name: &str) -> Result<String, std::io::Error> {
  let data_dir = env::data_dir();
  if !utils::is_safe_name(name) {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidInput,
      format!("Unsafe name: {}", name),
    ));
  }
  let container_dir = format!("{data_dir}/containers/{name}");
  DirBuilder::new().recursive(true).create(&container_dir)?;
  Ok(container_dir)
}
pub fn blob_dir(digest: &str) -> Result<String, std::io::Error> {
  let data_dir = env::data_dir();
  let prefix = digest.replace("sha256:", "").chars().take(2).collect::<String>();
  let blob_dir = format!("{data_dir}/blobs/{prefix}");
  DirBuilder::new().recursive(true).create(&blob_dir)?;
  Ok(blob_dir)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_blob_dir() {
    let digest = "sha256:f52fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7";
    let blob_dir = blob_dir(digest).unwrap();
    assert_eq!(blob_dir, "test-data/blobs/f5");
  }

  #[test]
  fn test_container_dir() {
    let name = "foo/bar";
    let container_dir = container_dir(name).unwrap();
    assert_eq!(container_dir, "test-data/containers/foo/bar");
  }

  #[test]
  fn test_get_path() {
    let name = "foo/bar";
    let digest = "sha256:f52fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7";
    let tag = "latest";
    let hunk = "35003fde-9a27-4b01-a296-1337deadbeef";

    // Blob
    let blob_path = get(name, digest, FileType::Blob).unwrap();
    assert_eq!(
      blob_path,
      "test-data/blobs/f5/2fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7.blob"
    );

    // BlobLink
    let blob_link_path = get(name, digest, FileType::BlobLink).unwrap();
    assert_eq!(blob_link_path, "test-data/containers/foo/bar/sha256:f52fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7.blob");

    // Hunk
    let hunk_path = get(name, hunk, FileType::Hunk).unwrap();
    assert_eq!(hunk_path, "test-data/containers/foo/bar/35003fde-9a27-4b01-a296-1337deadbeef.hunk");

    // Manifest
    let manifest_path = get(name, digest, FileType::Manifest).unwrap();
    assert_eq!(manifest_path, "test-data/containers/foo/bar/sha256:f52fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7.json");

    // Tag
    let tag_path = get(name, tag, FileType::Tag).unwrap();
    assert_eq!(tag_path, "test-data/containers/foo/bar/latest.json");
  }
}
