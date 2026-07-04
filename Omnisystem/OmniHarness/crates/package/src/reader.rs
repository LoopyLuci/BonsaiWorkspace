use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::manifest::PackageManifest;
use crate::{PackageError, Result};

/// Reads a .bkp ZIP archive.
pub struct PackageReader {
    archive: ZipArchive<File>,
    pub manifest: PackageManifest,
    pub path: PathBuf,
}

impl PackageReader {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        let manifest: PackageManifest = {
            let mut entry = archive
                .by_name("manifest.json")
                .map_err(|_| PackageError::NotFound("manifest.json".into()))?;
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            serde_json::from_str(&buf)?
        };

        Ok(PackageReader {
            archive,
            manifest,
            path: path.to_path_buf(),
        })
    }

    /// Read a single entry as bytes.
    pub fn read_entry(&mut self, name: &str) -> Result<Vec<u8>> {
        let mut entry = self
            .archive
            .by_name(name)
            .map_err(|_| PackageError::NotFound(name.into()))?;
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Extract a single entry to disk.
    pub fn extract_entry(&mut self, name: &str, dest: &Path) -> Result<()> {
        let data = self.read_entry(name)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, data)?;
        Ok(())
    }

    /// Extract all entries whose in-ZIP path starts with `prefix` into `dest_dir`.
    pub fn extract_prefix(&mut self, prefix: &str, dest_dir: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(dest_dir)?;

        let names: Vec<String> = (0..self.archive.len())
            .filter_map(|i| {
                self.archive.by_index(i).ok().and_then(|e| {
                    let n = e.name().to_owned();
                    if n.starts_with(prefix) { Some(n) } else { None }
                })
            })
            .collect();

        let mut extracted = Vec::new();
        for name in names {
            let rel = name
                .strip_prefix(prefix)
                .unwrap_or(&name)
                .trim_start_matches('/');
            let dest = dest_dir.join(rel);
            self.extract_entry(&name, &dest)?;
            extracted.push(dest);
        }
        Ok(extracted)
    }

    /// List all entry names in the archive.
    pub fn list_entries(&mut self) -> Result<Vec<String>> {
        let names = (0..self.archive.len())
            .map(|i| self.archive.by_index(i).map(|e| e.name().to_owned()))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(names)
    }

    /// Verify BLAKE3 hash of an entry.
    pub fn verify_entry(&mut self, name: &str, expected_blake3: &str) -> Result<bool> {
        let data = self.read_entry(name)?;
        let hash = blake3::hash(&data);
        let hex = hex::encode(hash.as_bytes());
        Ok(hex == expected_blake3)
    }
}
