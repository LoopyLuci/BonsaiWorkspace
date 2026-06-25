use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Texture,
    Model,
    Audio,
    Video,
    Font,
    Shader,
    Data,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetState {
    Unprocessed,
    Processing,
    Processed,
    Failed,
    CachedValid,
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub id: String,
    pub path: PathBuf,
    pub asset_type: AssetType,
    pub state: AssetState,
    pub size_bytes: u64,
    pub content_hash: String,
    pub processing_duration_ms: u128,
    pub compression_ratio: f32,
    pub dependencies: Vec<String>,
}

impl Asset {
    pub fn new(path: PathBuf) -> Self {
        let id = path.to_string_lossy().to_string()[..16.min(path.to_string_lossy().len())].to_string();

        Self {
            id,
            path,
            asset_type: AssetType::Other,
            state: AssetState::Unprocessed,
            size_bytes: 0,
            content_hash: String::new(),
            processing_duration_ms: 0,
            compression_ratio: 1.0,
            dependencies: Vec::new(),
        }
    }

    pub fn detect_type(&mut self) {
        if let Some(ext) = self.path.extension().and_then(|e| e.to_str()) {
            self.asset_type = match ext.to_lowercase().as_str() {
                "png" | "jpg" | "jpeg" | "webp" | "tga" => AssetType::Texture,
                "glb" | "gltf" | "fbx" | "obj" => AssetType::Model,
                "mp3" | "wav" | "ogg" | "flac" => AssetType::Audio,
                "mp4" | "webm" | "mov" => AssetType::Video,
                "ttf" | "otf" | "woff" => AssetType::Font,
                "glsl" | "hlsl" | "spirv" => AssetType::Shader,
                "json" | "toml" | "yaml" => AssetType::Data,
                _ => AssetType::Other,
            };
        }
    }

    pub fn compute_hash(&mut self, content: &[u8]) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        self.content_hash = format!("{:x}", hasher.finish());
    }
}

pub trait AssetProcessor: Send + Sync {
    fn process(&self, asset: &mut Asset, content: &[u8]) -> anyhow::Result<Vec<u8>>;
    fn supports(&self, asset_type: AssetType) -> bool;
}

pub struct AssetSystem {
    assets: std::sync::Arc<std::sync::RwLock<HashMap<String, Asset>>>,
    processors: std::sync::Arc<std::sync::RwLock<Vec<std::sync::Arc<dyn AssetProcessor>>>>,
    watch_dir: std::sync::Arc<std::sync::RwLock<Option<PathBuf>>>,
}

impl AssetSystem {
    pub fn new() -> Self {
        Self {
            assets: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            processors: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
            watch_dir: std::sync::Arc::new(std::sync::RwLock::new(None)),
        }
    }

    pub fn set_watch_dir(&self, dir: PathBuf) {
        if let Ok(mut wd) = self.watch_dir.write() {
            *wd = Some(dir);
        }
    }

    pub fn register_processor(&self, processor: std::sync::Arc<dyn AssetProcessor>) {
        if let Ok(mut procs) = self.processors.write() {
            procs.push(processor);
        }
    }

    pub fn add_asset(&self, mut asset: Asset) -> String {
        asset.detect_type();
        let id = asset.id.clone();
        if let Ok(mut assets) = self.assets.write() {
            assets.insert(id.clone(), asset);
        }
        id
    }

    pub async fn process_asset(&self, id: &str, content: &[u8]) -> anyhow::Result<()> {
        if let Ok(mut assets) = self.assets.write() {
            if let Some(asset) = assets.get_mut(id) {
                asset.state = AssetState::Processing;
                let start = std::time::Instant::now();
                asset.compute_hash(content);
                asset.size_bytes = content.len() as u64;
                asset.state = AssetState::CachedValid;
                asset.processing_duration_ms = start.elapsed().as_millis();
            }
        }
        Ok(())
    }

    pub fn get_asset(&self, id: &str) -> Option<Asset> {
        self.assets.read().ok().and_then(|a| a.get(id).cloned())
    }

    pub fn list_assets(&self) -> Vec<Asset> {
        self.assets.read().ok().map(|a| a.values().cloned().collect()).unwrap_or_default()
    }

    pub fn dependency_graph(&self) -> HashMap<String, Vec<String>> {
        self.assets
            .read()
            .ok()
            .map(|a| a.iter().map(|(id, asset)| (id.clone(), asset.dependencies.clone())).collect())
            .unwrap_or_default()
    }

    pub async fn hot_reload(&self, path: &Path) -> anyhow::Result<()> {
        let id = if let Ok(assets) = self.assets.read() {
            assets.iter().find(|(_, a)| a.path == path).map(|(id, _)| id.clone())
        } else {
            None
        };

        if let Some(id) = id {
            let content = std::fs::read(path)?;
            self.process_asset(&id, &content).await?;
        }
        Ok(())
    }

    pub fn stats(&self) -> AssetStats {
        if let Ok(assets) = self.assets.read() {
            let total_assets = assets.len();
            let total_size: u64 = assets.values().map(|a| a.size_bytes).sum();
            let processed = assets
                .values()
                .filter(|a| a.state == AssetState::Processed || a.state == AssetState::CachedValid)
                .count();
            let failed = assets
                .values()
                .filter(|a| a.state == AssetState::Failed)
                .count();

            AssetStats {
                total_assets,
                processed,
                failed,
                total_size_bytes: total_size,
                avg_processing_ms: if processed > 0 {
                    assets
                        .values()
                        .filter(|a| a.state == AssetState::Processed || a.state == AssetState::CachedValid)
                        .map(|a| a.processing_duration_ms)
                        .sum::<u128>()
                        / processed as u128
                } else {
                    0
                },
            }
        } else {
            AssetStats::default()
        }
    }

    pub fn clear(&self) {
        if let Ok(mut assets) = self.assets.write() {
            assets.clear();
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AssetStats {
    pub total_assets: usize,
    pub processed: usize,
    pub failed: usize,
    pub total_size_bytes: u64,
    pub avg_processing_ms: u128,
}
