//! Job delivery over Redis Streams.
//!
//! A conversion job's id is published to a stream; workers read it through a
//! consumer group so each job is delivered to exactly one worker at a time, and
//! redelivered if the worker dies before acknowledging. PostgreSQL remains the
//! source of truth (via the transactional outbox); this stream is only the
//! delivery mechanism.

use imageforge_job_queue::JobId;
use redis::aio::ConnectionManager;
use redis::streams::{StreamAutoClaimReply, StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, Client};
use uuid::Uuid;

/// Field name carrying the job id inside a stream entry.
const FIELD_JOB_ID: &str = "job_id";

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error(transparent)]
    Redis(#[from] redis::RedisError),
    #[error("malformed stream entry: {0}")]
    Malformed(String),
}

/// A job id delivered from the stream, with the entry id needed to ack it.
#[derive(Debug, Clone)]
pub struct Delivery {
    pub entry_id: String,
    pub job_id: JobId,
}

/// Redis Streams job queue.
#[derive(Clone)]
pub struct RedisQueue {
    conn: ConnectionManager,
    stream: String,
    group: String,
}

impl RedisQueue {
    /// Connect and ensure the consumer group exists (creating the stream).
    pub async fn connect(
        url: &str,
        stream: impl Into<String>,
        group: impl Into<String>,
    ) -> Result<Self, QueueError> {
        let client = Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        let q = Self {
            conn,
            stream: stream.into(),
            group: group.into(),
        };
        q.ensure_group().await?;
        Ok(q)
    }

    /// Create the consumer group (idempotent; ignores BUSYGROUP).
    async fn ensure_group(&self) -> Result<(), QueueError> {
        let mut conn = self.conn.clone();
        let res: redis::RedisResult<()> = conn
            .xgroup_create_mkstream(&self.stream, &self.group, "$")
            .await;
        match res {
            Ok(()) => Ok(()),
            Err(e) if e.code() == Some("BUSYGROUP") => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Publish a job id to the stream.
    pub async fn enqueue(&self, job: JobId) -> Result<String, QueueError> {
        let mut conn = self.conn.clone();
        let id: String = conn
            .xadd(&self.stream, "*", &[(FIELD_JOB_ID, job.0.to_string())])
            .await?;
        Ok(id)
    }

    /// Read up to `count` new entries for this consumer, blocking up to
    /// `block_ms` for new work.
    pub async fn read_new(
        &self,
        consumer: &str,
        count: usize,
        block_ms: usize,
    ) -> Result<Vec<Delivery>, QueueError> {
        let mut conn = self.conn.clone();
        let opts = StreamReadOptions::default()
            .group(&self.group, consumer)
            .count(count)
            .block(block_ms);
        let reply: StreamReadReply = conn.xread_options(&[&self.stream], &[">"], &opts).await?;
        parse_reply(reply)
    }

    /// Acknowledge a processed entry so it is not redelivered.
    pub async fn ack(&self, entry_id: &str) -> Result<(), QueueError> {
        let mut conn = self.conn.clone();
        let _: i64 = conn.xack(&self.stream, &self.group, &[entry_id]).await?;
        Ok(())
    }

    /// Reclaim entries idle longer than `min_idle_ms` (recovery of jobs whose
    /// worker died before acking). Returns the reclaimed deliveries.
    pub async fn reclaim_stale(
        &self,
        consumer: &str,
        min_idle_ms: usize,
        count: usize,
    ) -> Result<Vec<Delivery>, QueueError> {
        let mut conn = self.conn.clone();
        let reply: StreamAutoClaimReply = conn
            .xautoclaim_options(
                &self.stream,
                &self.group,
                consumer,
                min_idle_ms,
                "0-0",
                redis::streams::StreamAutoClaimOptions::default().count(count),
            )
            .await?;
        let mut out = Vec::with_capacity(reply.claimed.len());
        for entry in reply.claimed {
            out.push(to_delivery(&entry.id, &entry.map)?);
        }
        Ok(out)
    }
}

fn parse_reply(reply: StreamReadReply) -> Result<Vec<Delivery>, QueueError> {
    let mut out = Vec::new();
    for key in reply.keys {
        for entry in key.ids {
            out.push(to_delivery(&entry.id, &entry.map)?);
        }
    }
    Ok(out)
}

fn to_delivery(
    entry_id: &str,
    map: &std::collections::HashMap<String, redis::Value>,
) -> Result<Delivery, QueueError> {
    let raw = map
        .get(FIELD_JOB_ID)
        .ok_or_else(|| QueueError::Malformed(format!("entry {entry_id} missing {FIELD_JOB_ID}")))?;
    let s = match raw {
        redis::Value::BulkString(bytes) => String::from_utf8_lossy(bytes).to_string(),
        redis::Value::SimpleString(s) => s.clone(),
        other => {
            return Err(QueueError::Malformed(format!(
                "unexpected value: {other:?}"
            )))
        }
    };
    let uuid =
        Uuid::parse_str(&s).map_err(|_| QueueError::Malformed(format!("invalid job id: {s}")))?;
    Ok(Delivery {
        entry_id: entry_id.to_string(),
        job_id: JobId(uuid),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end against a real Redis (e.g. Upstash). Skipped unless REDIS_URL
    /// is set, so `cargo test` is green offline.
    #[tokio::test]
    async fn enqueue_read_ack_against_redis() {
        let Ok(url) = std::env::var("REDIS_URL") else {
            eprintln!("REDIS_URL not set — skipping Redis integration test");
            return;
        };
        // Unique stream/group per run to avoid cross-test interference.
        let suffix = Uuid::new_v4().simple().to_string();
        let stream = format!("test:jobs:{suffix}");
        let group = "workers";
        let q = RedisQueue::connect(&url, stream, group)
            .await
            .expect("connect");

        let job = JobId::new();
        q.enqueue(job).await.expect("enqueue");

        let deliveries = q.read_new("c1", 10, 1000).await.expect("read");
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].job_id, job);

        q.ack(&deliveries[0].entry_id).await.expect("ack");

        // Nothing new remains.
        let empty = q.read_new("c1", 10, 100).await.expect("read2");
        assert!(empty.is_empty());
    }
}
