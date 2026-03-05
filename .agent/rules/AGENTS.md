# Agent Instructions

## Core Rules

- **Strict Directory Structure**: Adhere strictly to the established layout. No files outside their designated locations.
  - `src/config/` — configuration only
  - `src/constants/` — fixed identifiers, protocol constants
  - `src/services/` — business logic
  - `src/controllers/` — mediators between view and services
  - `src/views/` — rendering, no logic
  - `src/styles/` — all CSS
  - `src-tauri/src/config/` — Rust config
  - `src-tauri/src/constants/` — Rust constants
  - `src-tauri/src/services/` — Rust business logic
  - `src-tauri/src/controllers/` — Tauri commands
- **No Commented Code**: Zero commented-out snippets. Only document non-obvious logic.
- **Keep it Simple**: Minimal, concise, clean. No over-engineering.

---

## Tauri Best Practices

### Architecture

- **Frontend is a thin shell**: Views render, controllers mediate, services call `invoke()`. No business logic in the browser.
- **Backend owns all logic**: Audio capture, transcription, file I/O, clipboard — all in Rust. Frontend only displays results and sends user actions.
- **IPC is the boundary**: Frontend communicates via `invoke()` (commands) and `listen()` (events). No shared state across the boundary.
- **Commands return `Result<T, String>`**: Every `#[tauri::command]` must return `Result`. No panics, no `unwrap()` in command handlers.
- **Events for backend→frontend**: Use `app.emit("event_name", payload)` for async notifications (silence detected, status changes, transcription results).

### State Management

- **`tauri::State<AppState>`** for shared state. Always wrap in `Mutex`.
- **Lock scope must be minimal**: Acquire lock, extract data, drop lock before doing work.
- **No `.unwrap()` on locks**: Use `.map_err()` to convert `PoisonError` into a user-facing `String`.

### Plugin Usage

- Register plugins in `tauri::Builder` chain.
- Declare required permissions in `src-tauri/capabilities/default.json`.
- If a plugin is platform-specific, gate it with `#[cfg(target_os = "...")]`.

### Error Handling (Rust)

- **No `panic!()` in runtime code**. Panics are only acceptable in `GLOBAL_CONFIG` initialization (fail-fast at startup).
- **No `unwrap()` or `expect()` inside command handlers**. Use `?` with `.map_err()`.
- **All errors must propagate to frontend** with descriptive messages.
- **Use `eprintln!("[audio-paste] ...")` for debug logging** with the `[audio-paste]` prefix.

### Error Handling (Frontend)

- **Every `await invoke(...)` must be in a `try/catch`**.
- **Use `console.debug("[tag] ...")` for tracing** IPC calls and events.
- **State must reset on error**: `finally` blocks to clear `isRecording`, `isFinalizing`, etc.

---

## Cross-Platform Rules (STRICT)

### Target Platforms

All features must work on: **macOS**, **Windows**, **Linux (X11 + Wayland)**.

### Conditional Compilation

- Use `#[cfg(target_os = "linux")]`, `#[cfg(target_os = "macos")]`, `#[cfg(target_os = "windows")]` for platform-specific code.
- Use `#[cfg(desktop)]` for desktop-only features (shortcuts, tray).
- Platform-specific dependencies go in `[target.'cfg(target_os = "...")'.dependencies]` in `Cargo.toml`.
- **Never assume X11 on Linux**. Always check `WAYLAND_DISPLAY` / `XDG_SESSION_TYPE` at runtime.

### Keyboard Simulation

| Platform | Method |
|---|---|
| macOS | `enigo` (CGEvent) — Cmd+V |
| Windows | `enigo` (SendInput) — Ctrl+V |
| Linux X11 | `xdotool type` → enigo fallback |
| Linux Wayland | `wtype` → `ydotool` fallback |

- **Never use `enigo::Keyboard::text()` on Linux** — it fails on most setups.
- **Prefer direct typing** (`wtype --`, `xdotool type --`) over clipboard+paste on Linux.
- **On macOS use `Key::Meta`**, on Windows/Linux use `Key::Control`** for paste shortcut.

### Global Shortcuts

| Platform | Method |
|---|---|
| macOS / Windows | `tauri-plugin-global-shortcut` (native) |
| Linux X11 | `tauri-plugin-global-shortcut` (XGrabKey) |
| Linux Wayland | `evdev` crate (reads `/dev/input/` directly) |

- `global-hotkey` crate is **X11-only**. Do not rely on it for Wayland.
- On Wayland, the evdev listener requires the user to be in the `input` group.
- Always log registration success/failure at startup.

### Clipboard

| Platform | Method |
|---|---|
| macOS / Windows / Linux X11 | `arboard` crate |
| Linux Wayland | `wl-copy` command |

### Audio

- `cpal` crate works on all platforms. No platform-specific audio code needed.
- Always request mono, 16kHz.

---

## Absolute Prohibitions

- **No `unwrap()` in command handlers or services** (except `GLOBAL_CONFIG` init).
- **No hardcoded paths** — resolve from executable location or config.
- **No platform assumptions** — if code touches keyboard/clipboard/shortcuts, it must handle all 4 targets.
- **No silent failures** — every error must log and propagate.
- **No runtime config mutation** — config is immutable after startup.
- **No business logic in views or controllers**.
- **No `unsafe` code** without explicit justification.

---

## Execution Warning

**DO NOT RUN THE APPLICATION** unless explicitly told to.
No `cargo run`, no `npx tauri dev`, no test commands without permission.
