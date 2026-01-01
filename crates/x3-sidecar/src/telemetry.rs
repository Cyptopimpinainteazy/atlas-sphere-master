//! Telemetry and metrics for x3-sidecar

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Telemetry collector
pub struct Telemetry {
    /// Total jobs received
    pub jobs_received: AtomicU64,
    /// Total jobs completed
    pub jobs_completed: AtomicU64,
    /// Total jobs failed
    pub jobs_failed: AtomicU64,
    /// Total gas consumed
    pub gas_consumed: AtomicU64,
    /// Total receipts submitted
    pub receipts_submitted: AtomicU64,
    /// Total receipt submission failures
    pub receipt_failures: AtomicU64,
    /// Total execution time (microseconds)
    pub execution_time_us: AtomicU64,
    /// Start time
    start_time: Instant,
}

impl Telemetry {
    /// Create new telemetry instance
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            jobs_received: AtomicU64::new(0),
            jobs_completed: AtomicU64::new(0),
            jobs_failed: AtomicU64::new(0),
            gas_consumed: AtomicU64::new(0),
            receipts_submitted: AtomicU64::new(0),
            receipt_failures: AtomicU64::new(0),
            execution_time_us: AtomicU64::new(0),
            start_time: Instant::now(),
        })
    }

    /// Record a job received
    pub fn record_job_received(&self) {
        self.jobs_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a job completed
    pub fn record_job_completed(&self, gas_used: u64, execution_time: Duration) {
        self.jobs_completed.fetch_add(1, Ordering::Relaxed);
        self.gas_consumed.fetch_add(gas_used, Ordering::Relaxed);
        self.execution_time_us
            .fetch_add(execution_time.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record a job failed
    pub fn record_job_failed(&self) {
        self.jobs_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a receipt submitted
    pub fn record_receipt_submitted(&self) {
        self.receipts_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a receipt failure
    pub fn record_receipt_failure(&self) {
        self.receipt_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Get snapshot of metrics
    pub fn snapshot(&self) -> TelemetrySnapshot {
        let jobs_completed = self.jobs_completed.load(Ordering::Relaxed);
        let execution_time_us = self.execution_time_us.load(Ordering::Relaxed);

        TelemetrySnapshot {
            jobs_received: self.jobs_received.load(Ordering::Relaxed),
            jobs_completed,
            jobs_failed: self.jobs_failed.load(Ordering::Relaxed),
            gas_consumed: self.gas_consumed.load(Ordering::Relaxed),
            receipts_submitted: self.receipts_submitted.load(Ordering::Relaxed),
            receipt_failures: self.receipt_failures.load(Ordering::Relaxed),
            avg_execution_time_ms: if jobs_completed > 0 {
                (execution_time_us / jobs_completed) / 1000
            } else {
                0
            },
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Format as Prometheus metrics
    pub fn prometheus_format(&self) -> String {
        let snap = self.snapshot();

        format!(
            r#"# HELP x3_sidecar_jobs_received_total Total jobs received
# TYPE x3_sidecar_jobs_received_total counter
x3_sidecar_jobs_received_total {}

# HELP x3_sidecar_jobs_completed_total Total jobs completed successfully
# TYPE x3_sidecar_jobs_completed_total counter
x3_sidecar_jobs_completed_total {}

# HELP x3_sidecar_jobs_failed_total Total jobs failed
# TYPE x3_sidecar_jobs_failed_total counter
x3_sidecar_jobs_failed_total {}

# HELP x3_sidecar_gas_consumed_total Total gas consumed
# TYPE x3_sidecar_gas_consumed_total counter
x3_sidecar_gas_consumed_total {}

# HELP x3_sidecar_receipts_submitted_total Total receipts submitted to chain
# TYPE x3_sidecar_receipts_submitted_total counter
x3_sidecar_receipts_submitted_total {}

# HELP x3_sidecar_receipt_failures_total Total receipt submission failures
# TYPE x3_sidecar_receipt_failures_total counter
x3_sidecar_receipt_failures_total {}

# HELP x3_sidecar_avg_execution_time_ms Average job execution time in milliseconds
# TYPE x3_sidecar_avg_execution_time_ms gauge
x3_sidecar_avg_execution_time_ms {}

# HELP x3_sidecar_uptime_seconds Sidecar uptime in seconds
# TYPE x3_sidecar_uptime_seconds gauge
x3_sidecar_uptime_seconds {}
"#,
            snap.jobs_received,
            snap.jobs_completed,
            snap.jobs_failed,
            snap.gas_consumed,
            snap.receipts_submitted,
            snap.receipt_failures,
            snap.avg_execution_time_ms,
            snap.uptime_secs
        )
    }
}

/// Telemetry snapshot
#[derive(Debug, Clone)]
pub struct TelemetrySnapshot {
    pub jobs_received: u64,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub gas_consumed: u64,
    pub receipts_submitted: u64,
    pub receipt_failures: u64,
    pub avg_execution_time_ms: u64,
    pub uptime_secs: u64,
}

/// Execution timer for measuring job duration
pub struct ExecutionTimer {
    start: Instant,
    telemetry: Arc<Telemetry>,
    completed: bool,
}

impl ExecutionTimer {
    /// Start a new execution timer
    pub fn start(telemetry: Arc<Telemetry>) -> Self {
        telemetry.record_job_received();
        Self {
            start: Instant::now(),
            telemetry,
            completed: false,
        }
    }

    /// Mark execution as completed
    pub fn complete(mut self, gas_used: u64) {
        self.completed = true;
        self.telemetry
            .record_job_completed(gas_used, self.start.elapsed());
    }

    /// Mark execution as failed
    pub fn fail(mut self) {
        self.completed = true;
        self.telemetry.record_job_failed();
    }
}

impl Drop for ExecutionTimer {
    fn drop(&mut self) {
        if !self.completed {
            // Job was cancelled or panicked
            self.telemetry.record_job_failed();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_basic() {
        let telemetry = Telemetry::new();

        telemetry.record_job_received();
        telemetry.record_job_received();
        telemetry.record_job_completed(1000, Duration::from_millis(50));
        telemetry.record_job_failed();

        let snap = telemetry.snapshot();
        assert_eq!(snap.jobs_received, 2);
        assert_eq!(snap.jobs_completed, 1);
        assert_eq!(snap.jobs_failed, 1);
        assert_eq!(snap.gas_consumed, 1000);
    }

    #[test]
    fn test_execution_timer() {
        let telemetry = Telemetry::new();

        {
            let timer = ExecutionTimer::start(telemetry.clone());
            timer.complete(500);
        }

        let snap = telemetry.snapshot();
        assert_eq!(snap.jobs_received, 1);
        assert_eq!(snap.jobs_completed, 1);
        assert_eq!(snap.gas_consumed, 500);
    }

    #[test]
    fn test_prometheus_format() {
        let telemetry = Telemetry::new();
        telemetry.record_job_completed(1000, Duration::from_millis(100));

        let output = telemetry.prometheus_format();
        assert!(output.contains("x3_sidecar_jobs_completed_total 1"));
        assert!(output.contains("x3_sidecar_gas_consumed_total 1000"));
    }
}
