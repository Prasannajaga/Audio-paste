# Audio paste

![Status](https://img.shields.io/badge/status-in%20development-yellow)
![Framework](https://img.shields.io/badge/framework-PyQt-blue)
![Python](https://img.shields.io/badge/python-3.x-blue)

---

## 📖 About

this project is inspired from superWhisper and paraspeech, developed soley by me to understand how things work under the hood, using the faster-whisper sdk which provides the openAI whisper speech to text model in many varieites, built with the pyQt python GUI framework to fully utilize the faster-whisper sdk or any custom model

I am currently working on, the app is still in the development phase but the core functionality is working as expected, and I might switch to Tauri later since it supports better cross pplatform

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
