use std::process::Command;
use std::time::Instant;
use crate::config::whisper_config::WhisperConfig;

fn preview(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let prefix: String = trimmed.chars().take(max_chars).collect();
    format!("{}...", prefix)
}

pub fn transcribe(config: &WhisperConfig, wav_path: &str) -> Result<String, String> {
    eprintln!(
        "[audio-paste][transcribe] exec: {:?} -m {:?} -f {:?} -t {} -nt -l en -np",
        config.cli_path,
        config.model_file_path,
        wav_path,
        config.cpu_threads
    );

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
    let code = output.status.code().map_or(-1, |c| c);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!(
        "[audio-paste][transcribe] finished in {}ms exit_code={} stdout_bytes={} stderr_bytes={}",
        elapsed,
        code,
        output.stdout.len(),
        output.stderr.len()
    );
    if !stdout.trim().is_empty() {
        eprintln!(
            "[audio-paste][transcribe] stdout_preview={}",
            preview(&stdout, 400)
        );
    }
    if !stderr.trim().is_empty() {
        eprintln!(
            "[audio-paste][transcribe] stderr_preview={}",
            preview(&stderr, 400)
        );
    }

    if !output.status.success() {
        return Err(format!(
            "Whisper CLI failed ({}ms, exit_code={}): {}",
            elapsed,
            code,
            preview(&stderr, 500)
        ));
    }

    let parsed: Vec<&str> = stdout.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("whisper_") && !l.starts_with("["))
        .collect();

    let result = parsed.join(" ");
    eprintln!(
        "[audio-paste][transcribe] parsed_text_len={} text_preview={:?}",
        result.len(),
        preview(&result, 200)
    );
    Ok(result)
}
