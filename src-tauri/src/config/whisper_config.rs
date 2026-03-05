use std::path::PathBuf;
use crate::constants::config;

#[derive(Debug, Clone)]
pub struct WhisperConfig {
    pub cli_path: PathBuf,
    pub models_dir: PathBuf,
    pub model_file_path: PathBuf,
    pub cpu_threads: usize,
}

impl WhisperConfig {
    pub fn new(project_root: PathBuf, model_name: &str, cpu_threads: usize) -> Result<Self, String> {
        let cli_path = project_root.join(config::WHISPER_BUILD_DIR).join(config::WHISPER_CLI_BINARY_NAME);
        if !cli_path.is_file() {
            return Err(format!("whisper-cli not found at {:?}. Please build whisper.cpp first.", cli_path));
        }

        let models_dir = project_root.join(config::WHISPER_MODELS_DIR);
        if !models_dir.is_dir() {
            return Err(format!("models directory not found at {:?}", models_dir));
        }

        let model_filename = format!("{}{}{}", config::WHISPER_GGML_MODEL_PREFIX, model_name, config::WHISPER_GGML_MODEL_SUFFIX);
        let model_file_path = models_dir.join(&model_filename);

        if !model_file_path.is_file() {
            return Err(format!("model file not found at {:?}", model_file_path));
        }

        Ok(WhisperConfig {
            cli_path,
            models_dir,
            model_file_path,
            cpu_threads,
        })
    }

    pub fn resolve_project_root() -> Result<PathBuf, String> {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or("Failed to get executable parent dir")?;

        // In dev mode: target/debug/audio_paste → go up 3 levels to project root
        // In release:  same structure applies
        let candidate = exe_dir.join("../../../");
        let candidate = candidate.canonicalize()
            .map_err(|e| format!("Failed to resolve project root from exe dir {:?}: {}", exe_dir, e))?;

        let whisper_dir = candidate.join("whisper.cpp");
        if whisper_dir.is_dir() {
            return Ok(candidate);
        }

        // Fallback: try current working directory
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Failed to get cwd: {}", e))?;
        let whisper_dir = cwd.join("whisper.cpp");
        if whisper_dir.is_dir() {
            return Ok(cwd);
        }

        Err(format!(
            "Could not locate project root. Checked {:?} and {:?}. Ensure whisper.cpp/ directory exists.",
            candidate, cwd
        ))
    }
}
