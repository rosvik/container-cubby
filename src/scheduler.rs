use crate::storage::prune;
use tokio::time;

const ONE_HOUR: time::Duration = time::Duration::from_secs(3600);
const DRY_RUN: bool = true;

pub async fn scheduler() {
  let mut interval = time::interval(ONE_HOUR);
  // Trigger the initial tick now, to avoid running jobs immediately
  interval.tick().await;

  loop {
    interval.tick().await;

    prune_job().await;
  }
}

pub async fn prune_job() {
  fn print_prune_stats(start: time::Instant, mode: prune::PruneMode) {
    println!("{mode:?} pruned in {:?}", start.elapsed());
  }

  let start = time::Instant::now();
  let start_manifests = time::Instant::now();
  prune::prune(prune::PruneMode::Manifests, DRY_RUN);
  print_prune_stats(start_manifests, prune::PruneMode::Manifests);

  let start_blob_links = time::Instant::now();
  prune::prune(prune::PruneMode::BlobLinks, DRY_RUN);
  print_prune_stats(start_blob_links, prune::PruneMode::BlobLinks);

  let start_blobs = time::Instant::now();
  prune::prune(prune::PruneMode::Blobs, DRY_RUN);
  print_prune_stats(start_blobs, prune::PruneMode::Blobs);

  println!("Total prune time: {:?}", start.elapsed());
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::env;
  use std::fs;

  #[tokio::test]
  async fn test_prune_job() {
    // Setup data directory if it doesn't exist
    let data_dir = env::data_dir();
    let containers_dir = format!("{data_dir}/containers");
    let blobs_dir = format!("{data_dir}/blobs");
    let _ = fs::create_dir_all(containers_dir);
    let _ = fs::create_dir_all(blobs_dir);

    prune_job().await;
  }
}
