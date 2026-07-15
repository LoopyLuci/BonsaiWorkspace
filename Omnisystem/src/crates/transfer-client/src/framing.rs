use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Maximum frame size: 256 MiB
const MAX_FRAME_SIZE: usize = 256 * 1024 * 1024;

/// Length‑delimited async frame I/O.
/// Wire format: 4‑byte big‑endian length prefix + payload bytes.
#[allow(dead_code)]
pub struct FrameCodec {
    read_buffer: BytesMut,
    write_buffer: Arc<Mutex<BytesMut>>,
}

impl FrameCodec {
    pub fn new() -> Self {
        Self {
            read_buffer: BytesMut::with_capacity(64 * 1024),
            write_buffer: Arc::new(Mutex::new(BytesMut::with_capacity(64 * 1024))),
        }
    }

    /// Read a complete frame from the stream.
    pub async fn read_frame<R: AsyncReadExt + Unpin>(
        &mut self,
        reader: &mut R,
    ) -> Result<Vec<u8>, std::io::Error> {
        loop {
            // Try to decode a frame from the existing buffer
            if self.read_buffer.len() >= 4 {
                let len = u32::from_be_bytes([
                    self.read_buffer[0],
                    self.read_buffer[1],
                    self.read_buffer[2],
                    self.read_buffer[3],
                ]) as usize;

                if len > MAX_FRAME_SIZE {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("frame too large: {} bytes (max {})", len, MAX_FRAME_SIZE),
                    ));
                }

                if self.read_buffer.len() >= 4 + len {
                    let _header = self.read_buffer.split_to(4);
                    let payload = self.read_buffer.split_to(len).to_vec();
                    return Ok(payload);
                }
            }

            // Need more data
            let mut tmp = [0u8; 8192];
            let n = reader.read(&mut tmp).await?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "connection closed",
                ));
            }
            self.read_buffer.extend_from_slice(&tmp[..n]);
        }
    }

    /// Write a frame to the stream.
    pub async fn write_frame<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        payload: &[u8],
    ) -> Result<(), std::io::Error> {
        let len = payload.len();
        if len > MAX_FRAME_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "payload too large",
            ));
        }

        let header = (len as u32).to_be_bytes();
        writer.write_all(&header).await?;
        writer.write_all(payload).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Perform a full JSON handshake: send request, receive response.
    pub async fn exchange_json<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
        request: &serde_json::Value,
    ) -> Result<serde_json::Value, std::io::Error> {
        let req_bytes = serde_json::to_vec(request)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.write_frame(writer, &req_bytes).await?;
        let resp_bytes = self.read_frame(reader).await?;
        serde_json::from_slice(&resp_bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn write_then_read_round_trips() {
        let codec = FrameCodec::new();
        let mut buf = Vec::new();
        codec.write_frame(&mut buf, b"hello world").await.unwrap();

        let mut cursor = Cursor::new(buf);
        let mut read_codec = FrameCodec::new();
        let frame = read_codec.read_frame(&mut cursor).await.unwrap();
        assert_eq!(frame, b"hello world");
    }

    #[tokio::test]
    async fn read_frame_handles_partial_reads() {
        // Simulate a stream that delivers the frame across multiple small
        // chunks rather than all at once, which is the whole reason
        // FrameCodec buffers instead of doing a single read_exact.
        let mut codec = FrameCodec::new();
        let mut full = Vec::new();
        FrameCodec::new().write_frame(&mut full, b"chunked-payload").await.unwrap();

        // A reader that yields 3 bytes at a time.
        struct ChunkedReader {
            data: Vec<u8>,
            pos: usize,
        }
        impl tokio::io::AsyncRead for ChunkedReader {
            fn poll_read(
                mut self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                let remaining = &self.data[self.pos..];
                let n = remaining.len().min(3);
                buf.put_slice(&remaining[..n]);
                self.pos += n;
                std::task::Poll::Ready(Ok(()))
            }
        }
        let mut reader = ChunkedReader { data: full, pos: 0 };

        let frame = codec.read_frame(&mut reader).await.unwrap();
        assert_eq!(frame, b"chunked-payload");
    }

    #[tokio::test]
    async fn read_frame_rejects_oversized_length_prefix() {
        let mut codec = FrameCodec::new();
        let mut buf = Vec::new();
        // A length prefix bigger than MAX_FRAME_SIZE.
        buf.extend_from_slice(&(MAX_FRAME_SIZE as u32 + 1).to_be_bytes());
        let mut cursor = Cursor::new(buf);

        let err = codec.read_frame(&mut cursor).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn exchange_json_round_trips_through_a_pipe() {
        let (mut client_reader, mut server_writer) = tokio::io::duplex(1024);
        let (mut server_reader, mut client_writer) = tokio::io::duplex(1024);

        let server = tokio::spawn(async move {
            let mut codec = FrameCodec::new();
            let req = codec.read_frame(&mut server_reader).await.unwrap();
            let req: serde_json::Value = serde_json::from_slice(&req).unwrap();
            assert_eq!(req["ping"], true);
            let resp = serde_json::json!({"pong": true});
            codec.write_frame(&mut server_writer, &serde_json::to_vec(&resp).unwrap()).await.unwrap();
        });

        let mut client_codec = FrameCodec::new();
        let response = client_codec
            .exchange_json(&mut client_reader, &mut client_writer, &serde_json::json!({"ping": true}))
            .await
            .unwrap();

        server.await.unwrap();
        assert_eq!(response["pong"], true);
    }
}
