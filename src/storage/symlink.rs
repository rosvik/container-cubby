use crate::env;

/// Creates a symlink using relative paths, so the containing directory can be
/// moved without breaking the symlink. Will overwrite existing symlinks.
/// - `from` is the path to the symlink file
/// - `to` is the path to the target (original) file
///
/// Example:
/// ```
/// create_relative_symlink("foo/bar/latest.json", "foo/bar/sha256:1234.json");
/// ```
pub fn create_relative_symlink(from: &str, to: &str) -> Result<(), std::io::Error> {
  // Disallow symlinks with '..' to prevent directory traversal attacks. This
  // also prevents files like "foo..bar.txt" from being created, but since
  // that's not a valid namespace or digest, this is ok.
  if to.contains("..") || from.contains("..") {
    return Err(std::io::Error::new(
      std::io::ErrorKind::InvalidInput,
      "Symlinks with '..' are disallowed",
    ));
  }

  let relative_path_to_target = get_short_relative_path(from, to);
  let full_path_to_link_file = format!("{}/{from}", env::data_dir());

  // If there already exists a symlink, remove it first.
  if let Ok(metadata) = std::fs::symlink_metadata(&full_path_to_link_file) {
    if metadata.file_type().is_symlink() {
      std::fs::remove_file(&full_path_to_link_file)?;
    }
  }

  std::os::unix::fs::symlink(relative_path_to_target, full_path_to_link_file)
}

/// Finds the relative path with the least amount of directory traversals from
/// `from` to `to`, assuming both inputs are relative to the same directory.
///
/// Example:
/// ```
/// let path = get_short_relative_path(
///   "foo/bar/latest.json",
///   "foo/sha256:1234.json"
/// );
/// assert_eq!(path, "../sha256:1234.json");
/// ```
fn get_short_relative_path(from: &str, to: &str) -> String {
  let mut from_parts = from.split("/").collect::<Vec<&str>>();
  let mut to_parts = to.split("/").collect::<Vec<&str>>();

  // If the first directory in from_path and to_path is the same, it can be
  // ignored.
  while from_parts[0] == to_parts[0] {
    from_parts.remove(0);
    to_parts.remove(0);
  }

  // The number of directory traversals needed is the number of remaining parts
  // in from_parts, minus 1 to compensate for the file name.
  let dir_levels = from_parts.len() - 1;
  let to_path = to_parts.join("/");
  format!("{}{to_path}", "../".repeat(dir_levels))
}

pub fn clean_broken_symlinks_in(dir: &str) -> Result<(), std::io::Error> {
  let absolute_dir = format!("{}/{}", env::data_dir(), dir);
  for entry in std::fs::read_dir(&absolute_dir)? {
    let file_name = entry?.file_name().into_string().unwrap();
    if file_name.ends_with(".json") && !file_name.starts_with("sha256:") {
      let link_path = format!("{absolute_dir}/{file_name}");
      if let Err(error) = std::fs::canonicalize(&link_path) {
        if error.kind() == std::io::ErrorKind::NotFound {
          std::fs::remove_file(&link_path)?;
          let tag_name = &file_name[..file_name.len() - 5];
          println!("Deleted tag: {}", tag_name);
        } else {
          return Err(error);
        }
      };
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::storage::{
    self, ensure_container_dir_exists,
    file::{self, try_read},
  };
  use crate::tests::utils::get_random_namespace;
  use std::io::{Read, Write};

  #[test]
  fn test_relative_symlink() {
    let name: String = get_random_namespace();
    ensure_container_dir_exists(&name).unwrap();

    let from_path =
      crate::storage::path::get(&name, "latest", crate::storage::path::FileType::Tag).unwrap();
    let to_path = crate::storage::path::get(
      &name,
      "sha256:315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3",
      crate::storage::path::FileType::Manifest,
    )
    .unwrap();

    // Set up target file
    let mut file = file::try_create(&to_path).unwrap();
    file.write_all(b"Hello, world!").unwrap();

    // Create the symlink
    create_relative_symlink(&from_path, &to_path).unwrap();

    // Verify that the symlink points to the correct file
    let mut file = try_read(&from_path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"Hello, world!");
  }

  #[test]
  fn test_get_relative_path_to_target() {
    assert_eq!(
      get_short_relative_path("foo/bar/latest.json", "foo/bar/sha256:1234.json"),
      "sha256:1234.json"
    );
    assert_eq!(
      get_short_relative_path("foo/bar/latest.json", "sha256:1234.json"),
      "../../sha256:1234.json"
    );
    assert_eq!(
      get_short_relative_path("foo/bar/latest.json", "foo/sha256:1234.json"),
      "../sha256:1234.json"
    );
    assert_eq!(
      get_short_relative_path("foo/latest.json", "foo/bar/sha256:1234.json"),
      "bar/sha256:1234.json"
    );
    assert_eq!(
      get_short_relative_path("latest.json", "foo/bar/sha256:1234.json"),
      "foo/bar/sha256:1234.json"
    );

    assert_eq!(
      get_short_relative_path("foo/bar/latest.json", "foo/sha256:1234.json"),
      "../sha256:1234.json"
    );
  }

  #[test]
  fn test_clean_broken_symlinks_in() {
    let name: String = get_random_namespace();
    ensure_container_dir_exists(&name).unwrap();

    let from_path =
      crate::storage::path::get(&name, "latest", crate::storage::path::FileType::Tag).unwrap();
    let to_path = crate::storage::path::get(
      &name,
      "sha256:315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3",
      crate::storage::path::FileType::Manifest,
    )
    .unwrap();

    // Create the symlink. Since the target file does not exist, the symlink
    // should be removed in the next step.
    create_relative_symlink(&from_path, &to_path).unwrap();

    // Clean broken symlinks.
    let container_dir = crate::storage::path::container_dir(&name).unwrap();
    clean_broken_symlinks_in(&container_dir).unwrap();

    // Verify that the symlink is gone.
    assert_eq!(
      storage::file::try_read(&from_path).unwrap_err().kind(),
      std::io::ErrorKind::NotFound
    );
  }
}
