use crate::{digest::Digest, utils};

/// The data directory is structured as follows:
///
/// ```txt
/// <DATA_DIR>/
/// ├── containers/
/// │   └── foo/
/// │       └── bar/
/// │           ├── latest.json                      [SYMLINK]
/// │           ├── sha256:<manifest hash>.json
/// │           ├── sha256:abc123.blob               [SYMLINK]
/// │           └── <UUID>.hunk
/// └── blobs/
///     └── ab/
///         └── c123.blob
/// ```
///
/// - **Tag** `latest.json` is a symlink to **Manifest** `sha256:<manifest hash>.json`
/// - **BlobLink** `sha256:abc123.blob` is a symlink to **Blob** `blobs/ab/c123.blob`
/// - **Hunk** `<UUID>.hunk` is a partial blob
pub enum FileType {
  Blob,     // .blob
  BlobLink, // Symlink to Blob
  Hunk,     // .hunk
  Manifest, // .json
  Tag,      // Symlink to Manifest
}

/// Get the path to a file on disk, relative to DATA_DIR.
pub fn get(name: &str, reference: &str, file_type: FileType) -> Result<String, std::io::Error> {
  let is_safe = match file_type {
    FileType::Blob => utils::is_safe_digest(reference),
    FileType::BlobLink => utils::is_safe_digest(reference),
    FileType::Hunk => utils::is_safe_hunk(reference),
    FileType::Manifest => utils::is_safe_digest(reference),
    FileType::Tag => utils::is_safe_tag(reference),
  };
  if !is_safe {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidInput,
      format!("Unsafe reference: {reference}"),
    ));
  }

  match file_type {
    FileType::Blob => {
      let digest = Digest::from_string(reference).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid digest: {e}"))
      })?;
      let file_name = digest.hex.chars().skip(2).collect::<String>();
      Ok(format!("{}/{file_name}.blob", blob_dir(&digest)))
    }
    FileType::BlobLink => Ok(format!("{}/{reference}.blob", container_dir(name)?)),
    FileType::Hunk => Ok(format!("{}/{reference}.hunk", container_dir(name)?)),
    FileType::Manifest => Ok(format!("{}/{reference}.json", container_dir(name)?)),
    FileType::Tag => Ok(format!("{}/{reference}.json", container_dir(name)?)),
  }
}

/// Returns the directory where a container should be stored, relative to DATA_DIR.
///
/// ```rust
/// let container_dir = container_dir("foo/bar").unwrap();
/// assert_eq!(container_dir, "containers/foo/bar");
/// ```
pub fn container_dir(name: &str) -> Result<String, std::io::Error> {
  if !utils::is_safe_name(name) {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidInput,
      format!("Unsafe name: {name}"),
    ));
  }
  let container_dir = format!("containers/{name}");
  Ok(container_dir)
}

/// Returns the directory where a blob should be stored, relative to DATA_DIR.
///
/// ```rust
/// let blob_dir = blob_dir("sha256:1234").unwrap();
/// assert_eq!(blob_dir, "blobs/12");
/// ```
pub fn blob_dir(digest: &Digest) -> String {
  format!("blobs/{}", digest.prefix())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_blob_dir() {
    let digest = "sha256:f52fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7";
    let digest = Digest::from_string(digest).unwrap();
    let blob_dir = blob_dir(&digest);
    assert_eq!(blob_dir, "blobs/f5");
  }

  #[test]
  fn test_container_dir() {
    let name = "foo/bar";
    let container_dir = container_dir(name).unwrap();
    assert_eq!(container_dir, "containers/foo/bar");
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
      "blobs/f5/2fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7.blob"
    );

    // BlobLink
    let blob_link_path = get(name, digest, FileType::BlobLink).unwrap();
    assert_eq!(
      blob_link_path,
      "containers/foo/bar/sha256:f52fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7.blob"
    );

    // Hunk
    let hunk_path = get(name, hunk, FileType::Hunk).unwrap();
    assert_eq!(hunk_path, "containers/foo/bar/35003fde-9a27-4b01-a296-1337deadbeef.hunk");

    // Manifest
    let manifest_path = get(name, digest, FileType::Manifest).unwrap();
    assert_eq!(
      manifest_path,
      "containers/foo/bar/sha256:f52fbd32b2b3b86ff88ef6c490628285f482af15ddcb29541f94bcf526a3f6c7.json"
    );

    // Tag
    let tag_path = get(name, tag, FileType::Tag).unwrap();
    assert_eq!(tag_path, "containers/foo/bar/latest.json");
  }
}
