import sys
import time
import threading
import numpy as np
import sounddevice as sd
import pyperclip
import evdev
from collections import deque
from evdev import ecodes as e, UInput
from PyQt6.QtWidgets import QApplication, QWidget, QLabel, QPushButton, QVBoxLayout
from PyQt6.QtCore import pyqtSignal, QObject
from faster_whisper import WhisperModel

# =========================
# Configuration
# =========================
HOTKEY_COMBO = {e.KEY_LEFTCTRL, e.KEY_LEFTALT, e.KEY_R}
SAMPLE_RATE = 16000
MODEL_SIZE = "tiny.en"

SILENCE_THRESHOLD = 0.01   # RMS threshold
SILENCE_SECONDS = .0      # Stop after 5s silence


# =========================
# Qt Signal Bridge
# =========================
class Signals(QObject):
    toggle = pyqtSignal()
    status = pyqtSignal(str)


class AudioPasteApp(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("AI Audio Paste")
        self.resize(360, 200)

        self.signals = Signals()
        self.signals.toggle.connect(self.toggle_recording)
        self.signals.status.connect(self.update_label)

        self.label = QLabel("Status: IDLE\nHotkey: Ctrl + Alt + R")
        self.button = QPushButton("Manual Toggle")
        self.button.clicked.connect(self.toggle_recording)

        layout = QVBoxLayout()
        layout.addWidget(self.label)
        layout.addWidget(self.button)
        self.setLayout(layout)

        print(f"Loading Whisper model: {MODEL_SIZE}")
        self.model = WhisperModel(
            MODEL_SIZE,
            device="cpu",
            compute_type="int8",
            num_workers=2
        )

        self.is_recording = False

        self.audio_buffer = deque()
        self.buffer_lock = threading.Lock()

        self.last_voice_time = None

        threading.Thread(target=self.audio_stream, daemon=True).start()
        threading.Thread(target=self.hotkey_listener, daemon=True).start()
        threading.Thread(target=self.silence_monitor, daemon=True).start()

    # =========================
    # UI
    # =========================
    def update_label(self, text):
        self.label.setText(text)

    # =========================
    # Audio
    # =========================
    def audio_callback(self, indata, frames, time_info, status):
        if not self.is_recording:
            return

        audio = indata[:, 0]
        rms = np.sqrt(np.mean(audio ** 2))

        with self.buffer_lock:
            self.audio_buffer.extend(audio)

        if rms > SILENCE_THRESHOLD:
            self.last_voice_time = time.time()

    def audio_stream(self):
        with sd.InputStream(
            samplerate=SAMPLE_RATE,
            channels=1,
            blocksize=1024,
            callback=self.audio_callback,
        ):
            while True:
                time.sleep(1)

    # =========================
    # Silence Logic
    # =========================
    def silence_monitor(self):
        while True:
            if not self.is_recording or self.last_voice_time is None:
                time.sleep(0.1)
                continue

            if time.time() - self.last_voice_time >= SILENCE_SECONDS:
                self.finalize_transcription()

            time.sleep(0.1)

    def finalize_transcription(self):
        with self.buffer_lock:
            if not self.audio_buffer:
                return

            audio = np.array(self.audio_buffer, dtype=np.float32)
            self.audio_buffer.clear()

        self.last_voice_time = None
        self.is_recording = False
        self.signals.status.emit("Status: TRANSCRIBING")

        segments, _ = self.model.transcribe(
            audio,
            beam_size=1,
            vad_filter=True,
        )

        text = "".join(s.text for s in segments).strip()
        if text:
            print("Final:", text)
            self.paste_text(text)

        self.signals.status.emit("Status: IDLE")

    # =========================
    # Paste
    # =========================
    def paste_text(self, text):
        pyperclip.copy(text + " ")
        try:
            with UInput() as ui:
                ui.write(e.EV_KEY, e.KEY_LEFTCTRL, 1)
                ui.write(e.EV_KEY, e.KEY_V, 1)
                ui.syn()
                time.sleep(0.03)
                ui.write(e.EV_KEY, e.KEY_V, 0)
                ui.write(e.EV_KEY, e.KEY_LEFTCTRL, 0)
                ui.syn()
        except Exception as err:
            print("Paste error:", err)

    # =========================
    # Hotkey
    # =========================
    def find_keyboard(self):
        for path in evdev.list_devices():
            dev = evdev.InputDevice(path)
            if e.EV_KEY in dev.capabilities() and len(dev.capabilities()[e.EV_KEY]) > 50:
                return dev
        return None

    def hotkey_listener(self):
        kbd = self.find_keyboard()
        if not kbd:
            print("Keyboard not found")
            return

        pressed = set()
        for event in kbd.read_loop():
            if event.type == e.EV_KEY:
                if event.value == 1:
                    pressed.add(event.code)
                elif event.value == 0:
                    pressed.discard(event.code)

                if HOTKEY_COMBO.issubset(pressed):
                    self.signals.toggle.emit()
                    pressed.clear()

    # =========================
    # Control
    # =========================
    def toggle_recording(self):
        self.is_recording = not self.is_recording

        if self.is_recording:
            with self.buffer_lock:
                self.audio_buffer.clear()
            self.last_voice_time = None
            self.signals.status.emit("Status: LISTENING")
            print("Recording started")
        else:
            self.signals.status.emit("Status: IDLE")
            print("Recording stopped")


# =========================
# Entry
# =========================
def main():
    app = QApplication(sys.argv)
    window = AudioPasteApp()
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
