use crate::{Result, RepositoryError};
use std::path::PathBuf;
use tokio::fs;

pub struct LocalLoader {
    cache_dir: PathBuf,
}

impl LocalLoader {
    pub fn new(cache_dir: PathBuf) -> Self {
        LocalLoader { cache_dir }
    }

    pub async fn load(&self, path: &PathBuf) -> Result<Vec<u8>> {
        tracing::info!("Loading package from: {:?}", path);

        if !path.exists() {
            return Err(RepositoryError::NotFound(format!("{:?}", path)));
        }

        fs::read(path)
            .await
            .map_err(|e| RepositoryError::IoError(e))
    }

    pub async fn save(&self, filename: &str, data: &[u8]) -> Result<PathBuf> {
        tracing::info!("Saving package: {}", filename);

        fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| RepositoryError::IoError(e))?;

        let path = self.cache_dir.join(filename);

        fs::write(&path, data)
            .await
            .map_err(|e| RepositoryError::IoError(e))?;

        Ok(path)
    }

    pub async fn list_cached(&self) -> Result<Vec<PathBuf>> {
        tracing::info!("Listing cached packages");

        if !self.cache_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&self.cache_dir)
            .await
            .map_err(|e| RepositoryError::IoError(e))?;

        let mut files = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| RepositoryError::IoError(e))? {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }

        Ok(files)
    }

    pub async fn clear_cache(&self) -> Result<()> {
        tracing::info!("Clearing cache");

        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .await
                .map_err(|e| RepositoryError::IoError(e))?;
        }

        Ok(())
    }

    pub async fn get_cache_size(&self) -> Result<u64> {
        let mut total_size = 0u64;

        for path in self.list_cached().await? {
            if let Ok(metadata) = fs::metadata(&path).await {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_loader_creation() {
        let temp_dir = TempDir::new().unwrap();
        let loader = LocalLoader::new(temp_dir.path().to_path_buf());
        assert_eq!(loader.cache_dir, temp_dir.path());
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let loader = LocalLoader::new(temp_dir.path().to_path_buf());

        let data = b"test data";
        let path = loader.save("test.txt", data).await.unwrap();

        let loaded = loader.load(&path).await.unwrap();
        assert_eq!(loaded, data);
    }

    #[tokio::test]
    async fn test_list_cached() {
        let temp_dir = TempDir::new().unwrap();
        let loader = LocalLoader::new(temp_dir.path().to_path_buf());

        loader.save("file1.txt", b"data1").await.unwrap();
        loader.save("file2.txt", b"data2").await.unwrap();

        let cached = loader.list_cached().await.unwrap();
        assert_eq!(cached.len(), 2);
    }
}
