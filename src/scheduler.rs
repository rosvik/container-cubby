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
  let start = time::Instant::now();
  let start_manifests = time::Instant::now();
  prune::prune(prune::PruneMode::DanglingManifests, DRY_RUN);
  println!("Dangling manifests pruned in {:?}", start_manifests.elapsed());

  let start_blob_links = time::Instant::now();
  prune::prune(prune::PruneMode::BlobLinks, DRY_RUN);
  println!("Blob links pruned in {:?}", start_blob_links.elapsed());

  let start_blobs = time::Instant::now();
  prune::prune(prune::PruneMode::DanglingBlobs, DRY_RUN);
  println!("Dangling blobs pruned in {:?}", start_blobs.elapsed());

  println!("Total prune time: {:?}", start.elapsed());
}
