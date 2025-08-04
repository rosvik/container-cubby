use std::fs::File;
use xattr::FileExt;

/// Gets the media type of a file by reading the `user.mime_type` extended attribute.
pub fn get_xattr_media_type(file: &File) -> Option<String> {
  let bytes = match file.get_xattr("user.mime_type") {
    Ok(bytes) => bytes?,
    Err(e) => {
      println!("Failed to get media type: {e:?}");
      return None;
    }
  };
  String::from_utf8(bytes).ok()
}

/// Sets the media type of a file by setting the `user.mime_type` extended attribute.
pub fn set_xattr_media_type(file: &File, media_type: &str) -> Result<(), std::io::Error> {
  file.set_xattr("user.mime_type", media_type.as_bytes())
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
