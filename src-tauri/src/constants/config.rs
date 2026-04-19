pub const SAMPLE_RATE: u32 = 16000;
pub const SILENCE_THRESHOLD: f32 = 0.01;
pub const SILENCE_SECONDS: u64 = 2;

#[cfg(target_os = "windows")]
pub const WHISPER_CLI_BINARY_NAME: &str = "whisper-cli.exe";

#[cfg(not(target_os = "windows"))]
pub const WHISPER_CLI_BINARY_NAME: &str = "whisper-cli";
pub const AUDIO_PASTE_DATA_DIR: &str = "audio-paste";
pub const WHISPER_ASSETS_SUBDIR: &str = "whisper";
pub const WHISPER_MODELS_SUBDIR: &str = "models";
pub const WHISPER_GGML_MODEL_PREFIX: &str = "ggml-";
pub const WHISPER_GGML_MODEL_SUFFIX: &str = ".bin";

pub const WHISPER_AVAILABLE_MODELS: &[&str] = &["tiny", "base", "tiny.en", "base.en"];

pub const WHISPER_DEFAULT_MODEL: &str = "tiny";
pub const WHISPER_DEFAULT_DEVICE: &str = "cpu";
pub const WHISPER_DEFAULT_THREADS: usize = 4;
pub const WHISPER_TRANSCRIPTION_TIMEOUT_SECONDS: u64 = 120;
