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
