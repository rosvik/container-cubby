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

  let dir_levels = from.split("/").count() - 1;
  let relative_path_to_target = format!("{}{to}", "../".repeat(dir_levels));

  let full_path_to_link_file = format!("{}/{from}", env::data_dir());

  // If there already exists a symlink, remove it first.
  if let Ok(metadata) = std::fs::symlink_metadata(&full_path_to_link_file) {
    if metadata.file_type().is_symlink() {
      std::fs::remove_file(&full_path_to_link_file)?;
    }
  }

  std::os::unix::fs::symlink(relative_path_to_target, full_path_to_link_file)
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
