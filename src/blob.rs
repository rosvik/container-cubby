use std::{
  fs::{DirBuilder, File},
  io::Write,
};

const DIR: &str = "data/blobs";

pub fn create_blob(digest: &str) -> std::io::Result<()> {
  let digest = digest.replace("sha256:", "");
  let prefix = digest.chars().take(2).collect::<String>();
  let name = digest.chars().skip(2).collect::<String>();

  DirBuilder::new().recursive(true).create(format!("{DIR}/{prefix}"))?;

  let mut file = File::create(format!("{DIR}/{prefix}/{name}.blob"))?;
  file.write_all(b"Hello, world!")?;
  Ok(())
}
