use std::env;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub environment: Environment,
    pub model_size: String,
    pub device: String,
    pub cpu_threads: usize,
}

lazy_static::lazy_static! {
    // Immutable configuration instance evaluated once at application startup.
    // Any invalid state will panic! / halt startup.
    pub static ref GLOBAL_CONFIG: AppConfig = load_configuration();
}

fn load_configuration() -> AppConfig {
    // 1. Base Configuration
    let mut config = AppConfig {
        environment: Environment::Development,
        model_size: crate::constants::config::WHISPER_DEFAULT_MODEL.to_string(),
        device: crate::constants::config::WHISPER_DEFAULT_DEVICE.to_string(),
        cpu_threads: crate::constants::config::WHISPER_DEFAULT_THREADS,
    };

    // 2. Environment Selection (Explicit & Deterministic)
    let env_var = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    config.environment = match env_var.to_lowercase().as_str() {
        "development" => Environment::Development,
        "testing" => Environment::Testing,
        "production" => Environment::Production,
        _ => panic!("CRITICAL: Invalid APP_ENV. Must be development, testing, or production."),
    };

    // Environment-Specific Overrides
    match config.environment {
        Environment::Testing => {
            config.cpu_threads = 1; // force minimal resource usage in tests
        }
        Environment::Production => {
            // Adjust production defaults if necessary
        }
        Environment::Development => {}
    }

    // 3. User Overrides (Loaded from ~/.audio-paste/config.json if exists)
    if let Some(mut home_dir) = dirs::home_dir() {
        home_dir.push(".audio-paste");
        let config_path = home_dir.join("config.json");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .unwrap_or_else(|_| panic!("CRITICAL: Failed to read user config at {:?}", config_path));
            let user_overrides: serde_json::Value = serde_json::from_str(&content)
                .unwrap_or_else(|_| panic!("CRITICAL: Invalid JSON in user config at {:?}", config_path));
            
            if let Some(m) = user_overrides.get("model_size").and_then(|v| v.as_str()) {
                config.model_size = m.to_string();
            }
            if let Some(d) = user_overrides.get("device").and_then(|v| v.as_str()) {
                config.device = d.to_string();
            }
            if let Some(t) = user_overrides.get("cpu_threads").and_then(|v| v.as_u64()) {
                config.cpu_threads = t as usize;
            }
        }
    }

    validate_config(&config);
    config
}

fn validate_config(config: &AppConfig) {
    if !crate::constants::config::WHISPER_AVAILABLE_MODELS.contains(&config.model_size.as_str()) {
        panic!("CRITICAL: Model {} is not supported.", config.model_size);
    }
    if config.cpu_threads < 1 || config.cpu_threads > 16 {
        panic!("CRITICAL: CPU threads must be between 1 and 16. Got {}", config.cpu_threads);
    }
}
