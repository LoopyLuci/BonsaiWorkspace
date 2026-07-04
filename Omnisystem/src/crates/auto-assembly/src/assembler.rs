//! Binary assembler module

use crate::Result;
use std::fs;
use std::path::Path;

pub struct BinaryAssembler {
    target_arch: String,
}

impl BinaryAssembler {
    pub fn new(target_arch: &str) -> Result<Self> {
        Ok(Self {
            target_arch: target_arch.to_string(),
        })
    }

    /// Validate object files
    pub fn validate_object_files(&self, files: &[String]) -> Result<()> {
        for file in files {
            if !Path::new(file).exists() {
                return Err(anyhow::anyhow!("Object file not found: {}", file));
            }

            // Verify ELF/Mach-O/PE header
            self.validate_object_format(file)?;
        }

        Ok(())
    }

    /// Validate object file format
    fn validate_object_format(&self, path: &str) -> Result<()> {
        let data = fs::read(path)?;

        if data.len() < 4 {
            return Err(anyhow::anyhow!("Invalid object file: too small"));
        }

        // Check magic bytes for ELF
        if &data[0..4] == b"\x7fELF" {
            return Ok(());
        }

        // Check for Mach-O
        if (data[0..4] == [0xfe, 0xed, 0xfa, 0xce]) ||
           (data[0..4] == [0xce, 0xfa, 0xed, 0xfe]) ||
           (data[0..4] == [0xcf, 0xfa, 0xed, 0xfe]) {
            return Ok(());
        }

        // Check for PE
        if &data[0..2] == b"MZ" {
            return Ok(());
        }

        Err(anyhow::anyhow!("Unsupported object file format: {}", path))
    }

    /// Assemble raw bytes into object file
    pub fn assemble_bytes(&self, data: &[u8], output_path: &str) -> Result<()> {
        fs::write(output_path, data)?;
        Ok(())
    }

    /// Get target architecture
    pub fn target_arch(&self) -> &str {
        &self.target_arch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assembler_creation() {
        let assembler = BinaryAssembler::new("x86_64");
        assert!(assembler.is_ok());
    }
}
