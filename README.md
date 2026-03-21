# Audio Paste

Offline voice-to-text desktop app built with **Tauri + Rust + Vanilla TypeScript**.

You press a hotkey (or click), speak, and the app transcribes locally with `whisper.cpp` and pastes the text into the focused app.

## What It Does

- Captures microphone audio at 16kHz
- Detects speech/silence and auto-stops recording
- Runs local transcription using `whisper-cli`
- Copies transcript to clipboard and tries auto-paste (`Ctrl+V` / `Cmd+V`)
- Supports Linux, macOS, and Windows (with OS-specific hotkey/paste paths)

## Tech Stack

- Frontend: Vanilla TypeScript + Vite
- Desktop runtime: Tauri v2
- Backend: Rust
- Speech engine: `whisper.cpp` (`whisper-cli` binary)

## Project Structure

- `src/`: frontend UI/controller/api
- `src-tauri/src/lib.rs`: app startup, event wiring, OS-specific hotkey setup
- `src-tauri/src/controllers/commands.rs`: Tauri commands (`start_recording`, `stop_recording`, `process_transcription`, `apply_config`)
- `src-tauri/src/services/audio_service.rs`: input stream, speech/silence logic, WAV writing
- `src-tauri/src/services/transcription_service.rs`: `whisper-cli` execution + parsing
- `src-tauri/src/services/clipboard_service.rs`: clipboard + paste simulation
- `src-tauri/src/config/whisper_config.rs`: model/binary discovery + model auto-download

## Architecture Diagram

> System-level overview — how the frontend, backend, and OS layers connect.

```mermaid
%%{init: {
  "theme": "base",
  "themeVariables": {
    "primaryColor":       "#1e1e2e",
    "primaryTextColor":   "#cdd6f4",
    "primaryBorderColor": "#585b70",
    "secondaryColor":     "#313244",
    "tertiaryColor":      "#45475a",
    "lineColor":          "#89b4fa",
    "textColor":          "#cdd6f4",
    "fontSize":           "14px"
  }
}}%%

flowchart TB
  USER(["🎤 User"]):::userNode

  subgraph FRONTEND["🖥️  Frontend — TypeScript + Vite"]
    direction LR
    CTRL["AppController\n─────────\nstate machine\nuser input routing"]:::feNode
    VIEW["MainView\n─────────\nDOM rendering\nstatus display\naudio feedback"]:::feNode
    API["ApiService\n─────────\ninvoke() wrapper\nevent listeners"]:::feNode

    CTRL -- "render" --> VIEW
    CTRL -- "IPC calls" --> API
  end

  subgraph IPC_LAYER["⚡ Tauri IPC Boundary"]
    direction LR
    CMD["Commands\n─────────\nstart_recording\nstop_recording\nprocess_transcription\napply_config"]:::ipcNode
  end

  subgraph BACKEND["⚙️  Backend — Rust"]
    direction TB
    subgraph SERVICES["Services"]
      direction LR
      AUDIO["AudioService\n─────────\ncpal capture\nvoice detection\nsilence tracking"]:::beNode
      TRANS["TranscriptionService\n─────────\nspawn whisper-cli\nparse output"]:::beNode
      CLIP["ClipboardService\n─────────\narboard · wl-copy\npaste simulation"]:::beNode
      HOTKEY["HotkeyService\n─────────\nevdev (Wayland)\nXGrabKey (X11)"]:::beNode
    end
    subgraph CONFIG["Configuration"]
      direction LR
      ENV["AppConfig\n─────────\nenv detection\nvalidation\nimmutable state"]:::cfgNode
      WCONF["WhisperConfig\n─────────\nbinary discovery\nmodel download\npath resolution"]:::cfgNode
    end
  end

  subgraph OS_LAYER["🐧 🍎 🪟  Operating System"]
    direction LR
    MIC(["Microphone"]):::osNode
    WCLI(["whisper-cli"]):::osNode
    CLIPBOARD(["Clipboard\n+ Key Injection"]):::osNode
  end

  USER -- "Click · Ctrl+Alt+R" --> CTRL
  API -- "invoke()" --> CMD
  CMD --> AUDIO & TRANS & CLIP
  CMD -- "read" --> WCONF
  AUDIO -. "silence_detected\nevent" .-> CTRL
  CMD -. "transcription_result\nstatus_change" .-> CTRL

  AUDIO --> MIC
  TRANS --> WCLI
  CLIP --> CLIPBOARD
  WCONF -- "resolve binary\ndownload model" --> WCLI
  ENV  -- "provides config" --> AUDIO

  classDef userNode   fill:#f38ba8,stroke:#f38ba8,color:#1e1e2e,stroke-width:2px,font-weight:bold
  classDef feNode     fill:#1e1e2e,stroke:#89b4fa,color:#89b4fa,stroke-width:2px
  classDef ipcNode    fill:#1e1e2e,stroke:#f9e2af,color:#f9e2af,stroke-width:2px,stroke-dasharray:5 5
  classDef beNode     fill:#1e1e2e,stroke:#a6e3a1,color:#a6e3a1,stroke-width:2px
  classDef cfgNode    fill:#1e1e2e,stroke:#cba6f7,color:#cba6f7,stroke-width:2px
  classDef osNode     fill:#45475a,stroke:#585b70,color:#cdd6f4,stroke-width:2px

  style FRONTEND   fill:#181825,stroke:#89b4fa,stroke-width:2px,color:#89b4fa
  style IPC_LAYER  fill:#181825,stroke:#f9e2af,stroke-width:2px,stroke-dasharray:5 5,color:#f9e2af
  style BACKEND    fill:#181825,stroke:#a6e3a1,stroke-width:2px,color:#a6e3a1
  style SERVICES   fill:#1e1e2e,stroke:#a6e3a1,stroke-width:1px,color:#a6e3a1
  style CONFIG     fill:#1e1e2e,stroke:#cba6f7,stroke-width:1px,color:#cba6f7
  style OS_LAYER   fill:#181825,stroke:#585b70,stroke-width:2px,color:#cdd6f4
```

---

## Application State Machine

> Every state the app can be in and what triggers transitions between them.

```mermaid
%%{init: {
  "theme": "base",
  "themeVariables": {
    "primaryColor":      "#1e1e2e",
    "primaryTextColor":  "#cdd6f4",
    "lineColor":         "#89b4fa",
    "fontSize":          "13px"
  }
}}%%

stateDiagram-v2
  direction LR

  [*] --> LOADING : App launches

  LOADING --> IDLE : apply_config OK
  LOADING --> IDLE : apply_config FAIL ⚠️

  IDLE --> LISTENING : User click / Ctrl+Alt+R

  LISTENING --> TRANSCRIBING : silence_detected
  LISTENING --> TRANSCRIBING : User click (manual stop)

  TRANSCRIBING --> IDLE : Transcript empty
  TRANSCRIBING --> IDLE : Transcript pasted ✅
  TRANSCRIBING --> IDLE : Transcription error ⚠️

  IDLE --> LOADING : Settings → Apply

  note right of LOADING
    WhisperConfig resolves binary,
    downloads model if missing
  end note

  note right of LISTENING
    cpal mic stream captures audio,
    adaptive silence detector runs
  end note

  note right of TRANSCRIBING
    Worker thread spawns whisper-cli,
    result → clipboard + paste
  end note
```

---

## Complete Runtime Flow

> Step-by-step sequence from startup → record → transcribe → paste.

```mermaid
%%{init: {
  "theme": "base",
  "themeVariables": {
    "primaryColor":       "#1e1e2e",
    "primaryTextColor":   "#cdd6f4",
    "primaryBorderColor": "#585b70",
    "lineColor":          "#89b4fa",
    "actorTextColor":     "#cdd6f4",
    "actorBkg":           "#313244",
    "actorBorder":        "#89b4fa",
    "activationBkgColor": "#313244",
    "activationBorderColor":"#89b4fa",
    "signalColor":        "#89b4fa",
    "noteBkgColor":       "#45475a",
    "noteTextColor":      "#cdd6f4",
    "noteBorderColor":    "#585b70",
    "altSectionBkgColor": "#181825",
    "loopTextColor":      "#f9e2af",
    "labelTextColor":     "#f38ba8",
    "fontSize":           "13px"
  }
}}%%

sequenceDiagram
  autonumber

  actor User as 🎤 User
  participant UI   as 🖥️ AppController
  participant API  as ⬡ ApiService
  participant CMD  as ⚡ Tauri Commands
  participant Audio as 🔊 AudioService
  participant WConf as 📦 WhisperConfig
  participant WCLI  as 🤖 whisper-cli
  participant Clip  as 📋 ClipboardService

  rect rgb(24, 24, 37)
    Note over UI, WConf: 🔧 STARTUP — Configuration & Model Resolution
    UI  ->>+ API  : applyConfig(tiny, cpu, 4, 0.01, 2)
    API ->>+ CMD  : invoke("apply_config", {...})
    CMD ->>+ WConf : WhisperConfig::new()
    WConf ->> WConf : resolve whisper-cli binary
    WConf ->> WConf : stage runtime libs (LD_LIBRARY_PATH)

    alt Model file missing
      WConf ->> WConf : download from huggingface (curl / wget)
    end

    WConf -->>- CMD : WhisperConfig { cli_path, model_path, ... }
    CMD  -->>- API : Ok(())
    API  -->>- UI  : status → IDLE ✅
  end

  rect rgb(24, 24, 37)
    Note over User, Audio: 🎙️ RECORDING — Voice Capture
    User ->>  UI   : Click or Ctrl+Alt+R
    UI   ->>  UI   : playTick() 🔔
    UI   ->>+ API  : startRecording()
    API  ->>+ CMD  : invoke("start_recording")
    CMD  ->>  Audio : start_recording() → set flag
    CMD  -->>- API : Ok(())
    API  -->>- UI  : status → LISTENING 🟢

    Note over Audio: cpal mic stream (mono 16kHz) is always running
    Note over Audio: Adaptive threshold detects voice & tracks silence
  end

  rect rgb(24, 24, 37)
    Note over User, Audio: ⏹️ STOP — Silence or Manual
    alt Silence detected automatically
      Audio -->> UI : emit("silence_detected") 🔇
    else User stops manually
      User  ->>  UI : Click or Ctrl+Alt+R
    end

    UI   ->>+ API  : stopRecording()
    API  ->>+ CMD  : invoke("stop_recording")
    CMD  ->>  Audio : stop_recording() → clear flag
    CMD  -->>- API : Ok(())
    API  -->>- UI  : status → TRANSCRIBING ⏳
  end

  rect rgb(24, 24, 37)
    Note over UI, Clip: 🧠 TRANSCRIPTION — Whisper Processing
    UI   ->>+ API  : processTranscription()
    API  ->>+ CMD  : invoke("process_transcription")
    CMD  ->>  Audio : take_audio_buffer() (Mutex + mem::take)
    CMD  ->>  CMD  : trim leading/trailing silence
    CMD  ->>  CMD  : write temp WAV (/tmp/audio-paste-*.wav)

    CMD  ->>+ WCLI : spawn whisper-cli -m <model> -f <wav>
    WCLI -->>- CMD : raw stdout

    CMD  ->>  CMD  : parse & clean transcript text

    alt Transcript is non-empty
      CMD  -->> UI  : emit("transcription_result", text) 📝
      CMD  ->>+ Clip : paste_text(text)
      Clip ->>  Clip : set clipboard (arboard / wl-copy)
      Clip ->>  Clip : simulate Ctrl+V / Cmd+V
      Clip -->>- CMD : done
    else Transcript is empty
      CMD  -->> UI  : emit("status_change", "IDLE") 💤
    end

    CMD  ->>  CMD  : remove temp WAV 🗑️
    CMD  -->>- API : Ok("")
    API  -->>- UI  : status → IDLE 🟢
  end
```

---

## Layered Architecture

> Clean separation of concerns across every layer of the stack.

```mermaid
%%{init: {
  "theme": "base",
  "themeVariables": {
    "primaryColor":       "#1e1e2e",
    "primaryTextColor":   "#cdd6f4",
    "primaryBorderColor": "#585b70",
    "lineColor":          "#89b4fa",
    "textColor":          "#cdd6f4",
    "fontSize":           "13px"
  }
}}%%

block-beta
  columns 4

  block:PRESENTATION:4
    columns 4
    P_TITLE["🎨 Presentation Layer\nViews + Styles"]
    P1["index.html\nDOM structure"]
    P2["MainView.ts\nrendering only"]
    P3["main.css\nall styling"]
  end

  space:4

  block:CONTROL:4
    columns 4
    C_TITLE["🕹️ Control Layer\nMediation + Routing"]
    C1["AppController.ts\nstate transitions\nuser input"]
    C2["commands.rs\n#[tauri::command]\nIPC handlers"]
    C3["lib.rs\napp bootstrap\nevent wiring"]
  end

  space:4

  block:SERVICE:4
    columns 4
    S_TITLE["⚙️ Service Layer\nBusiness Logic"]
    S1["AudioService\ncpal capture\nsilence detect"]
    S2["TranscriptionService\nwhisper-cli exec\noutput parsing"]
    S3["ClipboardService\npaste simulation\nOS adapters"]
  end

  space:4

  block:INFRA:4
    columns 4
    I_TITLE["🔧 Infrastructure Layer\nConfig + Constants"]
    I1["AppConfig (env.rs)\nenv detection\nvalidation\nimmutable"]
    I2["WhisperConfig\nbinary discovery\nmodel download"]
    I3["constants/\ndefault values\nprotocol IDs"]
  end

  PRESENTATION --> CONTROL
  CONTROL --> SERVICE
  SERVICE --> INFRA

  style PRESENTATION fill:#181825,stroke:#89b4fa,stroke-width:2px,color:#89b4fa
  style CONTROL       fill:#181825,stroke:#f9e2af,stroke-width:2px,color:#f9e2af
  style SERVICE        fill:#181825,stroke:#a6e3a1,stroke-width:2px,color:#a6e3a1
  style INFRA          fill:#181825,stroke:#cba6f7,stroke-width:2px,color:#cba6f7

  style P_TITLE fill:#1e1e2e,stroke:none,color:#89b4fa
  style P1      fill:#313244,stroke:#585b70,color:#cdd6f4
  style P2      fill:#313244,stroke:#585b70,color:#cdd6f4
  style P3      fill:#313244,stroke:#585b70,color:#cdd6f4

  style C_TITLE fill:#1e1e2e,stroke:none,color:#f9e2af
  style C1      fill:#313244,stroke:#585b70,color:#cdd6f4
  style C2      fill:#313244,stroke:#585b70,color:#cdd6f4
  style C3      fill:#313244,stroke:#585b70,color:#cdd6f4

  style S_TITLE fill:#1e1e2e,stroke:none,color:#a6e3a1
  style S1      fill:#313244,stroke:#585b70,color:#cdd6f4
  style S2      fill:#313244,stroke:#585b70,color:#cdd6f4
  style S3      fill:#313244,stroke:#585b70,color:#cdd6f4

  style I_TITLE fill:#1e1e2e,stroke:none,color:#cba6f7
  style I1      fill:#313244,stroke:#585b70,color:#cdd6f4
  style I2      fill:#313244,stroke:#585b70,color:#cdd6f4
  style I3      fill:#313244,stroke:#585b70,color:#cdd6f4
```

---

## Platform Compatibility

> How each OS-specific concern is handled across targets.

| Capability | 🐧 Linux (X11) | 🐧 Linux (Wayland) | 🍎 macOS | 🪟 Windows |
| :-: | :-: | :-: | :-: | :-: |
| **Audio Capture** | `cpal` | `cpal` | `cpal` | `cpal` |
| **Global Hotkey** | `tauri-plugin` / XGrabKey | `evdev` / /dev/input | `tauri-plugin` / native | `tauri-plugin` / native |
| **Clipboard** | `arboard` | `wl-copy` | `arboard` | `arboard` |
| **Paste Sim** | `xdotool key` → enigo | `wtype` → `ydotool` | `enigo` Cmd+V | `enigo` Ctrl+V |
| **Typing** | `xdotool type` | `wtype --` | ✘ (use paste) | ✘ (use paste) |
| **Model Download** | `curl` → `wget` | `curl` → `wget` | `curl` → `wget` | `curl` → `wget` |

## How Configuration Works

Startup config is loaded from:

1. Hardcoded defaults (`tiny.en`, `cpu`, `4 threads`)
2. `APP_ENV` (`development` / `testing` / `production`)
3. Optional user file: `~/.audio-paste/config.json`

Example user config:

```json
{
  "model_size": "tiny.en",
  "device": "cpu",
  "cpu_threads": 4
}
```

## Setup

### Linux (Build)

```bash
npm install
cd src-tauri/whisper.cpp
cmake -S . -B build
cmake --build build --config Release -j"$(nproc)"
cd ../..
npm run build
npx tauri build
```

### macOS (Build)

```bash
npm install
cd src-tauri/whisper.cpp
cmake -S . -B build
cmake --build build --config Release -j"$(sysctl -n hw.ncpu)"
cd ../..
npm run build
npx tauri build
```

### Windows (Build) (PowerShell)

```powershell
npm install
cd src-tauri/whisper.cpp
cmake -S . -B build
cmake --build build --config Release
cd ../..
npm run build
npx tauri build
```
