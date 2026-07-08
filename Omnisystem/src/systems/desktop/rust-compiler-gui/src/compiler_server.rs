use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum CompileProfile {
    Debug,
    Release,
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub success: bool,
    pub duration_ms: u128,
    pub errors: usize,
    pub warnings: usize,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct CompilerServer {
    project_root: PathBuf,
    profile: CompileProfile,
}

impl CompilerServer {
    pub fn new() -> Self {
        Self {
            project_root: std::env::current_dir().unwrap_or_default(),
            profile: CompileProfile::Debug,
        }
    }

    pub fn set_project_root(&mut self, root: PathBuf) {
        self.project_root = root;
    }

    pub fn set_profile(&mut self, profile: CompileProfile) {
        self.profile = profile;
    }

    pub async fn compile(&self) -> anyhow::Result<CompileResult> {
        let start = std::time::Instant::now();

        let profile_arg = match self.profile {
            CompileProfile::Release => "--release",
            CompileProfile::Debug => "",
        };

        let output = Command::new("cargo")
            .current_dir(&self.project_root)
            .arg("build")
            .arg(profile_arg)
            .output()?;

        let duration_ms = start.elapsed().as_millis();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined_output = format!("{}\n{}", stdout, stderr);

        // Count errors and warnings in output
        let errors = combined_output.matches("error").count();
        let warnings = combined_output.matches("warning").count();

        Ok(CompileResult {
            success: output.status.success(),
            duration_ms,
            errors,
            warnings,
            output: combined_output.to_string(),
        })
    }

    pub async fn get_ast(&self, _file_path: &Path) -> anyhow::Result<String> {
        Ok("AST information (not yet implemented)".to_string())
    }

    pub async fn get_type_info(&self, file: &Path, line: usize, col: usize) -> anyhow::Result<String> {
        Ok(format!("Type at {}:{}:{}", file.display(), line, col))
    }
}
