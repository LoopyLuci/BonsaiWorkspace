use std::path::PathBuf;

pub struct Trainer;

impl Trainer {
    /// Run finetune.py with a local GGUF model path (100% offline).
    /// Falls back to the Python 3.12 absolute path on Windows if `py` fails.
    pub fn run(gguf_path: Option<&str>, data: &str, output: &str) -> Result<PathBuf, String> {
        let out = PathBuf::from(output);

        let mut base_args: Vec<&str> = vec![
            "runtimes/bonsai-trainer/finetune.py",
            "--data",
            data,
            "--output",
            output,
            "--local-only",
        ];

        // Build --gguf arg outside the match to keep the lifetime alive
        let gguf_owned;
        if let Some(g) = gguf_path {
            if !PathBuf::from(g).exists() {
                return Err(format!("GGUF model not found: {g}"));
            }
            gguf_owned = Some(g.to_string());
            base_args.push("--gguf");
            base_args.push(gguf_owned.as_deref().unwrap());
        }

        // Windows Python 3.12 absolute path — avoids py.exe launcher issues
        #[cfg(windows)]
        let python_fallback = {
            let appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
            format!("{appdata}\\Programs\\Python\\Python312\\python.exe")
        };
        #[cfg(not(windows))]
        let python_fallback = "python3".to_string();

        let launchers: &[&str] = &["py", &python_fallback, "python"];

        let mut last_err = String::new();
        for launcher in launchers {
            let result = std::process::Command::new(launcher)
                .args(&base_args)
                .env("TRANSFORMERS_OFFLINE", "1")
                .env("HF_HUB_OFFLINE", "1")
                .env("HF_DATASETS_OFFLINE", "1")
                .env("HF_HUB_DISABLE_TELEMETRY", "1")
                .env("PYTHONUTF8", "1")
                .env("PYTHONUNBUFFERED", "1")
                .status();

            match result {
                Ok(status) if status.success() => return Ok(out),
                Ok(status) => return Err(format!("Training failed (exit {:?})", status.code())),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    last_err = format!("{launcher}: not found");
                    continue;
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Err(format!(
            "No Python interpreter found. Tried: py, python3, python. Last: {last_err}"
        ))
    }
}
