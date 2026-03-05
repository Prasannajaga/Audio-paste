use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v == "wayland")
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn check_tool(name: &str) -> bool {
    Command::new("which").arg(name).output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn wl_copy(text: &str) -> Result<(), String> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("wl-copy spawn: {}. Install: sudo apt install wl-clipboard", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())
            .map_err(|e| format!("wl-copy stdin write: {}", e))?;
    }

    let status = child.wait().map_err(|e| format!("wl-copy wait: {}", e))?;
    if !status.success() {
        return Err(format!("wl-copy exited with {}", status));
    }
    eprintln!("[audio-paste] wl-copy OK (clipboard persisted)");
    Ok(())
}

#[cfg(target_os = "linux")]
fn uinput_ctrl_v() -> Result<(), String> {
    use evdev::uinput::VirtualDeviceBuilder;
    use evdev::{AttributeSet, EventType, InputEvent, Key};

    let mut keys = AttributeSet::<Key>::new();
    keys.insert(Key::KEY_LEFTCTRL);
    keys.insert(Key::KEY_V);

    let mut device = VirtualDeviceBuilder::new()
        .map_err(|e| format!("uinput open: {}. Run: sudo chmod 660 /dev/uinput && sudo chown root:input /dev/uinput", e))?
        .name("audio-paste-kbd")
        .with_keys(&keys)
        .map_err(|e| format!("uinput keys: {}", e))?
        .build()
        .map_err(|e| format!("uinput build: {}", e))?;

    thread::sleep(Duration::from_millis(200));

    let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);

    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTCTRL.0, 1), syn])
        .map_err(|e| format!("emit: {}", e))?;
    thread::sleep(Duration::from_millis(20));

    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_V.0, 1), syn])
        .map_err(|e| format!("emit: {}", e))?;
    thread::sleep(Duration::from_millis(20));

    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_V.0, 0), syn])
        .map_err(|e| format!("emit: {}", e))?;
    thread::sleep(Duration::from_millis(20));

    device.emit(&[InputEvent::new(EventType::KEY, Key::KEY_LEFTCTRL.0, 0), syn])
        .map_err(|e| format!("emit: {}", e))?;

    eprintln!("[audio-paste] uinput Ctrl+V sent");
    Ok(())
}

#[cfg(target_os = "macos")]
fn simulate_paste(text: &str) -> Result<(), String> {
    use arboard::Clipboard;
    let mut ctx = Clipboard::new().map_err(|e| format!("Clipboard: {}", e))?;
    ctx.set_text(text).map_err(|e| format!("Clipboard set: {}", e))?;
    thread::sleep(Duration::from_millis(100));

    use enigo::{Enigo, Settings, Keyboard, Key, Direction};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo: {}", e))?;
    enigo.key(Key::Meta, Direction::Press).map_err(|e| format!("{}", e))?;
    thread::sleep(Duration::from_millis(30));
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| format!("{}", e))?;
    thread::sleep(Duration::from_millis(30));
    enigo.key(Key::Meta, Direction::Release).map_err(|e| format!("{}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn simulate_paste(text: &str) -> Result<(), String> {
    use arboard::Clipboard;
    let mut ctx = Clipboard::new().map_err(|e| format!("Clipboard: {}", e))?;
    ctx.set_text(text).map_err(|e| format!("Clipboard set: {}", e))?;
    thread::sleep(Duration::from_millis(100));

    use enigo::{Enigo, Settings, Keyboard, Key, Direction};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo: {}", e))?;
    enigo.key(Key::Control, Direction::Press).map_err(|e| format!("{}", e))?;
    thread::sleep(Duration::from_millis(30));
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| format!("{}", e))?;
    thread::sleep(Duration::from_millis(30));
    enigo.key(Key::Control, Direction::Release).map_err(|e| format!("{}", e))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn simulate_paste(text: &str) -> Result<(), String> {
    if is_wayland() {
        return wayland_paste(text);
    }
    x11_paste(text)
}

#[cfg(target_os = "linux")]
fn wayland_paste(text: &str) -> Result<(), String> {
    // Step 1: Set clipboard (wl-copy persists via daemon, unlike arboard)
    if check_tool("wl-copy") {
        eprintln!("[audio-paste] Setting clipboard via wl-copy");
        wl_copy(text)?;
    } else {
        eprintln!("[audio-paste] wl-copy not found, using arboard (may be unreliable)");
        use arboard::Clipboard;
        let mut ctx = Clipboard::new().map_err(|e| format!("Clipboard: {}", e))?;
        ctx.set_text(text).map_err(|e| format!("Clipboard set: {}", e))?;
        // Keep ctx alive — DON'T drop it before paste
        thread::sleep(Duration::from_millis(150));
        uinput_ctrl_v()?;
        thread::sleep(Duration::from_millis(300));
        drop(ctx);
        return Ok(());
    }

    thread::sleep(Duration::from_millis(100));

    // Step 2: Simulate Ctrl+V via uinput
    match uinput_ctrl_v() {
        Ok(_) => return Ok(()),
        Err(e) => eprintln!("[audio-paste] uinput failed: {}", e),
    }

    // Fallback: ydotool
    if check_tool("ydotool") {
        eprintln!("[audio-paste] Trying ydotool key ctrl+v");
        let r = Command::new("ydotool")
            .arg("key").arg("29:1").arg("47:1").arg("47:0").arg("29:0")
            .output();
        match r {
            Ok(o) if o.status.success() => return Ok(()),
            _ => {}
        }
    }

    // Fallback: xdotool via XWayland
    if check_tool("xdotool") {
        eprintln!("[audio-paste] Trying xdotool key ctrl+v");
        let r = Command::new("xdotool").arg("key").arg("ctrl+v").output();
        match r {
            Ok(o) if o.status.success() => return Ok(()),
            _ => {}
        }
    }

    Err("Ctrl+V simulation failed. Clipboard is set — paste manually with Ctrl+V.".into())
}

#[cfg(target_os = "linux")]
fn x11_paste(text: &str) -> Result<(), String> {
    if check_tool("xdotool") {
        eprintln!("[audio-paste] X11: xdotool type");
        let result = Command::new("xdotool")
            .arg("type").arg("--delay").arg("0")
            .arg("--").arg(text)
            .output();
        match result {
            Ok(o) if o.status.success() => return Ok(()),
            _ => {}
        }
    }

    use arboard::Clipboard;
    let mut ctx = Clipboard::new().map_err(|e| format!("Clipboard: {}", e))?;
    ctx.set_text(text).map_err(|e| format!("Clipboard set: {}", e))?;
    thread::sleep(Duration::from_millis(100));

    use enigo::{Enigo, Settings, Keyboard, Key, Direction};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo: {}", e))?;
    enigo.key(Key::Control, Direction::Press).map_err(|e| format!("{}", e))?;
    thread::sleep(Duration::from_millis(30));
    enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| format!("{}", e))?;
    thread::sleep(Duration::from_millis(30));
    enigo.key(Key::Control, Direction::Release).map_err(|e| format!("{}", e))?;
    drop(ctx);
    Ok(())
}

pub fn paste_text(text: &str) {
    if text.trim().is_empty() {
        return;
    }

    eprintln!("[audio-paste] paste_text: {} chars", text.len());

    if let Err(e) = simulate_paste(text) {
        eprintln!("[audio-paste] PASTE FAILED: {}", e);
    }
}
