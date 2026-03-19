use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use hound::{WavSpec, WavWriter, SampleFormat as HoundSampleFormat};
use crate::constants::config::{SAMPLE_RATE, SILENCE_THRESHOLD, SILENCE_SECONDS};

pub struct AudioState {
    pub is_recording: bool,
    pub audio_buffer: Vec<f32>,
    pub finalizing: bool,
    pub voice_started: bool,
    pub silent_samples_accum: usize,
    pub debug_samples_accum: usize,
    pub recording_samples_accum: usize,
    pub noise_rms_ema: f32,
    pub noise_peak_ema: f32,
    pub noise_activity_ema: f32,
}

pub struct AudioService {
    pub state: Arc<Mutex<AudioState>>,
}

impl AudioService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AudioState {
                is_recording: false,
                audio_buffer: Vec::new(),
                finalizing: false,
                voice_started: false,
                silent_samples_accum: 0,
                debug_samples_accum: 0,
                recording_samples_accum: 0,
                noise_rms_ema: 0.0,
                noise_peak_ema: 0.0,
                noise_activity_ema: 0.0,
            })),
        }
    }

    pub fn start_listening(&mut self, silence_tx: std::sync::mpsc::Sender<()>) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or("Failed to get default input device")?;
        let device_name = device.name().unwrap_or_else(|_| "<unknown>".to_string());
        eprintln!(
            "[audio-paste][audio] input device='{}' sample_rate={} threshold={} silence_seconds={}",
            device_name, SAMPLE_RATE, SILENCE_THRESHOLD, SILENCE_SECONDS
        );

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let state_clone = self.state.clone();
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut state = state_clone.lock().unwrap();
                if !state.is_recording {
                    return;
                }
                if data.is_empty() {
                    return;
                }
                state.recording_samples_accum += data.len();
                
                let mut sum_sq = 0.0;
                let mut peak = 0.0_f32;
                let mut active_samples = 0usize;
                let activity_gate = SILENCE_THRESHOLD * 0.3;
                for &sample in data {
                    state.audio_buffer.push(sample);
                    sum_sq += sample * sample;
                    let abs = sample.abs();
                    if abs > peak {
                        peak = abs;
                    }
                    if abs > activity_gate {
                        active_samples += 1;
                    }
                }
                
                let rms = (sum_sq / data.len() as f32).sqrt();
                let activity_ratio = active_samples as f32 / data.len() as f32;

                // Adapt thresholds to live microphone noise for this recording.
                // We keep learning noise while voice has not started.
                let alpha = 0.10_f32;
                if !state.voice_started {
                    if state.noise_rms_ema == 0.0 {
                        state.noise_rms_ema = rms;
                        state.noise_peak_ema = peak;
                        state.noise_activity_ema = activity_ratio;
                    } else {
                        state.noise_rms_ema = (1.0 - alpha) * state.noise_rms_ema + alpha * rms;
                        state.noise_peak_ema = (1.0 - alpha) * state.noise_peak_ema + alpha * peak;
                        state.noise_activity_ema = (1.0 - alpha) * state.noise_activity_ema + alpha * activity_ratio;
                    }
                }

                let voice_rms_threshold = (state.noise_rms_ema * 2.2).max(SILENCE_THRESHOLD * 0.8);
                let voice_peak_threshold = (state.noise_peak_ema * 2.0).max(SILENCE_THRESHOLD * 1.4);
                let voice_activity_threshold = (state.noise_activity_ema * 2.0).max(0.02);
                let rms_hit = rms > voice_rms_threshold;
                let peak_hit = peak > voice_peak_threshold;
                let activity_hit = activity_ratio > voice_activity_threshold;
                // Require at least 2/3 signals to classify as voice.
                let voice_hits = (rms_hit as u8) + (peak_hit as u8) + (activity_hit as u8);
                let is_voice_chunk = voice_hits >= 2;

                // Keep adapting background estimate when chunk is not considered voice.
                if !is_voice_chunk {
                    state.noise_rms_ema = (1.0 - alpha) * state.noise_rms_ema + alpha * rms;
                    state.noise_peak_ema = (1.0 - alpha) * state.noise_peak_ema + alpha * peak;
                    state.noise_activity_ema =
                        (1.0 - alpha) * state.noise_activity_ema + alpha * activity_ratio;
                }

                if is_voice_chunk {
                    if !state.voice_started {
                        eprintln!(
                            "[audio-paste][audio][debug] voice_start rms={:.6}>{:.6} peak={:.6}>{:.6} activity_ratio={:.4}>{:.4}",
                            rms,
                            voice_rms_threshold,
                            peak,
                            voice_peak_threshold,
                            activity_ratio,
                            voice_activity_threshold
                        );
                    }
                    state.voice_started = true;
                    state.silent_samples_accum = 0;
                } else if state.voice_started && !state.finalizing {
                    state.silent_samples_accum += data.len();
                    let silence_target_samples = (SILENCE_SECONDS as usize) * (SAMPLE_RATE as usize);
                    if state.silent_samples_accum >= silence_target_samples {
                        state.finalizing = true;
                        state.is_recording = false;
                        eprintln!(
                            "[audio-paste][audio] silence detected -> auto-stop (buffer_samples={}, rms={:.6}, peak={:.6}, activity_ratio={:.4}, silent_samples={})",
                            state.audio_buffer.len(),
                            rms,
                            peak,
                            activity_ratio,
                            state.silent_samples_accum
                        );
                        let _ = silence_tx.send(());
                    }
                }

                // If no voice was ever detected, don't keep listening forever.
                let no_voice_timeout_samples = (SAMPLE_RATE as usize) * ((SILENCE_SECONDS as usize) + 2);
                if !state.voice_started && !state.finalizing && state.recording_samples_accum >= no_voice_timeout_samples {
                    state.finalizing = true;
                    state.is_recording = false;
                    eprintln!(
                        "[audio-paste][audio] no voice detected for {:.2}s -> auto-stop",
                        state.recording_samples_accum as f32 / SAMPLE_RATE as f32
                    );
                    let _ = silence_tx.send(());
                }

                // Periodic debug snapshot (~1s) to inspect why auto-stop is not firing.
                state.debug_samples_accum += data.len();
                if state.debug_samples_accum >= SAMPLE_RATE as usize {
                    let silent_secs = state.silent_samples_accum as f32 / SAMPLE_RATE as f32;
                    let buffer_secs = state.audio_buffer.len() as f32 / SAMPLE_RATE as f32;
                    eprintln!(
                        "[audio-paste][audio][debug] chunk rms={:.6}/{:.6} peak={:.6}/{:.6} activity_ratio={:.4}/{:.4} hits=[rms:{} peak:{} act:{}] is_voice={} voice_started={} silent_secs={:.2}/{:.2} buffer_secs={:.2}",
                        rms,
                        voice_rms_threshold,
                        peak,
                        voice_peak_threshold,
                        activity_ratio,
                        voice_activity_threshold,
                        rms_hit,
                        peak_hit,
                        activity_hit,
                        is_voice_chunk,
                        state.voice_started,
                        silent_secs,
                        SILENCE_SECONDS as f32,
                        buffer_secs
                    );
                    state.debug_samples_accum = 0;
                }

                // Never get stuck in recording forever due detector edge cases.
                let max_recording_seconds: usize = 30;
                let max_recording_samples = (SAMPLE_RATE as usize) * max_recording_seconds;
                if !state.finalizing && state.recording_samples_accum >= max_recording_samples {
                    state.finalizing = true;
                    state.is_recording = false;
                    eprintln!(
                        "[audio-paste][audio] max recording timeout ({:.2}s) -> auto-stop",
                        state.recording_samples_accum as f32 / SAMPLE_RATE as f32
                    );
                    let _ = silence_tx.send(());
                }
            },
            |err| eprintln!("an error occurred on stream: {}", err),
            None,
        ).map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        Box::leak(Box::new(stream));
        
        Ok(())
    }

    pub fn start_recording(&self) {
        let mut state = self.state.lock().unwrap();
        state.audio_buffer.clear();
        state.finalizing = false;
        state.voice_started = false;
        state.silent_samples_accum = 0;
        state.debug_samples_accum = 0;
        state.recording_samples_accum = 0;
        state.noise_rms_ema = 0.0;
        state.noise_peak_ema = 0.0;
        state.noise_activity_ema = 0.0;
        state.is_recording = true;
        eprintln!("[audio-paste][audio] start_recording");
    }

    pub fn stop_recording(&self) {
        let mut state = self.state.lock().unwrap();
        let captured = state.audio_buffer.len();
        state.is_recording = false;
        state.finalizing = false;
        state.voice_started = false;
        state.silent_samples_accum = 0;
        state.debug_samples_accum = 0;
        state.recording_samples_accum = 0;
        state.noise_rms_ema = 0.0;
        state.noise_peak_ema = 0.0;
        state.noise_activity_ema = 0.0;
        eprintln!(
            "[audio-paste][audio] stop_recording (captured_samples={})",
            captured
        );
    }

    pub fn get_and_clear_audio(&self) -> Vec<f32> {
        let mut state = self.state.lock().unwrap();
        let buf = state.audio_buffer.clone();
        state.audio_buffer.clear();
        buf
    }

    pub fn trim_silence(audio: &[f32], threshold: f32) -> Vec<f32> {
        let mut start_idx = 0;
        let mut end_idx = audio.len();

        for (i, &sample) in audio.iter().enumerate() {
            if sample.abs() > threshold {
                start_idx = i;
                break;
            }
        }
        for (i, &sample) in audio.iter().enumerate().rev() {
            if sample.abs() > threshold {
                end_idx = i + 1;
                break;
            }
        }

        if start_idx >= end_idx {
            return audio.to_vec();
        }
        audio[start_idx..end_idx].to_vec()
    }

    pub fn write_temp_wav(audio: &[f32]) -> Result<String, String> {
        let spec = WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: HoundSampleFormat::Int,
        };
        
        let path = std::env::temp_dir().join(format!("audio_paste_{}.wav", std::process::id()));
        let path_str = path.to_str().unwrap().to_string();
        
        let mut writer = WavWriter::create(&path, spec).map_err(|e| e.to_string())?;
        
        for &sample in audio {
            let amplitude = (sample * 32768.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(amplitude).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
        
        Ok(path_str)
    }
}
