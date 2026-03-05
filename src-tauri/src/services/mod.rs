pub mod audio_service;
pub mod clipboard_service;
pub mod transcription_service;

#[cfg(target_os = "linux")]
pub mod hotkey_service;
