use crate::{
  env, schemas,
  storage::{file, path},
};
use std::{
  fs,
  path::{Path, PathBuf},
};
use tokio::time;

/// Do a complete prune of the registry.
///
/// This will:
/// 1. delete all manifests that are not linked to by a tag or image index
/// 2. delete all blob links that are not linked to by a manifest
/// 3. delete all blobs that are not linked to by a blob link
///
/// Files are kept if they are part of one of the following chains:
/// - Tag -> Image Index -> Manifests -> Blob Links -> Blob
/// - Tag -> Manifest -> Blob Links -> Blob
pub fn prune_all(dry_run: bool) {
  fn print_prune_stats(start: time::Instant, mode: PruneMode) {
    println!("{mode:?} pruned in {:?}", start.elapsed());
  }

  let start = time::Instant::now();
  let start_manifests = time::Instant::now();
  prune(PruneMode::Manifests, dry_run);
  print_prune_stats(start_manifests, PruneMode::Manifests);

  let start_blob_links = time::Instant::now();
  prune(PruneMode::BlobLinks, dry_run);
  print_prune_stats(start_blob_links, PruneMode::BlobLinks);

  let start_blobs = time::Instant::now();
  prune(PruneMode::Blobs, dry_run);
  print_prune_stats(start_blobs, PruneMode::Blobs);

  println!("Total prune time: {:?}", start.elapsed());
}

#[derive(Debug)]
pub enum PruneMode {
  /// If a manifest is not linked to by a tag or referenced image index, delete
  /// the manifest.
  Manifests,
  /// If a blob link is not referenced by a manifest, delete the blob link.
  BlobLinks,
  /// If a blob is not linked by a blob link, delete the blob.
  Blobs,
}

pub fn prune(mode: PruneMode, dry_run: bool) {
  println!("Pruning {mode:?} from database");

  let data_dir = env::data_dir();
  let containers_dir = PathBuf::from(format!("{data_dir}/containers"));

  let absolute_paths = match mode {
    PruneMode::Manifests => get_dangling_manifests_in(&containers_dir),
    PruneMode::BlobLinks => get_dangling_blob_links_in(&containers_dir),
    PruneMode::Blobs => get_dangling_blobs(),
  };
  if absolute_paths.is_empty() {
    println!("No {mode:?} to delete");
    return;
  }
  if !dry_run {
    println!("Deleting {} files:", absolute_paths.len());
    for absolute_path in absolute_paths {
      println!("{absolute_path:?}");
      fs::remove_file(absolute_path).unwrap();
    }
  } else {
    println!("Would delete {} files:", absolute_paths.len());
    for item in absolute_paths {
      println!("{item:?}");
    }
  }
}

/// Checks blobs against blob links in the configured data directory if there
/// are any that have no links to them.
fn get_dangling_blobs() -> Vec<PathBuf> {
  let all_blob_shas = list_all_blob_shas();
  let all_blob_link_targets = list_all_blob_link_target_shas();

  // Find all blobs that are not in all_blob_link_targets
  let dangling_blob_shas = all_blob_shas
    .iter()
    .filter(|sha| !all_blob_link_targets.contains(sha))
    .map(|s| s.to_string())
    .collect::<Vec<String>>();

  // Return the paths of the dangling blobs
  dangling_blob_shas
    .iter()
    .map(|sha| path::get("", sha, path::FileType::Blob).unwrap())
    .map(|s| PathBuf::from(format!("{}/{s}", env::data_dir())))
    .collect::<Vec<PathBuf>>()
}

/// Recursively finds all manifests in the given directory, and check if sibling
/// files are tags or image indexes that reference the manifest.
fn get_dangling_manifests_in(directory: &PathBuf) -> Vec<PathBuf> {
  let manifests = recursively_find(directory, |path| {
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
            if let Ok(image_index) = serde_json::from_reader::<_, schemas::ImageIndex>(
              &fs::File::open(dir_entry.path()).unwrap(),
            ) {
              if image_index.manifests.iter().any(|manifest| manifest.digest == manifest_digest) {
                // The manifest is referenced by an image index, so it is not dangling
                return false;
              }
            }
          }
        }
        // The manifest is not linked to by a tag or image index, so it is
        // dangling
        true
      })
    })
    .map(|s| s.to_owned())
    .collect::<Vec<PathBuf>>()
}

/// Recursively finds all blob links in the given directory, and check if
/// sibling files are manifests that reference the blob link.
fn get_dangling_blob_links_in(directory: &PathBuf) -> Vec<PathBuf> {
  let blob_links = recursively_find(directory, |path| path.ends_with(".blob"));
  blob_links
    .iter()
    .filter(|blob_link| {
      check_in_parent_dir(blob_link, |mut parent_dir, file_name| {
        let digest = file_name.strip_suffix(".blob").unwrap();

        // If no manifest references the blob link, it is dangling
        !parent_dir.any(|dir_entry| {
          let path = dir_entry.unwrap().path();
          let file_name = path.file_name().unwrap().to_str().unwrap();

          // Read all manifest files (excluding Tags)
          if !file_name.ends_with(".json") || !file_name.starts_with("sha256:") {
            // Skip non-manifest files
            return false;
          }

          let manifest: Option<schemas::ImageManifest> =
            serde_json::from_reader(fs::File::open(path).unwrap()).ok();

          // Does any layer reference the blob link?
          manifest
            .is_some_and(|manifest| manifest.layers.iter().any(|layer| layer.digest == digest))
        })
      })
    })
    .map(|s| s.to_owned())
    .collect::<Vec<PathBuf>>()
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
  let containers_dir = PathBuf::from(format!("{}/containers", env::data_dir()));
  let blob_link_paths = recursively_find(&containers_dir, |path| path.ends_with(".blob"));
  blob_link_paths
    .iter()
    .map(|path| path.to_str().unwrap().split("/").last().unwrap().replace(".blob", ""))
    .collect::<Vec<String>>()
}

fn recursively_find(dir_path: &PathBuf, predicate: impl Fn(&str) -> bool + Clone) -> Vec<PathBuf> {
  let dir = fs::read_dir(dir_path).unwrap();
  let mut matches = Vec::new();
  for entry in dir {
    let path = entry.unwrap().path();
    if path.is_dir() {
      matches.extend(recursively_find(&path, predicate.clone()));
    } else if path.is_file() && predicate(path.to_str().unwrap()) {
      matches.push(path);
    }
  }
  matches
}

fn check_in_parent_dir(
  file_path: &Path,
  predicate: impl Fn(fs::ReadDir, &str) -> bool + Clone,
) -> bool {
  let file_name = file_path.file_name().unwrap().to_str().unwrap();
  let parent_dir = fs::read_dir(file_path.parent().unwrap()).unwrap();
  predicate(parent_dir, file_name)
}

#[cfg(test)]
mod tests {
  use std::io::Write;

  use super::*;
  use crate::{digestor, storage, tests};
  use uuid::Uuid;

  #[test]
  fn test_prune_all() {
    // Setup data directory if it doesn't exist
    let data_dir = env::data_dir();
    let containers_dir = format!("{data_dir}/containers");
    let blobs_dir = format!("{data_dir}/blobs");
    let _ = fs::create_dir_all(containers_dir);
    let _ = fs::create_dir_all(blobs_dir);

    prune_all(true);
  }

  #[test]
  fn test_dangling_manifests_with_tag() {
    let data_dir = env::data_dir();
    let namespace = tests::utils::get_random_namespace();
    let directory = PathBuf::from(format!("{data_dir}/containers/{namespace}"));

    // Create a tagged manifest
    let mut manifest_file = storage::create_manifest(
      &namespace,
      "sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f",
      Some("latest"),
    )
    .unwrap();
    manifest_file
      .write_all(include_str!("../tests/fixtures/image_manifest.json").as_bytes())
      .unwrap();
    storage::xattr::set_xattr_media_type(
      &manifest_file,
      "application/vnd.docker.distribution.manifest.v2+json",
    )
    .unwrap();

    let dangling_manifests = get_dangling_manifests_in(&directory);

    println!("Dangling manifests: {dangling_manifests:?}");
    assert_eq!(dangling_manifests.len(), 0);

    // Delete the tag file to make the manifest dangling
    fs::remove_file(format!("{}/latest.json", directory.to_str().unwrap())).unwrap();

    // Get dangling manifests again
    let dangling_manifests = get_dangling_manifests_in(&directory);
    assert_eq!(dangling_manifests.len(), 1);
    assert!(dangling_manifests[0]
      .ends_with("sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f.json"));
  }

  #[test]
  fn test_dangling_manifests_with_image_index() {
    let data_dir = env::data_dir();
    let namespace = tests::utils::get_random_namespace();
    let directory = PathBuf::from(format!("{data_dir}/containers/{namespace}"));

    // Create an image index
    let mut index_file = storage::create_manifest(
      &namespace,
      "sha256:3c4006efc2e8c079b3244a070619746b36c1c5ab2eff30debc847acc489d763b",
      Some("latest"),
    )
    .unwrap();
    index_file.write_all(include_str!("../tests/fixtures/image_index.json").as_bytes()).unwrap();
    storage::xattr::set_xattr_media_type(&index_file, "application/vnd.oci.image.index.v1+json")
      .unwrap();

    // Create a manifest (referenced by the image index)
    let mut manifest_file = storage::create_manifest(
      &namespace,
      "sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f",
      None,
    )
    .unwrap();
    manifest_file
      .write_all(include_str!("../tests/fixtures/image_manifest.json").as_bytes())
      .unwrap();
    storage::xattr::set_xattr_media_type(
      &manifest_file,
      "application/vnd.docker.distribution.manifest.v2+json",
    )
    .unwrap();

    let dangling_manifests = get_dangling_manifests_in(&directory);

    println!("Dangling manifests: {dangling_manifests:?}");
    assert_eq!(dangling_manifests.len(), 0);

    // Delete the index file to make the manifest dangling
    fs::remove_file(format!(
      "{}/sha256:3c4006efc2e8c079b3244a070619746b36c1c5ab2eff30debc847acc489d763b.json",
      directory.to_str().unwrap()
    ))
    .unwrap();

    // Get dangling manifests again
    let dangling_manifests = get_dangling_manifests_in(&directory);
    assert_eq!(dangling_manifests.len(), 1);
    assert!(dangling_manifests[0]
      .ends_with("sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f.json"));
  }

  #[test]
  fn test_dangling_blob_links() {
    let data_dir = env::data_dir();
    let namespace = tests::utils::get_random_namespace();
    let directory = PathBuf::from(format!("{data_dir}/containers/{namespace}"));

    // Create a manifest
    let mut manifest_file = storage::create_manifest(
      &namespace,
      "sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f",
      None,
    )
    .unwrap();
    manifest_file
      .write_all(include_str!("../tests/fixtures/image_manifest.json").as_bytes())
      .unwrap();
    storage::xattr::set_xattr_media_type(
      &manifest_file,
      "application/vnd.docker.distribution.manifest.v2+json",
    )
    .unwrap();

    // Create a blob and blob link (referenced by the manifest)
    let _ = storage::create_blob(
      &namespace,
      "sha256:f9a3bdbb589d05a43b5fe12df2b42d885b94f0c56f46254c07b39b0526b4728b",
    );

    let dangling_blob_links = get_dangling_blob_links_in(&directory);
    assert_eq!(dangling_blob_links.len(), 0);

    // Delete the manifest to make the blob link dangling
    fs::remove_file(format!(
      "{}/sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f.json",
      directory.to_str().unwrap()
    ))
    .unwrap();

    let dangling_blob_links = get_dangling_blob_links_in(&directory);
    assert_eq!(dangling_blob_links.len(), 1);
    assert!(dangling_blob_links[0]
      .ends_with("sha256:f9a3bdbb589d05a43b5fe12df2b42d885b94f0c56f46254c07b39b0526b4728b.blob"));
  }

  #[test]
  fn test_dangling_blobs() {
    let namespace = tests::utils::get_random_namespace();
    let blob_data = Uuid::new_v4().to_string();
    let blob_sha = digestor::get_sha256_digest(&blob_data.as_bytes().to_vec());
    let blob_path = PathBuf::from(format!(
      "{}/{}",
      env::data_dir(),
      storage::path::get(&namespace, blob_sha.as_str(), storage::path::FileType::Blob).unwrap()
    ));

    // Create a blob (ignore error if it already exists)
    if let Ok(mut file) = storage::create_blob(&namespace, blob_sha.as_str()) {
      file.write_all(blob_data.as_bytes()).unwrap();
    }

    let dangling_blobs = get_dangling_blobs();
    println!("Dangling blobs: {dangling_blobs:?} (Looking for {blob_path:?})");
    assert!(!dangling_blobs.contains(&blob_path));

    // Deletes the blob link, making the blob dangling
    storage::delete_blob(&namespace, blob_sha.as_str()).unwrap();

    let dangling_blobs = get_dangling_blobs();
    println!("Dangling blobs: {dangling_blobs:?} (Looking for {blob_path:?})");
    assert!(dangling_blobs.contains(&blob_path));
  }
}
