/*
Prune modes:
- Dangling blobs. If no symlinks in the container directory links to a blob, delete the blob.
- Dangling manifests. If a manifest is not linked to by a tag or image index, delete the manifest.
- Dangling blob links. If a blob link is not referenced by a manifest, delete the blob link.

Running `container-cubby --prune untagged` could
1. delete all manifests that are not linked to by a tag
2. delete all blob links that are not linked to by a manifest
3. delete all blobs that are not linked to by a blob link ✅

In that case, there are two paths for files to be considered "in use":
- Tag -> Image Index -> Manifests -> Blob Links -> Blob
- Tag -> Manifest -> Blob Links -> Blob
*/

use crate::{env, schemas, storage::file};
use std::{fmt, fs, path::Path};

#[allow(dead_code)]
pub enum PruneMode {
  /// If no symlinks in the container directory links to a blob, delete the blob.
  DanglingBlobs,
  /// If a manifest is not linked to by a tag, delete the manifest.
  DanglingManifests,
  /// If a blob link is not referenced by a manifest, delete the blob link.
  BlobLinks,
}
impl fmt::Display for PruneMode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      PruneMode::DanglingBlobs => write!(f, "dangling blobs"),
      PruneMode::DanglingManifests => write!(f, "dangling manifests"),
      PruneMode::BlobLinks => write!(f, "blob links"),
    }
  }
}

#[allow(dead_code)]
pub fn prune(mode: PruneMode, dry_run: bool) {
  println!("Pruning {mode} from database");

  let result = match mode {
    PruneMode::DanglingBlobs => prune_dangling_blobs(),
    PruneMode::DanglingManifests => get_dangling_manifests(),
    PruneMode::BlobLinks => get_blob_links(),
  };
  if result.is_empty() {
    println!("No {mode} to delete");
    return;
  }
  if !dry_run {
    println!("Deleting {} files:", result.len());
    for item in result {
      println!("{item}");
      file::delete(&item).unwrap();
    }
  } else {
    println!("Would delete {} files:", result.len());
    for item in result {
      println!("{item}");
    }
  }
}

fn prune_dangling_blobs() -> Vec<String> {
  let all_blob_shas = list_all_blob_shas();
  let all_blob_link_targets = list_all_blob_link_target_shas();

  // Find all blobs that are not in all_blob_link_targets
  all_blob_shas
    .iter()
    .filter(|sha| !all_blob_link_targets.contains(sha))
    .map(|s| s.to_string())
    .collect::<Vec<String>>()
}

fn get_dangling_manifests() -> Vec<String> {
  let containers_dir = format!("{}/containers", env::data_dir());
  let manifests = recursively_find(&containers_dir, |path| {
    path.ends_with(".json") && path.split("/").last().unwrap().starts_with("sha256:")
  });

  // Search through the directory of each manifest to find a Tag that targets the manifest
  manifests
    .iter()
    .filter(|manifest| {
      check_in_parent_dir(manifest, |parent_dir, file_name| {
        let manifest_digest = file_name.strip_suffix(".json").unwrap();
        for dir_entry in parent_dir {
          let dir_entry = dir_entry.unwrap();

          // Skip non-manifest files
          if !dir_entry.path().to_str().unwrap().ends_with(".json") {
            continue;
          }

          // Is the file a Tag?
          if dir_entry.path().is_symlink() {
            let symlink_target = fs::read_link(dir_entry.path()).unwrap();

            // Does the Tag target the manifest?
            if symlink_target.to_str().unwrap() == file_name {
              // The manifest is linked to by a tag, so it is not dangling
              return false;
            }
          }

          // Is the file referenced by an image index?
          if dir_entry.path().is_file() {
            let file_content = fs::read_to_string(dir_entry.path()).unwrap();
            if let Ok(image_index) = serde_json::from_str::<schemas::ImageIndex>(&file_content) {
              if image_index.manifests.iter().any(|manifest| manifest.digest == manifest_digest) {
                // The manifest is referenced by an image index, so it is not dangling
                return false;
              }
            }
          }
        }
        // The manifest is not linked to by a tag or image index, so it is dangling
        true
      })
    })
    .map(|s| s.to_string())
    .collect::<Vec<String>>()
}
fn get_blob_links() -> Vec<String> {
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
  let containers_dir = format!("{}/containers", env::data_dir());
  let target_paths = recursively_find(&containers_dir, |path| path.ends_with(".blob"));
  target_paths.iter().map(|path| path.replace(".blob", "")).collect::<Vec<String>>()
}

fn recursively_find(dir_path: &str, predicate: impl Fn(&str) -> bool + Clone) -> Vec<String> {
  let dir = fs::read_dir(dir_path).unwrap();
  let mut matches = Vec::new();
  for entry in dir {
    let path = entry.unwrap().path();
    if path.is_dir() {
      matches.extend(recursively_find(path.to_str().unwrap(), predicate.clone()));
    } else if path.is_file() && predicate(path.to_str().unwrap()) {
      matches.push(path.to_str().unwrap().to_string());
    }
  }
  matches
}

fn check_in_parent_dir(
  file_path: &str,
  predicate: impl Fn(fs::ReadDir, &str) -> bool + Clone,
) -> bool {
  let file_path = Path::new(file_path);
  let file_name = file_path.file_name().unwrap().to_str().unwrap();
  let parent_dir = fs::read_dir(file_path.parent().unwrap()).unwrap();
  predicate(parent_dir, file_name)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_prune_dangling_blobs() {
    prune(PruneMode::DanglingManifests, true);
    // prune(PruneMode::DanglingBlobs, true);
  }
}
