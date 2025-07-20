/*
Structure:
Here, latest.json is a symlink to sha256:<manifest hash>.json, and sha256:abc123.blob is a symlink to blobs/ab/c123.blob.

<DATA_DIR>/
├── containers/
│   └── rosvik/
│       └── container-cubby/
│           ├── latest.json                      [SYMLINK]
│           ├── sha256:<manifest hash>.json
│           └── sha256:abc123.blob               [SYMLINK]
└── blobs/
    └── ab/
        └── c123.blob


Prune modes:
- Dangling blobs. If no symlinks in the container directory links to a blob, delete the blob.
- Dangling manifests. If a manifest is not linked to by a tag, delete the manifest.
- Dangling blob links. If a blob link is not referenced by a manifest, delete the blob link.

Running `container-cubby --prune untagged` could
1. delete all manifests that are not linked to by a tag
2. delete all blob links that are not linked to by a manifest
3. delete all blobs that are not linked to by a blob link
*/

use crate::{
  env,
  storage::{file, path},
};
use std::fs;

pub enum PruneMode {
  /// If no symlinks in the container directory links to a blob, delete the blob.
  DanglingBlobs,
  /// If a manifest is not linked to by a tag, delete the manifest.
  DanglingManifests,
  /// If a blob link is not referenced by a manifest, delete the blob link.
  BlobLinks,
}

#[allow(dead_code)]
pub fn prune(mode: PruneMode, dry_run: bool) {
  match mode {
    PruneMode::DanglingBlobs => prune_dangling_blobs(dry_run),
    PruneMode::DanglingManifests => prune_dangling_manifests(dry_run),
    PruneMode::BlobLinks => prune_blob_links(dry_run),
  }
}

fn prune_dangling_blobs(dry_run: bool) {
  let all_blob_shas = list_all_blob_shas();
  let all_blob_link_targets = list_all_blob_link_target_shas();

  // Find all blobs that are not in all_blob_link_targets
  let dangling_blobs = all_blob_shas
    .iter()
    .filter(|sha| !all_blob_link_targets.contains(sha))
    .collect::<Vec<&String>>();

  println!("Found {} dangling blobs", dangling_blobs.len());
  for blob in dangling_blobs {
    let blob_path = path::get("", blob, path::FileType::Blob).unwrap();
    if !dry_run {
      println!("Deleting dangling blob: {blob_path}");
      file::delete(&blob_path).unwrap();
    } else {
      println!("(dry-run) Would delete dangling blob: {blob_path}");
    }
  }
}
fn prune_dangling_manifests(_dry_run: bool) {
  panic!("Not implemented");
}
fn prune_blob_links(_dry_run: bool) {
  panic!("Not implemented");
}

fn list_all_blob_shas() -> Vec<String> {
  let blob_dir = file::read_dir("blobs").unwrap();
  let mut shas = Vec::new();
  for prefix_dir in blob_dir {
    let prefix_dir = prefix_dir.unwrap();
    let prefix_dir_path = prefix_dir.path();
    let prefix = prefix_dir_path.file_name().unwrap().to_str().unwrap();
    let blobs = fs::read_dir(prefix_dir_path.clone()).unwrap();
    for blob in blobs {
      let blob_path = blob.unwrap().path();
      let blob_name = blob_path.file_name().unwrap().to_str().unwrap();
      let sha = format!("sha256:{prefix}{}", blob_name.replace(".blob", ""));
      shas.push(sha);
    }
  }
  shas
}

fn list_all_blob_link_target_shas() -> Vec<String> {
  let blob_dir = format!("{}/containers", env::data_dir());
  let target_paths = recursively_find(&blob_dir, ".blob");
  target_paths.iter().map(|path| path.replace(".blob", "")).collect::<Vec<String>>()
}

fn recursively_find(dir_path: &str, ends_with: &str) -> Vec<String> {
  let dir = fs::read_dir(dir_path).unwrap();
  let mut matches = Vec::new();
  for entry in dir {
    let path = entry.unwrap().path();
    if path.is_dir() {
      matches.extend(recursively_find(path.to_str().unwrap(), ends_with));
    } else if path.is_file() && path.file_name().unwrap().to_str().unwrap().ends_with(ends_with) {
      matches.push(path.to_str().unwrap().to_string());
    }
  }
  matches
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_prune_dangling_blobs() {
    prune(PruneMode::DanglingBlobs, false);
  }
}
