import time
import threading
import pyperclip
import evdev
from evdev import ecodes as e, UInput
from PyQt6.QtWidgets import QWidget, QLabel, QPushButton, QVBoxLayout

from constants.config import HOTKEY_COMBO
from services.audio_service import AudioService
from services.transcription_service import TranscriptionService
from views.signals import Signals


class MainWindow(QWidget): 

    def __init__(self):
        super().__init__()
        self._setup_ui()
        self._setup_signals()
        self._setup_services()
        self._start_background_threads()

    def _setup_ui(self): 
        self.setWindowTitle("AI Audio Paste")
        self.resize(1280, 720)

        self.label = QLabel("Status: IDLE\nHotkey: Ctrl + Alt + R")
        self.button = QPushButton("Manual Toggle")
        self.button.clicked.connect(self.toggle_recording)

        layout = QVBoxLayout()
        layout.addWidget(self.label)
        layout.addWidget(self.button)
        self.setLayout(layout)

    def _setup_signals(self): 
        self.signals = Signals()
        self.signals.toggle.connect(self.toggle_recording)
        self.signals.status.connect(self._update_label)

    def _setup_services(self): 
        self.audio_service = AudioService(on_silence_callback=self._finalize_transcription)
        self.transcription_service = TranscriptionService()

    def _start_background_threads(self):
        """Start background threads for audio and hotkey listening."""
        self.audio_service.start()
        threading.Thread(target=self._hotkey_listener, daemon=True).start()

    def _update_label(self, text): 
        self.label.setText(text)

    def _finalize_transcription(self): 
        audio = self.audio_service.get_audio_data()
        if audio is None:
            return

        audio = AudioService.trim_silence(audio)
        self.audio_service.stop_recording()
        self.signals.status.emit("Status: TRANSCRIBING")

        text = self.transcription_service.transcribe(audio)
        if text:
            print("Final:", text)
            self._paste_text(text)

        self.signals.status.emit("Status: IDLE")

    def _paste_text(self, text): 
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

    def _find_keyboard(self): 
        for path in evdev.list_devices():
            dev = evdev.InputDevice(path)
            if e.EV_KEY in dev.capabilities() and len(dev.capabilities()[e.EV_KEY]) > 50:
                return dev
        return None

    def _hotkey_listener(self): 
        kbd = self._find_keyboard()
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

    def toggle_recording(self): 
        if not self.audio_service.is_recording:
            self.audio_service.start_recording()
            self.signals.status.emit("Status: LISTENING")
            print("Recording started")
        else:
            self.audio_service.stop_recording()
            self.signals.status.emit("Status: IDLE")
            print("Recording stopped")
