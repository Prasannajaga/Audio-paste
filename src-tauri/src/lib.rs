pub mod config;
pub mod constants;
pub mod controllers;
pub mod services;

use std::sync::Mutex;
use tauri::Emitter;

use crate::config::env::GLOBAL_CONFIG;
use crate::config::whisper_config::WhisperConfig;
use crate::services::audio_service::AudioService;
use crate::controllers::commands::{AppState, start_recording, stop_recording, process_transcription, apply_config};

#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v == "wayland")
            .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _cfg = GLOBAL_CONFIG.environment.clone();

    let root = WhisperConfig::resolve_project_root()
        .expect("CRITICAL: Could not resolve project root");
    eprintln!("[audio-paste] Project root: {:?}", root);

    let initial_whisper = WhisperConfig::new(root, &GLOBAL_CONFIG.model_size, GLOBAL_CONFIG.cpu_threads)
        .expect("CRITICAL: Failed to initialize WhisperConfig");
    eprintln!("[audio-paste] WhisperConfig initialized: {:?}", initial_whisper.cli_path);

    let mut audio_svc = AudioService::new();
    let (tx, rx) = std::sync::mpsc::channel();
    audio_svc.start_listening(tx).expect("Failed to start audio listening capture");

    let app_state = AppState {
        audio_service: Mutex::new(audio_svc),
        whisper_config: Mutex::new(Some(initial_whisper)),
    };

    let mut builder = tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_opener::init());

    // Global shortcut: use tauri plugin on macOS/Windows/X11, evdev on Wayland
    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState, Modifiers, Code};

        let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyR);
        builder = builder
            .plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, _shortcut, event| {
                        if event.state() == ShortcutState::Pressed {
                            eprintln!("[audio-paste] Global hotkey pressed");
                            let _ = app.emit("toggle_recording", ());
                        }
                    })
                    .build(),
            );
        // Need to capture shortcut for setup closure
        let shortcut_for_setup = shortcut;
        builder = builder.setup(move |app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for _ in rx {
                    eprintln!("[audio-paste] Silence detected");
                    let _ = handle.emit("silence_detected", ());
                }
            });

            match app.global_shortcut().register(shortcut_for_setup) {
                Ok(_) => eprintln!("[audio-paste] Registered global shortcut Ctrl+Alt+R"),
                Err(e) => eprintln!("[audio-paste] Failed to register shortcut: {}", e),
            }

            Ok(())
        });
    }

    #[cfg(target_os = "linux")]
    {
        let use_evdev = is_wayland();

        if use_evdev {
            eprintln!("[audio-paste] Wayland detected: using evdev for global hotkey");
            builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());

            let (hotkey_tx, hotkey_rx) = std::sync::mpsc::channel();
            services::hotkey_service::spawn_evdev_hotkey_listener(hotkey_tx);

            builder = builder.setup(move |app| {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    for _ in rx {
                        eprintln!("[audio-paste] Silence detected");
                        let _ = handle.emit("silence_detected", ());
                    }
                });

                let handle2 = app.handle().clone();
                std::thread::spawn(move || {
                    for _ in hotkey_rx {
                        eprintln!("[audio-paste] evdev hotkey → emitting toggle_recording");
                        let _ = handle2.emit("toggle_recording", ());
                    }
                });

                Ok(())
            });
        } else {
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState, Modifiers, Code};

            eprintln!("[audio-paste] X11 detected: using tauri global shortcut plugin");
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyR);
            builder = builder
                .plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, _shortcut, event| {
                            if event.state() == ShortcutState::Pressed {
                                eprintln!("[audio-paste] Global hotkey pressed (X11)");
                                let _ = app.emit("toggle_recording", ());
                            }
                        })
                        .build(),
                );

            builder = builder.setup(move |app| {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    for _ in rx {
                        eprintln!("[audio-paste] Silence detected");
                        let _ = handle.emit("silence_detected", ());
                    }
                });

                match app.global_shortcut().register(shortcut) {
                    Ok(_) => eprintln!("[audio-paste] Registered Ctrl+Alt+R (X11)"),
                    Err(e) => eprintln!("[audio-paste] Shortcut registration failed: {}", e),
                }

                Ok(())
            });
        }
    }

    builder
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            process_transcription,
            apply_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
