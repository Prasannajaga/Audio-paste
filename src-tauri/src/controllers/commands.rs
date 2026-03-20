use std::sync::RwLock;
use std::time::Instant;
use tauri::{State, Emitter};
use crate::config::env::GLOBAL_CONFIG;
use crate::config::whisper_config::WhisperConfig;
use crate::services::audio_service::AudioService;
use crate::services::transcription_service;
use crate::services::clipboard_service;

pub struct AppState {
    pub audio_service: AudioService,
    pub whisper_config: RwLock<Option<WhisperConfig>>,
}

#[tauri::command]
pub fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    eprintln!("[audio-paste] start_recording called");
    let service = &state.inner().audio_service;
    service.start_recording();
    Ok(())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) -> Result<(), String> {
    eprintln!("[audio-paste] stop_recording called");
    let service = &state.inner().audio_service;
    service.stop_recording();
    Ok(())
}

#[tauri::command]
pub fn process_transcription(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    eprintln!("[audio-paste][pipeline] process_transcription start");

    let config = {
        let guard = state
            .inner()
            .whisper_config
            .read()
            .map_err(|e| format!("Config lock error: {}", e))?;
        guard
            .clone()
            .ok_or("No Whisper Config loaded. Apply configuration first.")?
    };

    let audio_data = state.inner().audio_service.take_audio_buffer();
    eprintln!(
        "[audio-paste][pipeline] queued transcription samples={} sample_rate={} threshold={}",
        audio_data.len(),
        config.sample_rate,
        config.silence_threshold
    );

    if audio_data.is_empty() {
        eprintln!("[audio-paste][pipeline] no audio data, skipping transcription");
        let _ = app.emit("status_change", "IDLE");
        return Ok(String::new());
    }

    let app_for_job = app.clone();
    std::thread::spawn(move || {
        if let Err(err) = run_transcription_job(app_for_job.clone(), config, audio_data) {
            eprintln!("[audio-paste][pipeline] transcription job failed: {}", err);
            let _ = app_for_job.emit("status_change", "IDLE");
        }
    });

    let _ = app.emit("status_change", "TRANSCRIBING");
    Ok(String::new())
}

fn run_transcription_job(
    app: tauri::AppHandle,
    config: WhisperConfig,
    audio_data: Vec<f32>,
) -> Result<(), String> {
    let t0 = Instant::now();
    eprintln!(
        "[audio-paste][pipeline] transcription worker start samples={} cli={:?} model={:?}",
        audio_data.len(),
        config.cli_path,
        config.model_file_path
    );

    eprintln!(
        "[audio-paste][pipeline] captured_audio={} samples ({:.2}s at {}Hz)",
        audio_data.len(),
        audio_data.len() as f32 / config.sample_rate as f32,
        config.sample_rate
    );

    let trimmed = AudioService::trim_silence(&audio_data, config.silence_threshold);
    eprintln!(
        "[audio-paste][pipeline] trimmed_audio={} samples ({:.2}s)",
        trimmed.len(),
        trimmed.len() as f32 / config.sample_rate as f32
    );

    if trimmed.is_empty() {
        eprintln!("[audio-paste][pipeline] trimmed audio is empty");
        let _ = app.emit("status_change", "IDLE");
        return Ok(());
    }

    let tmp_wav = AudioService::write_temp_wav(&trimmed, config.sample_rate)?;
    eprintln!(
        "[audio-paste][pipeline] temp_wav={} size={} bytes",
        tmp_wav,
        std::fs::metadata(&tmp_wav).map(|m| m.len()).unwrap_or(0)
    );

    let t1 = Instant::now();
    let text = match transcription_service::transcribe(&config, &tmp_wav) {
        Ok(text) => text,
        Err(primary_err) => {
            eprintln!("[audio-paste][pipeline] primary transcription failed: {}", primary_err);
            let current_model = config
                .model_file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if current_model.contains("tiny") {
                return Err(primary_err);
            }

            eprintln!("[audio-paste][pipeline] attempting tiny fallback after model failure");
            let root = WhisperConfig::resolve_project_root()?;
            let tiny_config = WhisperConfig::new(
                &app,
                root,
                crate::constants::config::WHISPER_DEFAULT_MODEL,
                config.cpu_threads,
                config.sample_rate,
                config.silence_threshold,
            )?;
            match transcription_service::transcribe(&tiny_config, &tmp_wav) {
                Ok(text) => {
                    eprintln!("[audio-paste][pipeline] tiny fallback succeeded");
                    text
                }
                Err(fallback_err) => {
                    return Err(format!(
                        "Primary whisper run failed: {}; tiny fallback also failed: {}",
                        primary_err, fallback_err
                    ));
                }
            }
        }
    };

    eprintln!(
        "[audio-paste][pipeline] transcription_complete in {}ms text_len={}",
        t1.elapsed().as_millis(),
        text.len()
    );

    if let Err(e) = std::fs::remove_file(&tmp_wav) {
        eprintln!("[audio-paste][pipeline] temp_wav cleanup failed for {}: {}", tmp_wav, e);
    }

    if !text.trim().is_empty() {
        eprintln!("[audio-paste][pipeline] transcription non-empty -> emit + clipboard paste");
        let _ = app.emit("transcription_result", text.clone());
        clipboard_service::paste_text(&text);
    } else {
        eprintln!("[audio-paste][pipeline] transcription empty");
    }

    let total_ms = t0.elapsed().as_millis();
    eprintln!("[audio-paste][pipeline] done in {}ms", total_ms);
    let _ = app.emit("status_change", "IDLE");
    Ok(())
}

#[tauri::command]
pub fn apply_config(
    app: tauri::AppHandle,
    model: String,
    device: String,
    threads: usize,
    silence_threshold: f32,
    silence_seconds: u64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!(
        "[audio-paste] apply_config: model={}, device={}, threads={}, silence_threshold={}, silence_seconds={}",
        model, device, threads, silence_threshold, silence_seconds
    );
    let _ = app.emit("status_change", "LOADING");
    let root = WhisperConfig::resolve_project_root()?;
    let config = WhisperConfig::new(
        &app,
        root,
        &model,
        threads,
        GLOBAL_CONFIG.sample_rate,
        silence_threshold,
    )?;
    eprintln!("[audio-paste] Config applied: model_file={:?}, threads={}", config.model_file_path, config.cpu_threads);
    {
        let mut whisper = state.inner().whisper_config.write()
            .map_err(|e| format!("Config lock error: {}", e))?;
        *whisper = Some(config.clone());
    }
    {
        let audio = &state.inner().audio_service;
        let mut runtime = audio.config.lock().unwrap();
        *runtime = crate::config::env::AppConfig {
            environment: GLOBAL_CONFIG.environment.clone(),
            model_size: model.clone(),
            device: device.clone(),
            cpu_threads: threads,
            sample_rate: GLOBAL_CONFIG.sample_rate,
            silence_threshold,
            silence_seconds,
        };
    }
    let _ = app.emit("status_change", "IDLE");
    Ok(())
}
