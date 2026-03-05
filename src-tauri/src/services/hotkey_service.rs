use evdev::{Device, InputEventKind, Key};
use std::collections::HashSet;
use std::fs;
use std::sync::mpsc;
use std::thread;

pub fn spawn_evdev_hotkey_listener(tx: mpsc::Sender<()>) {
    thread::spawn(move || {
        eprintln!("[audio-paste] Starting evdev hotkey listener for Wayland");

        let keyboards = find_keyboard_devices();
        if keyboards.is_empty() {
            eprintln!("[audio-paste] No keyboard devices found in /dev/input/");
            eprintln!("[audio-paste] Tip: add your user to the 'input' group: sudo usermod -aG input $USER");
            return;
        }

        eprintln!("[audio-paste] Monitoring {} keyboard device(s)", keyboards.len());

        let mut handles = Vec::new();
        for path in keyboards {
            let tx_clone = tx.clone();
            let handle = thread::spawn(move || {
                monitor_device(&path, tx_clone);
            });
            handles.push(handle);
        }

        for h in handles {
            let _ = h.join();
        }
    });
}

fn find_keyboard_devices() -> Vec<String> {
    let mut devices = Vec::new();
    let entries = match fs::read_dir("/dev/input/") {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[audio-paste] Cannot read /dev/input/: {}", e);
            return devices;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if !name.starts_with("event") {
            continue;
        }

        match Device::open(&path) {
            Ok(dev) => {
                let keys = dev.supported_keys();
                let has_keyboard_keys = keys
                    .map(|k| k.contains(Key::KEY_A) && k.contains(Key::KEY_Z) && k.contains(Key::KEY_LEFTCTRL))
                    .unwrap_or(false);

                if has_keyboard_keys {
                    eprintln!("[audio-paste] Found keyboard: {} ({})",
                        dev.name().unwrap_or("unknown"), path.display());
                    devices.push(path.to_string_lossy().to_string());
                }
            }
            Err(_) => {}
        }
    }
    devices
}

fn monitor_device(path: &str, tx: mpsc::Sender<()>) {
    let mut dev = match Device::open(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[audio-paste] Cannot open {}: {}", path, e);
            return;
        }
    };

    let mut pressed: HashSet<Key> = HashSet::new();

    loop {
        match dev.fetch_events() {
            Ok(events) => {
                for event in events {
                    if let InputEventKind::Key(key) = event.kind() {
                        match event.value() {
                            1 => { pressed.insert(key); }
                            0 => { pressed.remove(&key); }
                            _ => {}
                        }

                        if pressed.contains(&Key::KEY_LEFTCTRL)
                            && pressed.contains(&Key::KEY_LEFTALT)
                            && pressed.contains(&Key::KEY_R)
                        {
                            eprintln!("[audio-paste] evdev: Ctrl+Alt+R detected");
                            let _ = tx.send(());
                            pressed.clear();
                        }

                        if pressed.contains(&Key::KEY_RIGHTCTRL)
                            && pressed.contains(&Key::KEY_RIGHTALT)
                            && pressed.contains(&Key::KEY_R)
                        {
                            eprintln!("[audio-paste] evdev: Ctrl+Alt+R detected (right modifiers)");
                            let _ = tx.send(());
                            pressed.clear();
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[audio-paste] evdev read error on {}: {}", path, e);
                thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }
}
