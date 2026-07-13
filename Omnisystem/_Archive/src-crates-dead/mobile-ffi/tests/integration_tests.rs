use mobile_ffi::{Decoder, DecoderConfig, CodecFormat, MediaFormat};

#[test]
fn test_h264_decoder_creation() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    assert_eq!(config.format.codec, CodecFormat::H264);
    assert_eq!(config.format.width, 1920);
    assert_eq!(config.format.height, 1080);

    let decoder = Decoder::new(config).unwrap();
    assert!(decoder.is_initialized());
}

#[test]
fn test_h265_decoder_creation() {
    let config = DecoderConfig::new("video/hevc", 1920, 1080).unwrap();
    assert_eq!(config.format.codec, CodecFormat::H265);

    let decoder = Decoder::new(config).unwrap();
    assert!(decoder.is_initialized());
}

#[test]
fn test_invalid_codec() {
    let result = DecoderConfig::new("video/invalid", 1920, 1080);
    assert!(result.is_err());
}

#[test]
fn test_decode_single_frame() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0x00, 0x00, 0x00, 0x01, 0x67]; // NAL header
    let ready_count = decoder.decode_frame(&dummy_nal, 33_333).unwrap();
    assert!(ready_count > 0);

    let frame = decoder.get_output_frame().unwrap();
    assert!(frame.is_some());

    let f = frame.unwrap();
    assert_eq!(f.width, 1920);
    assert_eq!(f.height, 1080);
    assert_eq!(f.timestamp_us, 33_333);
}

#[test]
fn test_decode_multiple_frames() {
    let config = DecoderConfig::new("video/avc", 1280, 720).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    for i in 0..10 {
        let timestamp = (i as i64) * 33_333;
        let dummy_nal = vec![0u8; 1024];
        let result = decoder.decode_frame(&dummy_nal, timestamp).unwrap();
        assert!(result > 0);
    }

    let metrics = decoder.metrics();
    assert_eq!(metrics.frames_decoded, 10);
    assert_eq!(metrics.frames_dropped, 0);
}

#[test]
fn test_frame_buffer_dequeue() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0u8; 2048];
    decoder.decode_frame(&dummy_nal, 33_333).unwrap();

    // Dequeue first frame
    let frame1 = decoder.get_output_frame().unwrap();
    assert!(frame1.is_some());

    // Queue should be empty now
    let frame2 = decoder.get_output_frame().unwrap();
    assert!(frame2.is_none());
}

#[test]
fn test_release_buffer() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0u8; 2048];
    decoder.decode_frame(&dummy_nal, 33_333).unwrap();

    let frame = decoder.get_output_frame().unwrap();
    assert!(frame.is_some());

    let result = decoder.release_output_buffer();
    assert!(result.is_ok());
}

#[test]
fn test_decoder_reset() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0u8; 2048];
    decoder.decode_frame(&dummy_nal, 33_333).unwrap();
    decoder.decode_frame(&dummy_nal, 66_666).unwrap();

    let metrics_before = decoder.metrics();
    assert_eq!(metrics_before.frames_decoded, 2);

    decoder.reset().unwrap();

    let metrics_after = decoder.metrics();
    assert_eq!(metrics_after.frames_decoded, 0);
}

#[test]
fn test_low_latency_mode() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let result = decoder.set_low_latency_mode(true);
    assert!(result.is_ok());

    let result = decoder.set_low_latency_mode(false);
    assert!(result.is_ok());
}

#[test]
fn test_metrics_collection() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0u8; 2048];

    for i in 0..5 {
        let timestamp = (i as i64) * 33_333;
        decoder.decode_frame(&dummy_nal, timestamp).unwrap();
    }

    let metrics = decoder.metrics();
    assert_eq!(metrics.frames_decoded, 5);
    assert_eq!(metrics.frames_dropped, 0);
    assert!(metrics.avg_decode_latency_us > 0);
    assert!(metrics.max_decode_latency_us > 0);
    assert!(metrics.total_bytes > 0);
    assert_eq!(metrics.last_timestamp_us, Some(4 * 33_333));
}

#[test]
fn test_frame_size_calculation() {
    // 1920x1080 YUV420
    let fmt = MediaFormat::new(CodecFormat::H264, 1920, 1080);
    assert_eq!(fmt.estimated_frame_size(), 1920 * 1080 * 3 / 2);

    // 1280x720 YUV420
    let fmt = MediaFormat::new(CodecFormat::H264, 1280, 720);
    assert_eq!(fmt.estimated_frame_size(), 1280 * 720 * 3 / 2);
}

#[test]
fn test_invalid_dimensions() {
    let fmt = MediaFormat::new(CodecFormat::H264, 1919, 1080);
    assert!(fmt.validate().is_err());

    let fmt = MediaFormat::new(CodecFormat::H264, 1920, 1081);
    assert!(fmt.validate().is_err());

    let fmt = MediaFormat::new(CodecFormat::H264, 0, 0);
    assert!(fmt.validate().is_err());
}

#[test]
fn test_60fps_sustained() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0u8; 2048];

    // Simulate 60 FPS for 1 second
    for i in 0..60 {
        let timestamp = (i as i64) * 16_667; // 1000000 / 60
        let result = decoder.decode_frame(&dummy_nal, timestamp);
        assert!(result.is_ok());
    }

    let metrics = decoder.metrics();
    assert_eq!(metrics.frames_decoded, 60);

    let fps = metrics.fps();
    // Should be approximately 60 FPS
    assert!(fps > 50.0 && fps < 70.0);
}

#[test]
fn test_4k_decoding() {
    let config = DecoderConfig::new("video/avc", 3840, 2160).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0u8; 8192];
    let result = decoder.decode_frame(&dummy_nal, 0);
    assert!(result.is_ok());

    let metrics = decoder.metrics();
    assert_eq!(metrics.frames_decoded, 1);
    assert_eq!(metrics.last_width, Some(3840));
    assert_eq!(metrics.last_height, Some(2160));
}

#[test]
fn test_empty_input_error() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let result = decoder.decode_frame(&[], 0);
    assert!(result.is_err());
}

#[test]
fn test_throughput_calculation() {
    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0u8; 2048];

    for i in 0..60 {
        let timestamp = (i as i64) * 16_667;
        decoder.decode_frame(&dummy_nal, timestamp).unwrap();
    }

    let metrics = decoder.metrics();
    let throughput_mbps = metrics.throughput_mbps();

    // With 60 frames * 2048 bytes at 16.667ms per frame
    // Should be approximately 60 * 2048 * 8 / 1_000_000 * 1e6 / 1_000_000 Mbps
    assert!(throughput_mbps > 0.0);
}

#[test]
fn test_frame_drops_tracking() {
    let config = DecoderConfig::new("video/avc", 1920, 1080)
        .unwrap()
        .with_max_buffers(2); // Small buffer to force drops

    let mut decoder = Decoder::new(config).unwrap();

    let dummy_nal = vec![0u8; 2048];

    // Try to queue more frames than buffer allows
    for i in 0..10 {
        let timestamp = (i as i64) * 33_333;
        let _ = decoder.decode_frame(&dummy_nal, timestamp);
    }

    let metrics = decoder.metrics();
    assert!(metrics.frames_decoded > 0);
    // Some frames may have been dropped due to small buffer
}

#[test]
fn test_concurrent_metrics_access() {
    use std::thread;
    use std::sync::Arc;
    use std::sync::Mutex;

    let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
    let decoder = Arc::new(Mutex::new(Decoder::new(config).unwrap()));

    let dummy_nal = vec![0u8; 2048];

    // Simulate concurrent access
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let decoder_clone = Arc::clone(&decoder);
            thread::spawn(move || {
                let mut d = decoder_clone.lock().unwrap();
                let timestamp = (i as i64) * 33_333;
                let result = d.decode_frame(&dummy_nal, timestamp);
                assert!(result.is_ok());
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let decoder = decoder.lock().unwrap();
    let metrics = decoder.metrics();
    assert!(metrics.frames_decoded > 0);
}
