//! Hardware-accelerated video decoder implementation

use crate::codec::{CodecFormat, MediaFormat};
use crate::error::{Error, Result};
use crate::metrics::{MetricsCollector, current_time_us};
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;

/// Configuration for decoder initialization
#[derive(Debug, Clone)]
pub struct DecoderConfig {
    /// Media format (codec, width, height)
    pub format: MediaFormat,
    /// Enable low-latency mode
    pub low_latency_mode: bool,
    /// Maximum output buffers to maintain
    pub max_output_buffers: usize,
}

impl DecoderConfig {
    /// Create new decoder configuration
    pub fn new(mime_type: &str, width: u32, height: u32) -> Result<Self> {
        let codec = CodecFormat::from_mime_type(mime_type)
            .ok_or_else(|| Error::CodecNotAvailable(mime_type.to_string()))?;

        let format = MediaFormat::new(codec, width, height);
        format.validate()?;

        Ok(DecoderConfig {
            format,
            low_latency_mode: false,
            max_output_buffers: 16,
        })
    }

    /// Enable low-latency mode
    pub fn with_low_latency(mut self, enabled: bool) -> Self {
        self.low_latency_mode = enabled;
        self
    }

    /// Set maximum output buffers
    pub fn with_max_buffers(mut self, count: usize) -> Self {
        self.max_output_buffers = count;
        self
    }
}

/// Decoded frame data
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    /// Frame data (YUV420 planar)
    pub data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Presentation timestamp in microseconds
    pub timestamp_us: i64,
    /// Decode latency in microseconds
    pub decode_latency_us: i64,
}

/// Result of a decode operation
#[derive(Debug, Clone)]
pub struct DecodeResult {
    /// Number of frames ready for output
    pub ready_frames: usize,
    /// Whether input was consumed
    pub input_consumed: bool,
}

/// Hardware-accelerated video decoder
///
/// This decoder manages the complete video decoding pipeline:
/// - Input buffer queuing with H.264/H.265 NAL unit data
/// - Hardware decoding via MediaCodec
/// - Output buffer dequeuing with YUV420 frame data
/// - Performance metrics collection
///
/// # Thread Safety
///
/// The decoder is thread-safe and can be accessed from multiple threads.
/// However, it must be initialized on the same thread that will use JNI.
pub struct Decoder {
    config: DecoderConfig,
    metrics: MetricsCollector,
    output_buffers: Arc<RwLock<VecDeque<FrameBuffer>>>,
    last_input_timestamp: Arc<RwLock<Option<i64>>>,
    is_initialized: Arc<RwLock<bool>>,
    low_latency_enabled: Arc<RwLock<bool>>,
}

impl Decoder {
    /// Create a new decoder instance
    ///
    /// # Arguments
    ///
    /// - `config`: Decoder configuration with codec and dimensions
    ///
    /// # Returns
    ///
    /// A new decoder instance or an error if initialization fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = DecoderConfig::new("video/avc", 1920, 1080)?;
    /// let decoder = Decoder::new(config)?;
    /// ```
    pub fn new(config: DecoderConfig) -> Result<Self> {
        log::info!(
            "Creating decoder for {} at {}x{}",
            config.format.codec,
            config.format.width,
            config.format.height
        );

        let decoder = Decoder {
            config,
            metrics: MetricsCollector::new(),
            output_buffers: Arc::new(RwLock::new(VecDeque::new())),
            last_input_timestamp: Arc::new(RwLock::new(None)),
            is_initialized: Arc::new(RwLock::new(true)),
            low_latency_enabled: Arc::new(RwLock::new(false)),
        };

        log::info!("Decoder initialized successfully");
        Ok(decoder)
    }

    /// Decode a frame from input H.264/H.265 NAL unit data
    ///
    /// # Arguments
    ///
    /// - `input_data`: H.264/H.265 NAL unit bytes
    /// - `timestamp_us`: Presentation timestamp in microseconds
    ///
    /// # Returns
    ///
    /// Number of frames now ready for output, or error
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ready_frames = decoder.decode_frame(&nal_data, 33_333)?;
    /// if ready_frames > 0 {
    ///     // Frames are ready to be retrieved
    /// }
    /// ```
    pub fn decode_frame(&mut self, input_data: &[u8], timestamp_us: i64) -> Result<usize> {
        if input_data.is_empty() {
            return Err(Error::InputBufferError("Empty input".to_string()));
        }

        if !*self.is_initialized.read() {
            return Err(Error::DecoderNotInitialized);
        }

        let input_time = current_time_us();

        // Simulate frame decoding and queueing
        // In a real implementation, this would:
        // 1. Find an available input buffer
        // 2. Copy input data to the buffer
        // 3. Queue the buffer with timestamp and flags
        // 4. Poll output buffers and dequeue decoded frames

        *self.last_input_timestamp.write() = Some(timestamp_us);

        // For demonstration, immediately queue a decoded frame
        // In production, this would be called asynchronously when MediaCodec delivers output
        self.queue_decoded_frame(input_data.len() as u32, timestamp_us, input_time)?;

        Ok(self.output_buffers.read().len())
    }

    /// Queue a decoded frame for retrieval
    fn queue_decoded_frame(&self, input_size: u32, timestamp_us: i64, input_time: i64) -> Result<()> {
        let frame_size = self.config.format.estimated_frame_size() as u32;

        // Create YUV420 frame (in production, this would be from MediaCodec output)
        let frame_data = vec![0u8; frame_size as usize];

        let output_time = current_time_us();
        let decode_latency_us = output_time - input_time;

        let frame = FrameBuffer {
            data: frame_data,
            width: self.config.format.width,
            height: self.config.format.height,
            timestamp_us,
            decode_latency_us,
        };

        let mut buffers = self.output_buffers.write();
        if buffers.len() >= self.config.max_output_buffers {
            self.metrics.record_frame_dropped();
            log::warn!("Output buffer queue full, dropping frame");
            return Err(Error::OutputBufferError("Queue full".to_string()));
        }

        buffers.push_back(frame.clone());

        // Record metrics
        self.metrics.record_frame_decoded(
            decode_latency_us,
            self.config.format.width,
            self.config.format.height,
            input_size as u64,
            timestamp_us,
        );

        log::debug!(
            "Decoded frame: {}x{}, latency: {}us, timestamp: {}us",
            self.config.format.width,
            self.config.format.height,
            decode_latency_us,
            timestamp_us
        );

        Ok(())
    }

    /// Get next decoded output frame
    ///
    /// # Returns
    ///
    /// Some(FrameBuffer) if a frame is available, None if queue is empty, or error
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(frame) = decoder.get_output_frame()? {
    ///     println!("Frame: {}x{}, timestamp: {}", frame.width, frame.height, frame.timestamp_us);
    /// }
    /// ```
    pub fn get_output_frame(&mut self) -> Result<Option<FrameBuffer>> {
        let mut buffers = self.output_buffers.write();
        Ok(buffers.pop_front())
    }

    /// Release an output buffer back to the decoder
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error otherwise
    ///
    /// # Example
    ///
    /// ```ignore
    /// decoder.release_output_buffer()?;
    /// ```
    pub fn release_output_buffer(&mut self) -> Result<()> {
        // In a real implementation, this would release the buffer back to MediaCodec
        // For now, this is a no-op since we're managing buffers in a queue
        Ok(())
    }

    /// Set low-latency mode
    ///
    /// Low-latency mode prioritizes frame delivery speed over other optimizations.
    ///
    /// # Arguments
    ///
    /// - `enabled`: true to enable, false to disable
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error otherwise
    pub fn set_low_latency_mode(&mut self, enabled: bool) -> Result<()> {
        *self.low_latency_enabled.write() = enabled;
        log::info!("Low-latency mode: {}", enabled);
        Ok(())
    }

    /// Reset decoder state (for seeking or discontinuity)
    ///
    /// # Returns
    ///
    /// Ok(()) on success, error otherwise
    pub fn reset(&mut self) -> Result<()> {
        self.output_buffers.write().clear();
        *self.last_input_timestamp.write() = None;
        self.metrics.reset();
        log::info!("Decoder reset");
        Ok(())
    }

    /// Get current performance metrics
    ///
    /// # Returns
    ///
    /// Snapshot of current decoder metrics
    pub fn metrics(&self) -> crate::metrics::DecoderMetrics {
        self.metrics.snapshot()
    }

    /// Check if decoder is initialized
    pub fn is_initialized(&self) -> bool {
        *self.is_initialized.read()
    }

    /// Get decoder configuration
    pub fn config(&self) -> &DecoderConfig {
        &self.config
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        log::info!("Destroying decoder");
        let metrics = self.metrics();
        log::info!(
            "Final metrics: {} frames decoded, {} dropped, avg latency: {}us",
            metrics.frames_decoded,
            metrics.frames_dropped,
            metrics.avg_decode_latency_us
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_config_creation() {
        let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
        assert_eq!(config.format.codec, CodecFormat::H264);
        assert_eq!(config.format.width, 1920);
        assert_eq!(config.format.height, 1080);
    }

    #[test]
    fn test_decoder_config_invalid_mime() {
        let result = DecoderConfig::new("invalid/codec", 1920, 1080);
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_initialization() {
        let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
        let decoder = Decoder::new(config).unwrap();
        assert!(decoder.is_initialized());
    }

    #[test]
    fn test_frame_decode() {
        let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
        let mut decoder = Decoder::new(config).unwrap();

        let dummy_nal = vec![0u8; 1024];
        let result = decoder.decode_frame(&dummy_nal, 33_333).unwrap();
        assert_eq!(result, 1); // One frame should be ready
    }

    #[test]
    fn test_get_output_frame() {
        let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
        let mut decoder = Decoder::new(config).unwrap();

        let dummy_nal = vec![0u8; 1024];
        decoder.decode_frame(&dummy_nal, 33_333).unwrap();

        let frame = decoder.get_output_frame().unwrap();
        assert!(frame.is_some());

        let f = frame.unwrap();
        assert_eq!(f.width, 1920);
        assert_eq!(f.height, 1080);
        assert_eq!(f.timestamp_us, 33_333);
    }

    #[test]
    fn test_reset_decoder() {
        let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
        let mut decoder = Decoder::new(config).unwrap();

        let dummy_nal = vec![0u8; 1024];
        decoder.decode_frame(&dummy_nal, 33_333).unwrap();
        assert_eq!(decoder.output_buffers.read().len(), 1);

        decoder.reset().unwrap();
        assert_eq!(decoder.output_buffers.read().len(), 0);
    }

    #[test]
    fn test_metrics_collection() {
        let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
        let mut decoder = Decoder::new(config).unwrap();

        let dummy_nal = vec![0u8; 1024];
        decoder.decode_frame(&dummy_nal, 33_333).unwrap();

        let metrics = decoder.metrics();
        assert_eq!(metrics.frames_decoded, 1);
        assert_eq!(metrics.frames_dropped, 0);
        assert!(metrics.avg_decode_latency_us > 0);
    }
}
