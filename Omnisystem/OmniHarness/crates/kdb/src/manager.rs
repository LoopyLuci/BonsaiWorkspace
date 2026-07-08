//! High-level KDB Manager for module lifecycle management.

use crate::module::ModuleManifest;
use crate::retriever::KdbRetriever;
use crate::store::KdbStore;
use crate::{KdbError, Result};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zip::ZipArchive;

/// High-level knowledge database manager.
pub struct KdbManager {
    retriever: KdbRetriever,
    store: KdbStore,
    base_dir: PathBuf,
}

impl KdbManager {
    /// Create a new KDB manager with given dimensions and top-k for retrieval.
    pub fn new(base_dir: &Path, dim: usize, top_k: usize) -> Result<Self> {
        let store = KdbStore::open(base_dir)?;
        let retriever = KdbRetriever::new(dim, top_k);

        Ok(KdbManager {
            retriever,
            store,
            base_dir: base_dir.to_path_buf(),
        })
    }

    /// Load a module from a .kmod ZIP file.
    pub fn load_module(&self, module_path: &Path) -> Result<()> {
        if !module_path.exists() {
            return Err(KdbError::NotFound(module_path.display().to_string()));
        }

        let file = fs::File::open(module_path)?;
        let mut archive = ZipArchive::new(file)?;

        // Extract manifest
        let manifest_json = {
            let mut manifest_file = archive
                .by_name("manifest.json")
                .map_err(|_| KdbError::Invalid("missing manifest.json in module".into()))?;

            let mut manifest_json = String::new();
            use std::io::Read;
            manifest_file.read_to_string(&mut manifest_json)?;
            manifest_json
        };

        let manifest: ModuleManifest = serde_json::from_str(&manifest_json)?;

        // Create extraction directory
        let module_dir = self.store.module_dir(&manifest.name);
        fs::create_dir_all(&module_dir)?;

        // Extract all files from ZIP to the module directory
        let file_count = archive.len();
        for i in 0..file_count {
            let mut file = archive.by_index(i)?;
            let outpath = module_dir.join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)?;
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        // Register in store
        self.store.register_module(&manifest, &module_dir)?;

        // Load into retriever
        self.retriever.load_module(&manifest.name, &module_dir)?;

        Ok(())
    }

    /// Unload a module from memory and optionally remove it from disk.
    pub fn unload_module(&self, name: &str, remove_disk: bool) -> Result<()> {
        self.retriever.unload_module(name);
        self.store.unregister_module(name)?;

        if remove_disk {
            let module_dir = self.store.module_dir(name);
            if module_dir.exists() {
                fs::remove_dir_all(&module_dir)?;
            }
        }

        Ok(())
    }

    /// Reload a module without interrupting inference.
    pub fn reload_module(&self, name: &str, module_path: &Path) -> Result<()> {
        // Temporarily unload from memory but keep on disk
        self.retriever.unload_module(name);

        // Load the new module
        self.load_module(module_path)?;

        Ok(())
    }

    /// Search for top-k nearest neighbors for a query vector.
    pub fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(String, String, f32)>> {
        let retriever = KdbRetriever::new(query.len(), top_k);
        let contexts = retriever.retrieve(query)?;

        let mut results = Vec::new();
        for ctx in contexts {
            for (content, distance) in ctx.entries {
                results.push((ctx.module_name.clone(), content, distance));
            }
        }

        // Sort by distance and limit to top_k
        results.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);

        Ok(results)
    }

    /// Create a new knowledge module from a dataset.
    ///
    /// # Arguments
    ///
    /// * `dataset_path` - Path to JSONL file with { "id", "content", "embedding" } objects
    /// * `module_name` - Name for the new module
    /// * `output_dir` - Where to save the .kmod file
    /// * `embedding_fn` - Function to generate embeddings for content strings
    pub async fn create_module_from_dataset<F>(
        &self,
        dataset_path: &Path,
        module_name: &str,
        output_dir: &Path,
        embedding_fn: F,
    ) -> Result<PathBuf>
    where
        F: Fn(&str) -> Result<Vec<f32>>,
    {
        let dataset_content = fs::read_to_string(dataset_path)?;
        let lines: Vec<&str> = dataset_content.lines().collect();

        let mut embeddings: Vec<Vec<f32>> = Vec::new();
        let mut chunks = Vec::new();

        for line in lines {
            let obj: serde_json::Value = serde_json::from_str(line)?;
            let content = obj
                .get("content")
                .ok_or_else(|| KdbError::Invalid("missing 'content' field".into()))?
                .as_str()
                .ok_or_else(|| KdbError::Invalid("'content' must be a string".into()))?;

            // Get or generate embedding
            let embedding = if let Some(emb) = obj.get("embedding").and_then(|e| e.as_array()) {
                emb.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect()
            } else {
                embedding_fn(content)?
            };

            chunks.push(content.to_owned());
            embeddings.push(embedding);
        }

        if chunks.is_empty() {
            return Err(KdbError::Invalid("no chunks in dataset".into()));
        }

        let dim = embeddings[0].len();

        // Create HNSW index (m=16, ef_construction=200, cosine distance)
        let mut index = hnsw::HnswIndex::new(dim, 16, 200, hnsw::Distance::Cosine);
        for emb in embeddings.iter() {
            index.insert(emb.clone())?;
        }

        // Create module directory
        fs::create_dir_all(output_dir)?;

        // Save index
        let index_path = output_dir.join("index.hnsw");
        index.save(&index_path)?;

        // Save chunks
        let chunks_path = output_dir.join("values.txt");
        let chunks_content = chunks.join("\n");
        fs::write(&chunks_path, chunks_content)?;

        // Compute hashes
        let index_bytes = fs::read(&index_path)?;
        let blake3_index = blake3::hash(&index_bytes).to_hex().to_string();
        let chunks_bytes = chunks.join("\n");
        let blake3_chunks = blake3::hash(chunks_bytes.as_bytes()).to_hex().to_string();

        // Create manifest
        let manifest = ModuleManifest {
            id: Uuid::new_v4(),
            name: module_name.to_owned(),
            version: "1.0.0".to_owned(),
            domain: "knowledge".to_owned(),
            description: format!("Auto-generated module from {}", dataset_path.display()),
            dim,
            entry_count: chunks.len(),
            distance: hnsw::Distance::Cosine,
            created_at: Utc::now(),
            blake3_index,
            blake3_values: blake3_chunks,
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        let manifest_path = output_dir.join("manifest.json");
        fs::write(&manifest_path, manifest_json)?;

        // Create .kmod ZIP file
        let kmod_path = output_dir.parent().unwrap_or_else(|| Path::new(".")).join(format!(
            "{}.kmod",
            module_name
        ));
        self.create_kmod_zip(&kmod_path, output_dir)?;

        Ok(kmod_path)
    }

    /// Create a .kmod ZIP archive from a module directory.
    fn create_kmod_zip(&self, zip_path: &Path, module_dir: &Path) -> Result<()> {
        let file = fs::File::create(zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        for entry in fs::read_dir(module_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path
                .file_name()
                .ok_or_else(|| KdbError::Invalid("invalid file name".into()))?
                .to_string_lossy()
                .into_owned();

            if path.is_file() {
                let content = fs::read(&path)?;
                zip.start_file(&file_name, Default::default())?;
                use std::io::Write;
                zip.write_all(&content)?;
            }
        }

        zip.finish()?;
        Ok(())
    }

    /// List all loaded modules.
    pub fn list_loaded_modules(&self) -> Vec<crate::ModuleInfo> {
        self.retriever.list_modules()
    }

    /// Check if a module is loaded.
    pub fn is_module_loaded(&self, name: &str) -> bool {
        self.retriever
            .list_modules()
            .iter()
            .any(|m| m.name == name)
    }
}
