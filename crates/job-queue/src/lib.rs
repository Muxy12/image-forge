//! Async conversion job domain.
//!
//! Pure, infrastructure-free types for the async processing path: the job
//! lifecycle state machine, a bounded exponential-backoff retry policy, and the
//! serializable conversion spec that travels through the queue and database.
//! This crate has no HTTP, database, Redis, or image-engine dependencies, so
//! the lifecycle rules can be unit-tested in isolation and reused by both the
//! API (which creates jobs) and the worker (which runs them).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identifier for a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub Uuid);

impl JobId {
    pub fn new() -> Self {
        JobId(Uuid::new_v4())
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Lifecycle status of a job. Transitions are validated by
/// [`JobStatus::can_transition_to`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Created, awaiting its upload to be verified.
    Pending,
    /// Published to the queue, awaiting a worker.
    Queued,
    /// Claimed by a worker and processing.
    Running,
    /// Finished successfully; a result is available.
    Completed,
    /// Failed but may be retried.
    Failed,
    /// Permanently failed after exhausting retries.
    DeadLettered,
    /// Cancelled by the owner.
    Cancelled,
}

impl JobStatus {
    /// Whether this is an end state (no further transitions).
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            JobStatus::Completed | JobStatus::DeadLettered | JobStatus::Cancelled
        )
    }

    /// Whether a transition to `next` is allowed from this status.
    pub fn can_transition_to(self, next: JobStatus) -> bool {
        use JobStatus::*;
        matches!(
            (self, next),
            (Pending, Queued)
                | (Pending, Cancelled)
                | (Queued, Running)
                | (Queued, Cancelled)
                | (Running, Completed)
                | (Running, Failed)
                | (Running, Cancelled)
                // A failed job is requeued for another attempt, or dead-lettered.
                | (Failed, Queued)
                | (Failed, DeadLettered)
                | (Failed, Cancelled)
        )
    }
}

/// Errors from job domain operations.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JobError {
    #[error("invalid job transition from {from:?} to {to:?}")]
    InvalidTransition { from: JobStatus, to: JobStatus },
}

/// A serializable description of the requested conversion. Engine-independent
/// (plain strings/numbers) so it can be stored and queued.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionSpec {
    /// Target format token, e.g. "webp".
    pub to: String,
    /// Encoder quality (1..=100) for lossy targets.
    pub quality: u8,
    /// Optional resize width.
    pub width: Option<u32>,
    /// Optional resize height.
    pub height: Option<u32>,
    /// Strip metadata on output.
    pub strip_metadata: bool,
}

impl Default for ConversionSpec {
    fn default() -> Self {
        Self {
            to: "webp".to_string(),
            quality: 82,
            width: None,
            height: None,
            strip_metadata: true,
        }
    }
}

/// A conversion job and its mutable lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub status: JobStatus,
    /// Number of processing attempts started so far.
    pub attempts: u32,
    pub spec: ConversionSpec,
    /// Storage key of the uploaded input object.
    pub input_key: String,
    /// Storage key of the produced output object, once completed.
    pub output_key: Option<String>,
    /// Stable error code from the last failure, if any.
    pub last_error: Option<String>,
}

impl Job {
    /// Create a new `Pending` job for `spec` with the given input object key.
    pub fn new(spec: ConversionSpec, input_key: impl Into<String>) -> Self {
        Self {
            id: JobId::new(),
            status: JobStatus::Pending,
            attempts: 0,
            spec,
            input_key: input_key.into(),
            output_key: None,
            last_error: None,
        }
    }

    /// Apply a validated status transition.
    pub fn transition(&mut self, to: JobStatus) -> Result<(), JobError> {
        if !self.status.can_transition_to(to) {
            return Err(JobError::InvalidTransition {
                from: self.status,
                to,
            });
        }
        if to == JobStatus::Running {
            self.attempts += 1;
        }
        self.status = to;
        Ok(())
    }
}

/// Bounded exponential-backoff retry policy.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Maximum number of attempts before dead-lettering.
    pub max_attempts: u32,
    /// Base delay in milliseconds (delay for the first retry).
    pub base_delay_ms: u64,
    /// Upper bound on any single delay, in milliseconds.
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 500,
            max_delay_ms: 30_000,
        }
    }
}

impl RetryPolicy {
    /// Whether a job that has used `attempts` attempts may be retried.
    pub fn should_retry(&self, attempts: u32) -> bool {
        attempts < self.max_attempts
    }

    /// Delay before the retry that follows `attempts` completed attempts.
    /// `attempts = 1` → `base_delay_ms`, doubling each time, capped at
    /// `max_delay_ms`.
    pub fn backoff_ms(&self, attempts: u32) -> u64 {
        if attempts == 0 {
            return 0;
        }
        let shift = attempts.saturating_sub(1).min(63);
        let factor = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
        self.base_delay_ms
            .saturating_mul(factor)
            .min(self.max_delay_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_transitions() {
        let mut job = Job::new(ConversionSpec::default(), "uploads/in.png");
        assert_eq!(job.status, JobStatus::Pending);
        job.transition(JobStatus::Queued).unwrap();
        job.transition(JobStatus::Running).unwrap();
        assert_eq!(job.attempts, 1);
        job.transition(JobStatus::Completed).unwrap();
        assert!(job.status.is_terminal());
    }

    #[test]
    fn retry_path_increments_attempts() {
        let mut job = Job::new(ConversionSpec::default(), "k");
        job.transition(JobStatus::Queued).unwrap();
        job.transition(JobStatus::Running).unwrap();
        job.transition(JobStatus::Failed).unwrap();
        job.transition(JobStatus::Queued).unwrap(); // requeue
        job.transition(JobStatus::Running).unwrap();
        assert_eq!(job.attempts, 2);
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut job = Job::new(ConversionSpec::default(), "k");
        let err = job.transition(JobStatus::Completed).unwrap_err();
        assert_eq!(
            err,
            JobError::InvalidTransition {
                from: JobStatus::Pending,
                to: JobStatus::Completed
            }
        );
    }

    #[test]
    fn terminal_states_do_not_transition() {
        let mut job = Job::new(ConversionSpec::default(), "k");
        job.transition(JobStatus::Cancelled).unwrap();
        assert!(job.transition(JobStatus::Queued).is_err());
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        let p = RetryPolicy {
            max_attempts: 5,
            base_delay_ms: 500,
            max_delay_ms: 4000,
        };
        assert_eq!(p.backoff_ms(0), 0);
        assert_eq!(p.backoff_ms(1), 500);
        assert_eq!(p.backoff_ms(2), 1000);
        assert_eq!(p.backoff_ms(3), 2000);
        assert_eq!(p.backoff_ms(4), 4000);
        assert_eq!(p.backoff_ms(5), 4000); // capped
        assert_eq!(p.backoff_ms(100), 4000); // no overflow
    }

    #[test]
    fn should_retry_respects_max_attempts() {
        let p = RetryPolicy::default();
        assert!(p.should_retry(4));
        assert!(!p.should_retry(5));
        assert!(!p.should_retry(6));
    }

    #[test]
    fn spec_and_job_serde_roundtrip() {
        let job = Job::new(
            ConversionSpec {
                to: "png".into(),
                quality: 90,
                width: Some(100),
                height: None,
                strip_metadata: true,
            },
            "uploads/abc",
        );
        let json = serde_json::to_string(&job).unwrap();
        let back: Job = serde_json::from_str(&json).unwrap();
        assert_eq!(job, back);
    }
}
