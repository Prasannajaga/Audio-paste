use std::sync::Mutex;
use std::time::Instant;
use tauri::{State, Emitter};
use crate::config::whisper_config::WhisperConfig;
use crate::services::audio_service::AudioService;
use crate::services::transcription_service;
use crate::services::clipboard_service;

pub struct AppState {
    pub audio_service: Mutex<AudioService>,
    pub whisper_config: Mutex<Option<WhisperConfig>>,
}

#[tauri::command]
pub fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    eprintln!("[audio-paste] start_recording called");
    let service = state.inner().audio_service.lock().map_err(|e| format!("Lock error: {}", e))?;
    service.start_recording();
    Ok(())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) -> Result<(), String> {
    eprintln!("[audio-paste] stop_recording called");
    let service = state.inner().audio_service.lock().map_err(|e| format!("Lock error: {}", e))?;
    service.stop_recording();
    Ok(())
}

#[tauri::command]
pub async fn process_transcription(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let t0 = Instant::now();
    eprintln!("[audio-paste] process_transcription called");

    let (audio_data, config) = {
        let service = state.inner().audio_service.lock()
            .map_err(|e| format!("Audio lock error: {}", e))?;
        let audio = service.get_and_clear_audio();
        let config = state.inner().whisper_config.lock()
            .map_err(|e| format!("Config lock error: {}", e))?
            .clone()
            .ok_or("No Whisper Config loaded. Apply configuration first.")?;
        (audio, config)
    };

    if audio_data.is_empty() {
        eprintln!("[audio-paste] No audio data, skipping");
        let _ = app.emit("status_change", "IDLE");
        return Ok(String::new());
    }

    eprintln!("[audio-paste] Audio: {} samples ({:.1}s at 16kHz)",
        audio_data.len(), audio_data.len() as f32 / 16000.0);

    let trimmed = AudioService::trim_silence(&audio_data, crate::constants::config::SILENCE_THRESHOLD);
    eprintln!("[audio-paste] Trimmed: {} samples ({:.1}s)",
        trimmed.len(), trimmed.len() as f32 / 16000.0);

    if trimmed.is_empty() {
        let _ = app.emit("status_change", "IDLE");
        return Ok(String::new());
    }

    let tmp_wav = AudioService::write_temp_wav(&trimmed)?;

    let t1 = Instant::now();
    let text = transcription_service::transcribe(&config, &tmp_wav)?;
    let transcribe_ms = t1.elapsed().as_millis();
    eprintln!("[audio-paste] Transcription took {}ms: {:?}", transcribe_ms, text);

    let _ = std::fs::remove_file(&tmp_wav);

    if !text.trim().is_empty() {
        let _ = app.emit("transcription_result", text.clone());
        clipboard_service::paste_text(&text);
    }

    let total_ms = t0.elapsed().as_millis();
    eprintln!("[audio-paste] Total pipeline: {}ms", total_ms);

    let _ = app.emit("status_change", "IDLE");
    Ok(text)
}

#[tauri::command]
pub fn apply_config(
    model: String,
    device: String,
    threads: usize,
    state: State<'_, AppState>,
) -> Result<(), String> {
    eprintln!("[audio-paste] apply_config: model={}, device={}, threads={}", model, device, threads);
    let root = WhisperConfig::resolve_project_root()?;
    let config = WhisperConfig::new(root, &model, threads)?;
    eprintln!("[audio-paste] Config applied: model_file={:?}, threads={}", config.model_file_path, config.cpu_threads);
    *state.inner().whisper_config.lock()
        .map_err(|e| format!("Config lock error: {}", e))? = Some(config);
    Ok(())
}
