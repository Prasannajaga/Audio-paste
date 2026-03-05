use std::process::Command;
use std::time::Instant;
use crate::config::whisper_config::WhisperConfig;

pub fn transcribe(config: &WhisperConfig, wav_path: &str) -> Result<String, String> {
    eprintln!("[audio-paste] whisper-cli: {:?} -m {:?} -t {}",
        config.cli_path, config.model_file_path.file_name().unwrap_or_default(), config.cpu_threads);

    let t0 = Instant::now();

    let output = Command::new(&config.cli_path)
        .arg("-m").arg(&config.model_file_path)
        .arg("-f").arg(wav_path)
        .arg("-t").arg(config.cpu_threads.to_string())
        .arg("-nt")
        .arg("-l").arg("en")
        .arg("-np")
        .output()
        .map_err(|e| format!("Failed to execute whisper-cli: {}", e))?;

    let elapsed = t0.elapsed().as_millis();
    eprintln!("[audio-paste] whisper-cli finished in {}ms", elapsed);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Whisper CLI error ({}ms): {}", elapsed, stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let parsed: Vec<&str> = stdout.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("whisper_") && !l.starts_with("["))
        .collect();

    let result = parsed.join(" ");
    eprintln!("[audio-paste] Result ({}ms): {:?}", elapsed, result);
    Ok(result)
}
