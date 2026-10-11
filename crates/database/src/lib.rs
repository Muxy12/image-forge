//! PostgreSQL persistence for ImageForge (sqlx, runtime-checked queries).
//!
//! Stores uploads, conversion jobs, and a transactional outbox. Job creation
//! writes the job row and its outbox row in a single transaction, so a job is
//! never queued without being persisted and never persisted without a pending
//! enqueue record — the basis for at-least-once delivery.
//!
//! Queries are runtime-checked (no `sqlx::query!` macros), so the crate builds
//! without a database connection. Schema lives in `/migrations` and is embedded
//! via `sqlx::migrate!` and applied by [`Db::migrate`].

use chrono::{DateTime, Utc};
use imageforge_job_queue::{ConversionSpec, Job, JobId, JobStatus};
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Errors from the database layer.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("unknown job status in database: {0}")]
    UnknownStatus(String),
}

/// A connected database handle.
#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    /// Connect with a bounded pool.
    pub async fn connect(database_url: &str) -> Result<Self, DbError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    /// Wrap an existing pool (e.g. shared with the API).
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Apply embedded migrations.
    pub async fn migrate(&self) -> Result<(), DbError> {
        sqlx::migrate!("../../migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Record a verified upload; returns its id.
    pub async fn create_upload(
        &self,
        object_key: &str,
        content_type: &str,
        byte_size: i64,
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, DbError> {
        let row = sqlx::query(
            "INSERT INTO uploads (object_key, content_type, byte_size, verified, expires_at)
             VALUES ($1, $2, $3, TRUE, $4) RETURNING id",
        )
        .bind(object_key)
        .bind(content_type)
        .bind(byte_size)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get::<Uuid, _>("id"))
    }

    /// Create a queued job and its outbox row in one transaction.
    pub async fn create_job(
        &self,
        spec: &ConversionSpec,
        input_key: &str,
        api_key_id: Option<Uuid>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Job, DbError> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query(
            "INSERT INTO conversion_jobs (status, attempts, spec, input_key, api_key_id, expires_at)
             VALUES ('queued', 0, $1, $2, $3, $4) RETURNING id",
        )
        .bind(Json(spec))
        .bind(input_key)
        .bind(api_key_id)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;
        let id: Uuid = row.get("id");

        sqlx::query(
            "INSERT INTO outbox (aggregate_id, topic, payload) VALUES ($1, 'conversion.job.queued', $2)",
        )
        .bind(id)
        .bind(Json(serde_json::json!({ "job_id": id })))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Job {
            id: JobId(id),
            status: JobStatus::Queued,
            attempts: 0,
            spec: spec.clone(),
            input_key: input_key.to_string(),
            output_key: None,
            last_error: None,
        })
    }

    /// Fetch a job by id.
    pub async fn fetch_job(&self, id: JobId) -> Result<Option<Job>, DbError> {
        let row = sqlx::query(
            "SELECT id, status, attempts, spec, input_key, output_key, last_error
             FROM conversion_jobs WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else { return Ok(None) };
        Ok(Some(Job {
            id: JobId(row.get::<Uuid, _>("id")),
            status: parse_status(row.get::<String, _>("status").as_str())?,
            attempts: row.get::<i32, _>("attempts") as u32,
            spec: row.get::<Json<ConversionSpec>, _>("spec").0,
            input_key: row.get::<String, _>("input_key"),
            output_key: row.get::<Option<String>, _>("output_key"),
            last_error: row.get::<Option<String>, _>("last_error"),
        }))
    }

    /// Persist a job's mutable state.
    pub async fn update_job(&self, job: &Job) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE conversion_jobs
             SET status = $2, attempts = $3, output_key = $4, last_error = $5, updated_at = now(),
                 completed_at = CASE WHEN $2 = 'completed' THEN now() ELSE completed_at END
             WHERE id = $1",
        )
        .bind(job.id.0)
        .bind(status_str(job.status))
        .bind(job.attempts as i32)
        .bind(job.output_key.as_deref())
        .bind(job.last_error.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Claim a batch of unpublished outbox rows (for the Redis relay).
    pub async fn unpublished_outbox(&self, limit: i64) -> Result<Vec<OutboxRow>, DbError> {
        let rows = sqlx::query(
            "SELECT id, aggregate_id, topic, payload FROM outbox
             WHERE published_at IS NULL ORDER BY id ASC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OutboxRow {
                id: row.get::<i64, _>("id"),
                aggregate_id: row.get::<Uuid, _>("aggregate_id"),
                topic: row.get::<String, _>("topic"),
                payload: row.get::<Json<serde_json::Value>, _>("payload").0,
            })
            .collect())
    }

    /// Mark an outbox row as published.
    pub async fn mark_outbox_published(&self, id: i64) -> Result<(), DbError> {
        sqlx::query("UPDATE outbox SET published_at = now() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// A pending outbox record awaiting publication to the queue.
#[derive(Debug, Clone)]
pub struct OutboxRow {
    pub id: i64,
    pub aggregate_id: Uuid,
    pub topic: String,
    pub payload: serde_json::Value,
}

fn status_str(s: JobStatus) -> &'static str {
    match s {
        JobStatus::Pending => "pending",
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
        JobStatus::DeadLettered => "dead_lettered",
        JobStatus::Cancelled => "cancelled",
    }
}

fn parse_status(s: &str) -> Result<JobStatus, DbError> {
    Ok(match s {
        "pending" => JobStatus::Pending,
        "queued" => JobStatus::Queued,
        "running" => JobStatus::Running,
        "completed" => JobStatus::Completed,
        "failed" => JobStatus::Failed,
        "dead_lettered" => JobStatus::DeadLettered,
        "cancelled" => JobStatus::Cancelled,
        other => return Err(DbError::UnknownStatus(other.to_string())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrip() {
        for s in [
            JobStatus::Pending,
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Completed,
            JobStatus::Failed,
            JobStatus::DeadLettered,
            JobStatus::Cancelled,
        ] {
            assert_eq!(parse_status(status_str(s)).unwrap(), s);
        }
    }

    #[test]
    fn parse_rejects_unknown_status() {
        assert!(matches!(
            parse_status("bogus"),
            Err(DbError::UnknownStatus(_))
        ));
    }

    /// Full round-trip against a real Postgres. Skipped unless DATABASE_URL is
    /// set (e.g. a Neon connection string), so `cargo test` is green offline.
    #[tokio::test]
    async fn job_lifecycle_against_postgres() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("DATABASE_URL not set — skipping Postgres integration test");
            return;
        };
        let db = Db::connect(&url).await.expect("connect");
        db.migrate().await.expect("migrate");

        let spec = ConversionSpec::default();
        let job = db
            .create_job(&spec, "uploads/test-input.png", None, None)
            .await
            .expect("create_job");
        assert_eq!(job.status, JobStatus::Queued);

        let fetched = db.fetch_job(job.id).await.expect("fetch").expect("exists");
        assert_eq!(fetched.spec, spec);

        let outbox = db.unpublished_outbox(10).await.expect("outbox");
        assert!(outbox.iter().any(|o| o.aggregate_id == job.id.0));

        let mut done = fetched;
        done.status = JobStatus::Completed;
        done.output_key = Some("results/test-output.webp".to_string());
        db.update_job(&done).await.expect("update");
        let reread = db
            .fetch_job(job.id)
            .await
            .expect("fetch2")
            .expect("exists2");
        assert_eq!(reread.status, JobStatus::Completed);
        assert_eq!(
            reread.output_key.as_deref(),
            Some("results/test-output.webp")
        );
    }
}
