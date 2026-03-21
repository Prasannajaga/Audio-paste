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
        model_name: &str,
        cpu_threads: usize,
        sample_rate: u32,
        silence_threshold: f32,
    ) -> Result<Self, String> {
        let project_root = Self::resolve_project_root().ok();
        eprintln!(
            "[audio-paste][model] init start model={} threads={} project_root={:?}",
            model_name, cpu_threads, project_root
        );
        let cli_path = Self::resolve_or_stage_cli(app, project_root.as_deref())?;

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

    fn ensure_staging_parent(staged: &Path) -> Result<(), String> {
        if let Some(parent) = staged.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create whisper staging dir {:?}: {}", parent, e))?;
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn runtime_library_names() -> &'static [&'static str] {
        &[
            "libwhisper.so.1",
            "libggml.so.0",
            "libggml-base.so.0",
            "libggml-cpu.so.0",
        ]
    }

    #[cfg(not(target_os = "linux"))]
    fn runtime_library_names() -> &'static [&'static str] {
        &[]
    }

    fn staged_runtime_libraries_present(staged: &Path) -> bool {
        if Self::runtime_library_names().is_empty() {
            return true;
        }
        let Some(parent) = staged.parent() else {
            return false;
        };
        Self::runtime_library_names()
            .iter()
            .all(|name| parent.join(name).is_file())
    }

    #[cfg(target_os = "linux")]
    fn stage_runtime_libraries(source_cli: &Path, staged_cli: &Path) -> Result<(), String> {
        let source_bin_dir = source_cli
            .parent()
            .ok_or_else(|| format!("Failed to resolve parent directory for {:?}", source_cli))?;
        let build_dir = source_bin_dir
            .parent()
            .ok_or_else(|| format!("Failed to resolve build directory for {:?}", source_cli))?;
        let staged_dir = staged_cli
            .parent()
            .ok_or_else(|| format!("Failed to resolve staged dir for {:?}", staged_cli))?;

        for lib_name in Self::runtime_library_names() {
            let source = if lib_name.starts_with("libwhisper") {
                build_dir.join("src").join(lib_name)
            } else {
                build_dir.join("ggml").join("src").join(lib_name)
            };

            if !source.is_file() {
                return Err(format!(
                    "Required runtime library {:?} not found next to whisper-cli source at {:?}",
                    lib_name, source
                ));
            }

            let target = staged_dir.join(lib_name);
            std::fs::copy(&source, &target).map_err(|e| {
                format!(
                    "Failed to stage runtime library {:?} from {:?} to {:?}: {}",
                    lib_name, source, target, e
                )
            })?;
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn stage_runtime_libraries(_source_cli: &Path, _staged_cli: &Path) -> Result<(), String> {
        Ok(())
    }

    fn stage_cli(source: &Path, staged: &Path, source_label: &str) -> Result<PathBuf, String> {
        Self::ensure_staging_parent(staged)?;
        eprintln!(
            "[audio-paste][model] staging whisper-cli from {} {:?} -> {:?}",
            source_label, source, staged
        );
        std::fs::copy(source, staged).map_err(|e| {
            format!(
                "Failed to copy whisper-cli from {} {:?} to {:?}: {}",
                source_label, source, staged, e
            )
        })?;
        Self::stage_runtime_libraries(source, staged)?;
        Self::ensure_executable(staged)?;
        Ok(staged.to_path_buf())
    }

    fn bundled_resource_cli_candidates(app: &AppHandle) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        let resource_names = [
            "whisper-cli",
            "whisper.cpp/build/bin/whisper-cli",
        ];
        for name in resource_names {
            if let Ok(path) = app.path().resolve(name, tauri::path::BaseDirectory::Resource) {
                candidates.push(path);
            }
        }

        if let Ok(resource_dir) = app.path().resource_dir() {
            candidates.push(resource_dir.join("whisper-cli"));
            candidates.push(
                resource_dir
                    .join("whisper.cpp")
                    .join("build")
                    .join("bin")
                    .join(config::WHISPER_CLI_BINARY_NAME),
            );
        }

        candidates
    }

    fn resolve_or_stage_cli(app: &AppHandle, project_root: Option<&Path>) -> Result<PathBuf, String> {
        let staged = Self::cli_staged_path()?;
        let mut staging_errors: Vec<String> = Vec::new();
        eprintln!("[audio-paste][model] staged cli path = {:?}", staged);
        if staged.is_file() {
            let _ = Self::ensure_executable(&staged);
            if Self::staged_runtime_libraries_present(&staged) {
                eprintln!("[audio-paste][model] using staged whisper-cli at {:?}", staged);
                return Ok(staged);
            }
            eprintln!(
                "[audio-paste][model] staged whisper-cli found but runtime libraries are missing; restaging"
            );
        }

        for resource in Self::bundled_resource_cli_candidates(app) {
            eprintln!("[audio-paste][model] checking resource cli path {:?}", resource);
            if resource.is_file() {
                match Self::stage_cli(&resource, &staged, "resource") {
                    Ok(path) => return Ok(path),
                    Err(err) => {
                        eprintln!("[audio-paste][model] failed to stage resource cli: {}", err);
                        staging_errors.push(err);
                    }
                }
            }
        }

        let installed = Self::installed_cli_path()?;
        eprintln!("[audio-paste][model] checking installed cli path {:?}", installed);
        if installed.is_file() {
            match Self::stage_cli(&installed, &staged, "installed") {
                Ok(path) => return Ok(path),
                Err(err) => {
                    eprintln!("[audio-paste][model] failed to stage installed cli: {}", err);
                    staging_errors.push(err);
                }
            }
        }

        let mut local_candidate: Option<PathBuf> = None;
        if let Some(root) = project_root {
            let local = Self::local_cli_path(root);
            eprintln!("[audio-paste][model] checking local cli path {:?}", local);
            if local.is_file() {
                match Self::stage_cli(&local, &staged, "local") {
                    Ok(path) => return Ok(path),
                    Err(err) => {
                        eprintln!("[audio-paste][model] failed to stage local cli: {}", err);
                        staging_errors.push(err);
                    }
                }
            }
            local_candidate = Some(local);
        }

        if let Some(path_cli) = Self::find_cli_on_path() {
            eprintln!("[audio-paste][model] using PATH whisper-cli at {:?}", path_cli);
            return Ok(path_cli);
        }

        match local_candidate {
            Some(local) => Err(format!(
                "whisper-cli not available in app data at {:?}, bundled resources, installed location, repo path {:?}, or PATH. staging_errors={}",
                staged,
                local,
                if staging_errors.is_empty() { "none".to_string() } else { staging_errors.join(" | ") }
            )),
            None => Err(format!(
                "whisper-cli not available in app data at {:?}, bundled resources, installed location, or PATH. staging_errors={}",
                staged,
                if staging_errors.is_empty() { "none".to_string() } else { staging_errors.join(" | ") }
            )),
        }
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
