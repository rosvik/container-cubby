use std::fs::{DirBuilder, File};

const DIR: &str = "data/blobs";
pub fn get_blob_dir_path(digest: &str) -> String {
  let digest = digest.replace("sha256:", "");
  let prefix = digest.chars().take(2).collect::<String>();
  format!("{DIR}/{prefix}/")
}
pub fn get_blob_file_name(digest: &str) -> String {
  let digest = digest.replace("sha256:", "");
  let digest = format!("{}.blob", digest);
  digest.chars().skip(2).collect::<String>()
}

pub fn create_blob(digest: &str) -> Result<File, std::io::Error> {
  let directory = get_blob_dir_path(digest);
  DirBuilder::new().recursive(true).create(&directory)?;

  let file_path = format!("{directory}/{}", get_blob_file_name(digest));
  let file = File::create(file_path)?;
  Ok(file)
}
