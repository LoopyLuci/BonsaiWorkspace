use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::FileOptions;

use crate::manifest::PackageManifest;
use crate::{PackageError, Result};

/// Builds a .bkp ZIP store-mode archive (no compression for random access).
pub struct PackageWriter {
    zip: ZipWriter<File>,
    manifest: PackageManifest,
    out_path: PathBuf,
}

impl PackageWriter {
    pub fn create(path: &Path, manifest: PackageManifest) -> Result<Self> {
        let file = File::create(path)?;
        Ok(PackageWriter {
            zip: ZipWriter::new(file),
            manifest,
            out_path: path.to_path_buf(),
        })
    }

    fn store_options() -> FileOptions<'static, ()> {
        FileOptions::default().compression_method(CompressionMethod::Stored)
    }

    /// Add a file from disk into the package at `entry_path` (the in-ZIP path).
    pub fn add_file(&mut self, src: &Path, entry_path: &str) -> Result<String> {
        let mut f = File::open(src)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;

        let hash = blake3::hash(&buf);
        let hex = hex::encode(hash.as_bytes());

        self.zip.start_file(entry_path, Self::store_options())?;
        self.zip.write_all(&buf)?;

        Ok(hex)
    }

    /// Add raw bytes into the package.
    pub fn add_bytes(&mut self, data: &[u8], entry_path: &str) -> Result<String> {
        let hash = blake3::hash(data);
        let hex = hex::encode(hash.as_bytes());

        self.zip.start_file(entry_path, Self::store_options())?;
        self.zip.write_all(data)?;

        Ok(hex)
    }

    /// Add a directory tree recursively.
    pub fn add_dir(&mut self, src_dir: &Path, prefix: &str) -> Result<()> {
        for entry in walkdir(src_dir)? {
            let rel = entry
                .strip_prefix(src_dir)
                .map_err(|e| PackageError::Invalid(e.to_string()))?;
            let entry_path = format!("{}/{}", prefix, rel.to_string_lossy().replace('\\', "/"));
            self.add_file(&entry, &entry_path)?;
        }
        Ok(())
    }

    /// Finalize: write manifest.json and seal the ZIP.
    pub fn finish(mut self) -> Result<PathBuf> {
        let manifest_json = serde_json::to_vec_pretty(&self.manifest)?;
        self.zip
            .start_file("manifest.json", Self::store_options())?;
        self.zip.write_all(&manifest_json)?;

        let provenance = serde_json::json!({
            "created_by": "bonsai-package",
            "created_at": self.manifest.created_at.to_rfc3339(),
            "bkp_version": self.manifest.bkp_version,
        });
        let prov_bytes = serde_json::to_vec_pretty(&provenance)?;
        self.zip
            .start_file("provenance.json", Self::store_options())?;
        self.zip.write_all(&prov_bytes)?;

        self.zip.finish()?;
        Ok(self.out_path)
    }
}

fn walkdir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walkdir(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
