use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferHeader {
    pub file_name: String,
    pub file_size: u64,
    pub chunk_size: u32,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub offset: u64,
    pub data: Vec<u8>,
}

pub struct FileTransferStream;

impl FileTransferStream {
    pub async fn send_file(
        writer: &mut (impl tokio::io::AsyncWriteExt + Unpin),
        path: &std::path::Path,
    ) -> Result<(), String> {
        let data = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let meta = tokio::fs::metadata(path).await.map_err(|e| e.to_string())?;
        let checksum = blake3::hash(&data).to_hex().to_string();

        let header = FileTransferHeader {
            file_name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_size: meta.len(),
            chunk_size: 65_536,
            checksum,
        };

        let hdr_bytes = serde_json::to_vec(&header).map_err(|e| e.to_string())?;
        writer
            .write_all(&(hdr_bytes.len() as u32).to_le_bytes())
            .await
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&hdr_bytes)
            .await
            .map_err(|e| e.to_string())?;

        let mut offset = 0u64;
        for chunk_data in data.chunks(65_536) {
            let chunk = FileChunk {
                offset,
                data: chunk_data.to_vec(),
            };
            let chunk_bytes = serde_json::to_vec(&chunk).map_err(|e| e.to_string())?;
            writer
                .write_all(&(chunk_bytes.len() as u32).to_le_bytes())
                .await
                .map_err(|e| e.to_string())?;
            writer
                .write_all(&chunk_bytes)
                .await
                .map_err(|e| e.to_string())?;
            offset += chunk_data.len() as u64;
        }
        Ok(())
    }

    pub async fn receive_file(
        reader: &mut (impl tokio::io::AsyncReadExt + Unpin),
        output_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let mut len_buf = [0u8; 4];
        reader
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| e.to_string())?;
        let hdr_len = u32::from_le_bytes(len_buf) as usize;
        let mut hdr_buf = vec![0u8; hdr_len];
        reader
            .read_exact(&mut hdr_buf)
            .await
            .map_err(|e| e.to_string())?;
        let header: FileTransferHeader =
            serde_json::from_slice(&hdr_buf).map_err(|e| e.to_string())?;

        let mut all_data = Vec::with_capacity(header.file_size as usize);
        while all_data.len() < header.file_size as usize {
            reader
                .read_exact(&mut len_buf)
                .await
                .map_err(|e| e.to_string())?;
            let chunk_len = u32::from_le_bytes(len_buf) as usize;
            let mut chunk_buf = vec![0u8; chunk_len];
            reader
                .read_exact(&mut chunk_buf)
                .await
                .map_err(|e| e.to_string())?;
            let chunk: FileChunk = serde_json::from_slice(&chunk_buf).map_err(|e| e.to_string())?;
            all_data.extend_from_slice(&chunk.data);
        }

        let actual = blake3::hash(&all_data).to_hex().to_string();
        if actual != header.checksum {
            return Err(format!(
                "checksum mismatch: expected {} got {}",
                header.checksum, actual
            ));
        }

        tokio::fs::create_dir_all(output_dir)
            .await
            .map_err(|e| e.to_string())?;
        let out_path = output_dir.join(&header.file_name);
        tokio::fs::write(&out_path, &all_data)
            .await
            .map_err(|e| e.to_string())?;
        Ok(out_path)
    }
}
