use std::process::{Command, Stdio, Output};
use std::thread;
use std::time::{Duration, Instant};
use crate::constants::config;
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
    let model_size = std::fs::metadata(&config.model_file_path).map(|m| m.len()).unwrap_or(0);
    eprintln!(
        "[audio-paste][transcribe] start cli={:?} model={:?} model_size={} bytes wav={:?} threads={}",
        config.cli_path,
        config.model_file_path,
        model_size,
        wav_path,
        config.cpu_threads
    );

    let output = run_whisper_cli(config, wav_path)?;
    let elapsed = output.elapsed_ms;
    let output = output.output;

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
        eprintln!(
            "[audio-paste][transcribe] failure cli={:?} model={:?} wav={:?}",
            config.cli_path, config.model_file_path, wav_path
        );
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

struct TimedOutput {
    output: Output,
    elapsed_ms: u128,
}

fn run_whisper_cli(config: &WhisperConfig, wav_path: &str) -> Result<TimedOutput, String> {
    let timeout = Duration::from_secs(config::WHISPER_TRANSCRIPTION_TIMEOUT_SECONDS);
    eprintln!(
        "[audio-paste][transcribe] exec: {:?} -m {:?} -f {:?} -t {} -nt -l en -np timeout={}s",
        config.cli_path,
        config.model_file_path,
        wav_path,
        config.cpu_threads,
        timeout.as_secs()
    );

    let t0 = Instant::now();
    let mut child = Command::new(&config.cli_path)
        .arg("-m").arg(&config.model_file_path)
        .arg("-f").arg(wav_path)
        .arg("-t").arg(config.cpu_threads.to_string())
        .arg("-nt")
        .arg("-l").arg("en")
        .arg("-np")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to execute whisper-cli at {:?}: {}", config.cli_path, e))?;

    eprintln!("[audio-paste][transcribe] spawned whisper-cli pid={}", child.id());

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                eprintln!(
                    "[audio-paste][transcribe] whisper-cli exited status={} elapsed={}ms",
                    status,
                    t0.elapsed().as_millis()
                );
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("Failed to collect whisper-cli output: {}", e))?;
                return Ok(TimedOutput {
                    output,
                    elapsed_ms: t0.elapsed().as_millis(),
                });
            }
            Ok(None) => {
                if t0.elapsed() > timeout {
                    eprintln!(
                        "[audio-paste][transcribe] whisper-cli timeout after {}s, killing pid={}",
                        timeout.as_secs(),
                        child.id()
                    );
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Whisper CLI timed out after {} seconds",
                        timeout.as_secs()
                    ));
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Failed to poll whisper-cli process: {}", e));
            }
        }
    }
}
