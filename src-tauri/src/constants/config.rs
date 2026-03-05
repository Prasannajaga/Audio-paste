pub const SAMPLE_RATE: u32 = 16000;
pub const SILENCE_THRESHOLD: f32 = 0.01;
pub const SILENCE_SECONDS: u64 = 3;

pub const WHISPER_CLI_BINARY_NAME: &str = "whisper-cli";
pub const WHISPER_BUILD_DIR: &str = "whisper.cpp/build/bin";
pub const WHISPER_MODELS_DIR: &str = "whisper.cpp/models";
pub const WHISPER_GGML_MODEL_PREFIX: &str = "ggml-";
pub const WHISPER_GGML_MODEL_SUFFIX: &str = ".bin";

pub const WHISPER_AVAILABLE_MODELS: &[&str] = &[
    "tiny", "tiny.en", "base", "base.en", "small", "small.en",
    "medium", "medium.en", "large-v1", "large-v2", "large-v3", "large-v3-turbo",
];

pub const WHISPER_DEFAULT_MODEL: &str = "base.en";
pub const WHISPER_DEFAULT_DEVICE: &str = "cpu";
pub const WHISPER_DEFAULT_THREADS: usize = 4;
pub const WHISPER_TRANSCRIPTION_TIMEOUT_SECONDS: u64 = 120;
