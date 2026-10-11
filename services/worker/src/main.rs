//! ImageForge conversion worker.
//!
//! Consumes job ids from Redis Streams, loads the job from Postgres, runs the
//! conversion engine on the input object, stores the result, and advances the
//! job's lifecycle — with bounded retries and dead-lettering. It also relays
//! the transactional outbox to the stream so jobs created by the API are
//! delivered even if a direct enqueue failed.
//!
//! The pure conversion step [`processing::process_conversion`] works over the
//! `ObjectStore` trait and is unit-tested with an in-memory store. The
//! orchestration loop requires live Postgres/Redis/R2 and is exercised
//! end-to-end once those are configured.

use imageforge_worker_core as processing;
use std::time::Duration;

use imageforge_database::Db;
use imageforge_job_queue::{JobId, JobStatus, RetryPolicy};
use imageforge_redis_queue::{Delivery, RedisQueue};
use imageforge_storage::{S3Store, StorageConfig};

const STREAM: &str = "imageforge:jobs";
const GROUP: &str = "workers";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "imageforge_worker=info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")?;
    let redis_url = std::env::var("REDIS_URL")?;

    let db = Db::connect(&database_url).await?;
    db.migrate().await?;
    let queue = RedisQueue::connect(&redis_url, STREAM, GROUP).await?;
    let store = S3Store::from_env()?;
    let cfg = StorageConfig::from_env();
    let policy = RetryPolicy::default();
    let consumer = format!("worker-{}", std::process::id());

    tracing::info!("worker {consumer} started");

    loop {
        if let Err(e) = relay_outbox(&db, &queue).await {
            tracing::warn!("outbox relay error: {e}");
        }

        match queue.read_new(&consumer, 10, 5000).await {
            Ok(deliveries) => {
                for delivery in deliveries {
                    if let Err(e) = handle(&db, &queue, &store, &cfg, &policy, delivery).await {
                        tracing::error!("job handling error: {e}");
                    }
                }
            }
            Err(e) => {
                tracing::error!("queue read error: {e}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Publish pending outbox rows to the stream, then mark them published.
async fn relay_outbox(db: &Db, queue: &RedisQueue) -> Result<(), Box<dyn std::error::Error>> {
    for row in db.unpublished_outbox(100).await? {
        queue.enqueue(JobId(row.aggregate_id)).await?;
        db.mark_outbox_published(row.id).await?;
    }
    Ok(())
}

/// Process one delivered job: run it, store the result, advance its lifecycle,
/// and acknowledge the stream entry.
async fn handle(
    db: &Db,
    queue: &RedisQueue,
    store: &S3Store,
    cfg: &StorageConfig,
    policy: &RetryPolicy,
    delivery: Delivery,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(mut job) = db.fetch_job(delivery.job_id).await? else {
        // Unknown job id — nothing to do; drop it from the stream.
        queue.ack(&delivery.entry_id).await?;
        return Ok(());
    };

    if job.status.is_terminal() {
        queue.ack(&delivery.entry_id).await?;
        return Ok(());
    }

    // Queued -> Running (records an attempt).
    if job.transition(JobStatus::Running).is_err() {
        queue.ack(&delivery.entry_id).await?;
        return Ok(());
    }
    db.update_job(&job).await?;

    match processing::process_conversion(store, cfg, &job).await {
        Ok(output_key) => {
            job.output_key = Some(output_key);
            job.last_error = None;
            job.transition(JobStatus::Completed)?;
            db.update_job(&job).await?;
            queue.ack(&delivery.entry_id).await?;
            tracing::info!(job = %job.id, "completed");
        }
        Err(err) => {
            let code = err.code();
            job.last_error = Some(code.to_string());
            job.transition(JobStatus::Failed)?;
            if policy.should_retry(job.attempts) {
                // Requeue for another attempt after backoff.
                job.transition(JobStatus::Queued)?;
                db.update_job(&job).await?;
                queue.ack(&delivery.entry_id).await?;
                tokio::time::sleep(Duration::from_millis(policy.backoff_ms(job.attempts))).await;
                queue.enqueue(job.id).await?;
                tracing::warn!(job = %job.id, attempts = job.attempts, "retry scheduled: {code}");
            } else {
                job.transition(JobStatus::DeadLettered)?;
                db.update_job(&job).await?;
                queue.ack(&delivery.entry_id).await?;
                tracing::error!(job = %job.id, "dead-lettered after {} attempts: {code}", job.attempts);
            }
        }
    }

    Ok(())
}
