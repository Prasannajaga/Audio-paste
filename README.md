# Audio paste

![Status](https://img.shields.io/badge/status-in%20development-yellow)
![Framework](https://img.shields.io/badge/framework-PyQt-blue)
![Python](https://img.shields.io/badge/python-3.x-blue)

---

## 📖 About

this is project inspired from superWhisper and paraspeech

this is developed soley by me to understand how it works under the hood

here I have used the faster-whisper sdk which provide openAI whisper speech to text model in many varieites

this is developed on pyQt python GUI framework to get the full use of faster-whisper sdk or any custom model I currently working on

this app is still in development phase, but the core functionality is working as expected

might switch to Tauri which supports better cross pplatform

---

## 🎙️ Usage

### Keyboard Shortcut

Press **`Ctrl + Alt + R`** to trigger voice recording

- The app will start listening when you press the shortcut
- Speak your text
- The app automatically detects silence and stops recording
- Transcribed text is automatically pasted at your cursor position

### Features

- **Global Hotkey**: Works even when the app is minimized or in background
- **Auto-paste**: Transcribed text is automatically typed where your cursor is
- **Configurable Models**: Choose from multiple whisper model sizes
- **Device Selection**: Run on CPU or GPU (CUDA)
- **Adjustable CPU Threads**: Optimize performance based on your system

---
