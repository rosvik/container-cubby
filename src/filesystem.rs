use std::fs::{DirBuilder, File};
use std::io;

const DATA_DIR: &str = "data";

fn make_dir(folder: &str, digest: &str) -> Result<String, io::Error> {
  let digest = digest.replace("sha256:", "");
  let prefix = digest.chars().take(2).collect::<String>();
  let directory = format!("{DATA_DIR}/{folder}/{prefix}/");
  DirBuilder::new().recursive(true).create(&directory)?;
  Ok(directory)
}

fn get_file_name(digest: &str, extension: &str) -> String {
  let digest = digest.replace("sha256:", "");
  let digest = format!("{}.{extension}", digest);
  digest.chars().skip(2).collect::<String>()
}

pub fn create_blob_file(digest: &str) -> Result<File, io::Error> {
  let directory = make_dir("blobs", digest)?;
  let file_name = get_file_name(digest, "blob");

  let file = File::create(format!("{directory}/{file_name}"))?;
  Ok(file)
}

pub fn get_blob_file(digest: &str) -> Result<File, io::Error> {
  let directory = make_dir("blobs", digest)?;
  let file_name = get_file_name(digest, "blob");

  let file = File::open(format!("{directory}/{file_name}"))?;
  Ok(file)
}

pub fn create_manifest_file(digest: &str) -> Result<File, io::Error> {
  let directory = make_dir("manifests", digest)?;
  let file_name = get_file_name(digest, "json");

  let file = File::create(format!("{directory}/{file_name}"))?;
  Ok(file)
}

pub fn get_manifest_file(digest: &str) -> Result<File, io::Error> {
  let directory = make_dir("manifests", digest)?;
  let file_name = get_file_name(digest, "json");

  let file = File::open(format!("{directory}/{file_name}"))?;
  Ok(file)
}
