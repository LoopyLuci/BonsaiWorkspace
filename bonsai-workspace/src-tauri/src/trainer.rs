use std::path::PathBuf;

pub struct Trainer;

impl Trainer {
    pub fn run(data: &str, output: &str) -> Result<PathBuf, String> {
        let out = PathBuf::from(output);

        // Try `py` (Windows launcher) first, fall back to `python`
        let launcher = if cfg!(windows) { "py" } else { "python3" };

        let status = std::process::Command::new(launcher)
            .args([
                "runtimes/bonsai-trainer/finetune.py",
                "--data", data,
                "--output", output,
            ])
            .status()
            .or_else(|_| {
                std::process::Command::new("python")
                    .args([
                        "runtimes/bonsai-trainer/finetune.py",
                        "--data", data,
                        "--output", output,
                    ])
                    .status()
            })
            .map_err(|e| format!("Failed to start training: {e}"))?;

        if !status.success() {
            return Err(format!("Training exited with code {:?}", status.code()));
        }
        Ok(out)
    }
}
