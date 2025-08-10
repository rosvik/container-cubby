use crate::{env, storage::prune};
use tokio_cron_scheduler::{Job, JobScheduler};

const PRUNE_DRY_RUN: bool = false;

pub async fn start_scheduler() -> Result<JobScheduler, Box<dyn std::error::Error>> {
  let prune_cron = env::prune_cron();
  let mut scheduler = JobScheduler::new().await?;

  // Prune job
  if let Some(prune_cron) = prune_cron {
    let prune_job = Job::new(prune_cron.as_str(), |_, _| {
      prune::prune_all(PRUNE_DRY_RUN);
    })?;
    scheduler.add(prune_job).await?;
  }

  scheduler.start().await?;
  if let Ok(Some(next_job_in)) = scheduler.time_till_next_job().await {
    println!("Scheduler started, next job in {next_job_in:?}");
  }
  Ok(scheduler)
}
