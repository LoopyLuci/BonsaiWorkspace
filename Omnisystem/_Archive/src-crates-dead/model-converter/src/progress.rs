//! Progress reporting for long-running conversion operations

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Progress update for a conversion operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionProgress {
    /// Operation identifier
    pub operation_id: String,

    /// Current stage (e.g., "reading", "converting", "writing", "verifying")
    pub stage: String,

    /// Current progress as percentage (0-100)
    pub percent: u32,

    /// Bytes processed so far
    pub bytes_processed: u64,

    /// Total bytes to process (0 if unknown)
    pub total_bytes: u64,

    /// Human-readable status message
    pub message: String,

    /// Elapsed time in seconds
    pub elapsed_secs: u64,

    /// Estimated remaining time in seconds (0 if unknown)
    pub eta_secs: u64,

    /// Current throughput in MB/s
    pub throughput_mbps: f64,
}

impl ConversionProgress {
    pub fn new(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            stage: "initializing".to_string(),
            percent: 0,
            bytes_processed: 0,
            total_bytes: 0,
            message: String::new(),
            elapsed_secs: 0,
            eta_secs: 0,
            throughput_mbps: 0.0,
        }
    }

    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.stage = stage.into();
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn update_progress(
        mut self,
        bytes_processed: u64,
        total_bytes: u64,
        elapsed_secs: u64,
    ) -> Self {
        self.bytes_processed = bytes_processed;
        self.total_bytes = total_bytes;
        self.elapsed_secs = elapsed_secs;

        if total_bytes > 0 {
            self.percent = ((bytes_processed as f64 / total_bytes as f64) * 100.0) as u32;
            if elapsed_secs > 0 {
                let throughput = (bytes_processed as f64 / 1_000_000.0) / (elapsed_secs as f64);
                self.throughput_mbps = throughput;
                if throughput > 0.0 {
                    let remaining_bytes = total_bytes - bytes_processed;
                    self.eta_secs = (remaining_bytes as f64 / (throughput * 1_000_000.0)) as u64;
                }
            }
        }

        self
    }
}

/// Reporter for conversion progress updates
pub struct ProgressReporter {
    tx: mpsc::UnboundedSender<ConversionProgress>,
    operation_id: String,
}

impl ProgressReporter {
    /// Create a new progress reporter with an unbounded channel
    pub fn new(operation_id: impl Into<String>) -> (Self, mpsc::UnboundedReceiver<ConversionProgress>) {
        let operation_id = operation_id.into();
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                tx,
                operation_id,
            },
            rx,
        )
    }

    /// Report progress update
    pub fn report(&self, progress: ConversionProgress) -> crate::error::ConverterResult<()> {
        self.tx
            .send(progress)
            .map_err(|e| crate::error::ConverterError::ChannelError(e.to_string()))
    }

    /// Report with stage and message
    pub fn update(
        &self,
        stage: impl Into<String>,
        message: impl Into<String>,
        bytes_processed: u64,
        total_bytes: u64,
        elapsed_secs: u64,
    ) -> crate::error::ConverterResult<()> {
        let progress = ConversionProgress::new(self.operation_id.clone())
            .with_stage(stage)
            .with_message(message)
            .update_progress(bytes_processed, total_bytes, elapsed_secs);
        self.report(progress)
    }

    /// Report completion
    pub fn complete(&self) -> crate::error::ConverterResult<()> {
        let progress = ConversionProgress::new(self.operation_id.clone())
            .with_stage("complete")
            .with_message("Conversion completed successfully");
        self.report(progress)
    }

    /// Report an error
    pub fn error(&self, message: impl Into<String>) -> crate::error::ConverterResult<()> {
        let progress = ConversionProgress::new(self.operation_id.clone())
            .with_stage("error")
            .with_message(message);
        self.report(progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_calculation() {
        let progress = ConversionProgress::new("test")
            .update_progress(50_000_000, 100_000_000, 10);
        assert_eq!(progress.percent, 50);
        assert!(progress.throughput_mbps > 0.0);
    }

    #[tokio::test]
    async fn test_progress_reporter() {
        let (reporter, mut rx) = ProgressReporter::new("test-op");

        let progress = ConversionProgress::new("test-op")
            .with_stage("reading")
            .with_message("Reading model...");

        reporter.report(progress.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.operation_id, "test-op");
        assert_eq!(received.stage, "reading");
    }
}
