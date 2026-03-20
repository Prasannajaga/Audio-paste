use std::path::{Path, PathBuf};
use std::process::Command;
use crate::constants::config;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone)]
pub struct WhisperConfig {
    pub cli_path: PathBuf,
    pub models_dir: PathBuf,
    pub model_file_path: PathBuf,
    pub cpu_threads: usize,
    pub sample_rate: u32,
    pub silence_threshold: f32,
}

impl WhisperConfig {
    pub fn new(
        app: &AppHandle,
        project_root: PathBuf,
        model_name: &str,
        cpu_threads: usize,
        sample_rate: u32,
        silence_threshold: f32,
    ) -> Result<Self, String> {
        eprintln!(
            "[audio-paste][model] init start model={} threads={} project_root={:?}",
            model_name, cpu_threads, project_root
        );
        let cli_path = Self::resolve_or_stage_cli(app, &project_root)?;

        let model_file_path = Self::resolve_or_download_model(model_name)?;
        let model_size = std::fs::metadata(&model_file_path).map(|m| m.len()).unwrap_or(0);
        eprintln!(
            "[audio-paste][model] ready model={} path={:?} size={} bytes",
            model_name, model_file_path, model_size
        );

        Ok(WhisperConfig {
            cli_path,
            models_dir: model_file_path.parent().unwrap_or_else(|| std::path::Path::new("")).to_path_buf(),
            model_file_path,
            cpu_threads,
            sample_rate,
            silence_threshold,
        })
    }

    fn whisper_root() -> Result<PathBuf, String> {
        let mut base = dirs::data_local_dir()
            .or_else(dirs::home_dir)
            .ok_or("Failed to resolve a writable data directory")?;
        base.push(config::AUDIO_PASTE_DATA_DIR);
        base.push(config::WHISPER_ASSETS_SUBDIR);
        Ok(base)
    }

    fn models_dir() -> Result<PathBuf, String> {
        let mut base = Self::whisper_root()?;
        base.push(config::WHISPER_MODELS_SUBDIR);
        eprintln!("[audio-paste][model] models dir = {:?}", base);
        Ok(base)
    }

    fn cli_staged_path() -> Result<PathBuf, String> {
        let mut base = Self::whisper_root()?;
        base.push(config::WHISPER_CLI_BINARY_NAME);
        Ok(base)
    }

    fn find_cli_on_path() -> Option<PathBuf> {
        let output = Command::new("sh")
            .arg("-lc")
            .arg("command -v whisper-cli")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            None
        } else {
            Some(PathBuf::from(path))
        }
    }

    fn local_cli_path(project_root: &Path) -> PathBuf {
        project_root.join("whisper.cpp").join("build").join("bin").join(config::WHISPER_CLI_BINARY_NAME)
    }

    fn installed_cli_path() -> Result<PathBuf, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        let bin_dir = exe.parent().ok_or("Failed to resolve executable directory")?;
        let mut path = bin_dir.to_path_buf();
        path.pop(); // /usr/bin -> /usr
        path.push("lib");
        path.push("audio-paste");
        path.push("whisper.cpp");
        path.push("build");
        path.push("bin");
        path.push(config::WHISPER_CLI_BINARY_NAME);
        Ok(path)
    }

    fn ensure_executable(path: &Path) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)
                .map_err(|e| format!("Failed to read metadata for {:?}: {}", path, e))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms)
                .map_err(|e| format!("Failed to set executable bit for {:?}: {}", path, e))?;
        }
        Ok(())
    }

    fn resolve_or_stage_cli(app: &AppHandle, project_root: &Path) -> Result<PathBuf, String> {
        let staged = Self::cli_staged_path()?;
        eprintln!("[audio-paste][model] staged cli path = {:?}", staged);
        if staged.is_file() {
            eprintln!("[audio-paste][model] using staged whisper-cli at {:?}", staged);
            return Ok(staged);
        }

        let resource = app.path()
            .resolve("whisper-cli", tauri::path::BaseDirectory::Resource)
            .map_err(|e| format!("Failed to resolve bundled whisper-cli resource: {}", e))?;
        eprintln!("[audio-paste][model] checking resource cli path {:?}", resource);
        if resource.is_file() {
            if let Some(parent) = staged.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create whisper staging dir {:?}: {}", parent, e))?;
            }
            eprintln!("[audio-paste][model] staging packaged whisper-cli from {:?}", resource);
            std::fs::copy(&resource, &staged)
                .map_err(|e| format!("Failed to copy packaged whisper-cli from {:?} to {:?}: {}", resource, staged, e))?;
            Self::ensure_executable(&staged)?;
            return Ok(staged);
        }

        let installed = Self::installed_cli_path()?;
        eprintln!("[audio-paste][model] checking installed cli path {:?}", installed);
        if installed.is_file() {
            eprintln!("[audio-paste][model] staging installed whisper-cli from {:?}", installed);
            std::fs::copy(&installed, &staged)
                .map_err(|e| format!("Failed to copy installed whisper-cli from {:?} to {:?}: {}", installed, staged, e))?;
            Self::ensure_executable(&staged)?;
            return Ok(staged);
        }

        if let Some(parent) = staged.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create whisper staging dir {:?}: {}", parent, e))?;
        }

        let local = Self::local_cli_path(project_root);
        eprintln!("[audio-paste][model] checking local cli path {:?}", local);
        if local.is_file() {
            eprintln!("[audio-paste][model] staging local whisper-cli from {:?}", local);
            std::fs::copy(&local, &staged)
                .map_err(|e| format!("Failed to copy whisper-cli from {:?} to {:?}: {}", local, staged, e))?;
            Self::ensure_executable(&staged)?;
            return Ok(staged);
        }

        if let Some(path_cli) = Self::find_cli_on_path() {
            eprintln!("[audio-paste][model] using PATH whisper-cli at {:?}", path_cli);
            return Ok(path_cli);
        }

        Err(format!(
            "whisper-cli not found in app data at {:?}, not found in repo at {:?}, and not on PATH.",
            staged, local
        ))
    }

    fn model_download_url(model_name: &str) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}{}{}",
            config::WHISPER_GGML_MODEL_PREFIX,
            model_name,
            config::WHISPER_GGML_MODEL_SUFFIX
        )
    }

    fn ensure_model_present(model_name: &str, model_file_path: &PathBuf) -> Result<(), String> {
        if model_file_path.is_file() {
            if std::fs::metadata(model_file_path).map(|m| m.len() > 0).unwrap_or(false) {
                eprintln!("[audio-paste][model] cache hit for {:?} ({} bytes)", model_file_path, std::fs::metadata(model_file_path).map(|m| m.len()).unwrap_or(0));
                return Ok(());
            }
            eprintln!("[audio-paste][model] found empty/corrupt model file, deleting {:?}", model_file_path);
            let _ = std::fs::remove_file(model_file_path);
        }

        let url = Self::model_download_url(model_name);
        eprintln!(
            "[audio-paste][model] downloading model={} target={:?} url={}",
            model_name, model_file_path, url
        );

        let status = if Command::new("curl").arg("--version").output().is_ok() {
            eprintln!("[audio-paste][model] downloader=curl");
            Command::new("curl")
                .arg("-L")
                .arg("--fail")
                .arg("--output")
                .arg(model_file_path)
                .arg(&url)
                .status()
        } else if Command::new("wget").arg("--version").output().is_ok() {
            eprintln!("[audio-paste][model] downloader=wget");
            Command::new("wget")
                .arg("-O")
                .arg(model_file_path)
                .arg(&url)
                .status()
        } else {
            return Err("Neither curl nor wget is available to download Whisper models.".to_string());
        }
        .map_err(|e| format!("Failed to launch downloader: {}", e))?;

        if !status.success() {
            return Err(format!("Failed to download model '{}' from {}", model_name, url));
        }

        if !model_file_path.is_file() {
            return Err(format!("Downloader finished but model file is still missing at {:?}", model_file_path));
        }

        let downloaded_size = std::fs::metadata(model_file_path).map(|m| m.len()).unwrap_or(0);
        eprintln!(
            "[audio-paste][model] download complete model={} path={:?} size={} bytes",
            model_name, model_file_path, downloaded_size
        );

        if downloaded_size == 0 {
            return Err(format!("Downloaded model at {:?} is empty", model_file_path));
        }

        Ok(())
    }

    pub fn resolve_project_root() -> Result<PathBuf, String> {
        if let Ok(override_root) = std::env::var("AUDIO_PASTE_PROJECT_ROOT") {
            let override_path = PathBuf::from(&override_root);
            if override_path.join("whisper.cpp").is_dir() {
                return Ok(override_path);
            }
        }

        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or("Failed to get executable parent dir")?;

        let candidates = [
            exe_dir.join("../../../"),
            exe_dir.join("../../"),
            exe_dir.join(".."),
            std::env::current_dir().map_err(|e| format!("Failed to get cwd: {}", e))?,
        ];

        for candidate in candidates {
            if let Ok(canonical) = candidate.canonicalize() {
                if canonical.join("whisper.cpp").is_dir() {
                    return Ok(canonical);
                }
            }
        }

        Err(format!(
            "Could not locate project root. Set AUDIO_PASTE_PROJECT_ROOT or place whisper.cpp next to the app source tree."
        ))
    }

    pub fn resolve_or_download_model(model_name: &str) -> Result<PathBuf, String> {
        eprintln!("[audio-paste][model] resolve_or_download_model model={}", model_name);
        let mut models_dir = Self::models_dir()?;
        if !models_dir.is_dir() {
            eprintln!("[audio-paste][model] creating models dir {:?}", models_dir);
            std::fs::create_dir_all(&models_dir)
                .map_err(|e| format!("Failed to create models directory at {:?}: {}", models_dir, e))?;
        }

        let model_filename = format!(
            "{}{}{}",
            config::WHISPER_GGML_MODEL_PREFIX,
            model_name,
            config::WHISPER_GGML_MODEL_SUFFIX
        );
        models_dir.push(model_filename);
        eprintln!("[audio-paste][model] resolved model path {:?}", models_dir);

        Self::ensure_model_present(model_name, &models_dir)?;
        Ok(models_dir)
    }
}
