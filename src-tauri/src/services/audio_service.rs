use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use hound::{WavSpec, WavWriter, SampleFormat as HoundSampleFormat};
use crate::constants::config::{SAMPLE_RATE, SILENCE_THRESHOLD, SILENCE_SECONDS};

pub struct AudioState {
    pub is_recording: bool,
    pub audio_buffer: Vec<f32>,
    pub last_voice_time: Option<Instant>,
    pub finalizing: bool,
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
                last_voice_time: None,
                finalizing: false,
            })),
        }
    }

    pub fn start_listening(&mut self, silence_tx: std::sync::mpsc::Sender<()>) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or("Failed to get default input device")?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let state_clone = self.state.clone();
        let state_monitor = self.state.clone();
        
        // Spawn monitor thread
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(100));
                let mut state = state_monitor.lock().unwrap();
                
                if !state.is_recording || state.finalizing || state.last_voice_time.is_none() {
                    continue;
                }
                
                if state.last_voice_time.unwrap().elapsed().as_secs_f32() >= SILENCE_SECONDS as f32 {
                    state.finalizing = true;
                    state.last_voice_time = None;
                    state.is_recording = false;
                    let _ = silence_tx.send(());
                }
            }
        });

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut state = state_clone.lock().unwrap();
                if !state.is_recording {
                    return;
                }
                
                let mut sum_sq = 0.0;
                for &sample in data {
                    state.audio_buffer.push(sample);
                    sum_sq += sample * sample;
                }
                
                let rms = (sum_sq / data.len() as f32).sqrt();
                if rms > SILENCE_THRESHOLD {
                    state.last_voice_time = Some(Instant::now());
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
        state.last_voice_time = None;
        state.finalizing = false;
        state.is_recording = true;
    }

    pub fn stop_recording(&self) {
        let mut state = self.state.lock().unwrap();
        state.is_recording = false;
        state.finalizing = false;
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
