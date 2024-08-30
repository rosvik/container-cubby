use regex_lite::Regex;
use std::fs::{DirBuilder, File};
use std::io;

const DATA_DIR: &str = "data";

fn make_blobs_dir(folder: &str, digest: &str) -> Result<String, io::Error> {
  let digest = digest.replace("sha256:", "");
  let prefix = digest.chars().take(2).collect::<String>();
  let directory = format!("{DATA_DIR}/{folder}/{prefix}/");
  DirBuilder::new().recursive(true).create(&directory)?;
  Ok(directory)
}

fn make_named_dir(folder: &str, name: &str) -> Result<String, io::Error> {
  let directory = format!("{DATA_DIR}/{folder}/{name}/");
  DirBuilder::new().recursive(true).create(&directory)?;
  Ok(directory)
}

fn get_blob_file_name(digest: &str, extension: &str) -> String {
  let digest = digest.replace("sha256:", "");
  let digest = format!("{}.{extension}", digest);
  digest.chars().skip(2).collect::<String>()
}

fn get_manifest_file_name(digest: &str, extension: &str) -> String {
  match digest.starts_with("sha256:") {
    true => {
      let digest = digest.replace("sha256:", "");
      format!("{}.{extension}", digest)
    }
    false => {
      format!("{}.{extension}", digest)
    }
  }
}

pub fn create_blob_file(digest: &str) -> Result<File, io::Error> {
  let directory = make_blobs_dir("blobs", digest)?;
  let file_name = get_blob_file_name(digest, "blob");

  let file = File::create(format!("{directory}/{file_name}"))?;
  Ok(file)
}

pub fn get_blob_file(digest: &str) -> Result<File, io::Error> {
  let directory = make_blobs_dir("blobs", digest)?;
  let file_name = get_blob_file_name(digest, "blob");

  let file = File::open(format!("{directory}/{file_name}"))?;
  Ok(file)
}

/// Create a manifest file
///
/// - name: &str - Namespace. MUST match the following regular expression:
///   `[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*(\/[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*)*`
///
/// - reference: &str - Reference as a tag MUST be at most 128 characters in
///   length and MUST match the following regular expression:
///   `[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}`
pub fn create_manifest_file(name: &str, reference: &str) -> Result<File, io::Error> {
  if !is_safe_name(name) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe name: {}", name)));
  }
  if !is_safe_reference(reference) {
    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      format!("Unsafe reference: {}", reference),
    ));
  }

  let directory = make_named_dir("manifests", name)?;
  let file_name = get_manifest_file_name(reference, "json");

  let file = File::create(format!("{directory}/{file_name}"))?;
  Ok(file)
}

pub fn get_manifest_file(name: &str, reference: &str) -> Result<File, io::Error> {
  if !is_safe_name(name) {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Unsafe name: {}", name)));
  }
  if !is_safe_reference(reference) {
    return Err(io::Error::new(
      io::ErrorKind::InvalidInput,
      format!("Unsafe reference: {}", reference),
    ));
  }

  let directory = make_named_dir("manifests", name)?;
  let file_name = get_manifest_file_name(reference, "json");

  let file = File::open(format!("{directory}/{file_name}"))?;
  Ok(file)
}

pub fn is_safe_name(path: &str) -> bool {
  let re = Regex::new(r"^[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*(\/[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*)*$")
    .unwrap();
  re.is_match(path)
}

pub fn is_safe_reference(reference: &str) -> bool {
  let re = Regex::new(r"^[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}$").unwrap();
  re.is_match(reference)
}
