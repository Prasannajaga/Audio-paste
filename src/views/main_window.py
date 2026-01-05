import time
import threading
import pyperclip
from PyQt6.QtWidgets import (
    QWidget, QLabel, QPushButton, QVBoxLayout, QHBoxLayout,
    QComboBox, QSpinBox, QGroupBox, QGridLayout, QFrame, QMessageBox
)
from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QFont , QShortcut, QKeySequence 
from evdev import ecodes as e, UInput
from services.audio_service import AudioService
from services.transcription_service import (
    TranscriptionService,
    DeviceNotSupportedError
)
from services.config_manager import ConfigManager
from views.signals import Signals
from views.styles import DARK_THEME, STATUS_COLORS 
class MainWindow(QWidget):

    def __init__(self):
        super().__init__()

        self.config_manager = ConfigManager()
        self._finalizing = False

        try:
            self.ui = UInput()
        except Exception as err:
            print(f"UInput error: {err}")
            self.ui = None

        self._setup_ui() 
        self._setup_signals()
        self._setup_services() 
        self._setup_shortcuts()
    
    def _setup_ui(self):
        self.setWindowTitle("Audio Paste")
        self.resize(700, 400)
        self.setMinimumSize(600, 350)
        self.setStyleSheet(DARK_THEME)

        main_layout = QHBoxLayout(self)
        main_layout.setSpacing(20)
        main_layout.setContentsMargins(20, 20, 20, 20)

        main_layout.addWidget(self._create_config_panel())
        main_layout.addWidget(self._create_status_panel(), stretch=1)

    def _create_config_panel(self):
        group = QGroupBox("Configuration")
        layout = QGridLayout(group)

        self.model_combo = QComboBox()
        self.model_combo.addItems(TranscriptionService.get_available_models())
        self.model_combo.setCurrentText(self.config_manager.model_size)

        self.device_combo = QComboBox()
        self.device_combo.addItems(TranscriptionService.get_available_devices())
        self.device_combo.setCurrentText(self.config_manager.device)

        self.threads_spin = QSpinBox()
        self.threads_spin.setRange(1, 16)
        self.threads_spin.setValue(self.config_manager.cpu_threads)

        self.apply_btn = QPushButton("Apply Configuration")
        self.apply_btn.clicked.connect(self._apply_config)

        layout.addWidget(QLabel("Model:"), 0, 0)
        layout.addWidget(self.model_combo, 0, 1)
        layout.addWidget(QLabel("Device:"), 1, 0)
        layout.addWidget(self.device_combo, 1, 1)
        layout.addWidget(QLabel("CPU Threads:"), 2, 0)
        layout.addWidget(self.threads_spin, 2, 1)
        layout.addWidget(self.apply_btn, 3, 0, 1, 2)

        return group

    def _create_status_panel(self):
        frame = QFrame()
        layout = QVBoxLayout(frame)

        self.status_label = QLabel("● IDLE")
        self.status_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        font = QFont()
        font.setPointSize(24)
        font.setBold(True)
        self.status_label.setFont(font)

        self.model_info_label = QLabel(self._model_info())
        self.model_info_label.setAlignment(Qt.AlignmentFlag.AlignCenter)

        self.toggle_btn = QPushButton("🎙️ Start Recording")
        self.toggle_btn.clicked.connect(self.toggle_recording)

        layout.addWidget(self.status_label)
        layout.addWidget(self.model_info_label)
        layout.addStretch()
        layout.addWidget(QLabel("Hotkey: Ctrl + Alt + R", alignment=Qt.AlignmentFlag.AlignCenter))
        layout.addWidget(self.toggle_btn)

        return frame

    def _model_info(self):
        return (
            f"Model: {self.config_manager.model_size} | "
            f"Device: {self.config_manager.device.upper()} | "
            f"Threads: {self.config_manager.cpu_threads}"
        )  
    
    def _setup_signals(self):
        self.signals = Signals()
        self.signals.status.connect(self._update_status)      

    def _setup_shortcuts(self):
        # 1. Local Shortcut (when window is focused)
        self.qt_shortcut = QShortcut(QKeySequence("Ctrl+Alt+R"), self)
        self.qt_shortcut.setContext(Qt.ShortcutContext.WindowShortcut)
        self.qt_shortcut.activated.connect(self.toggle_recording) 


    def _setup_services(self):
        self.audio_service = AudioService(
            on_silence_callback=self._on_silence_detected
        )

        cfg = self.config_manager.get_all()
        self.transcription_service = TranscriptionService(
            model_size=cfg["model_size"],
            device=cfg["device"],
            cpu_threads=cfg["cpu_threads"]
        )

        self.audio_service.start() 

    def _update_status(self, status):
        status_l = status.lower()

        if "idle" in status_l:
            color = STATUS_COLORS["idle"]
            self.toggle_btn.setText("🎙️ Start Recording")
            self.toggle_btn.setEnabled(True)
            self.apply_btn.setEnabled(True)
        elif "listening" in status_l:
            color = STATUS_COLORS["listening"]
            self.toggle_btn.setText("⏹️ Stop Recording")
            self.apply_btn.setEnabled(False)
        elif "transcribing" in status_l:
            color = STATUS_COLORS["transcribing"]
            self.toggle_btn.setEnabled(False)
            self.apply_btn.setEnabled(False)
        else:
            color = "#888"

        self.status_label.setText(f"● {status}")
        self.status_label.setStyleSheet(f"color: {color}")
 

    def toggle_recording(self):
        if not self.audio_service.is_recording:
            self.audio_service.start_recording()
            self.signals.status.emit("LISTENING")
        else:
            self.audio_service.stop_recording()
            self.signals.status.emit("IDLE") 

    def _on_silence_detected(self):
        if self.audio_service.is_recording:
            self.audio_service.stop_recording()
            QTimer.singleShot(0, self._finalize_transcription)

    def _finalize_transcription(self):
        if self._finalizing:
            return

        self._finalizing = True
        audio = self.audio_service.get_audio_data()

        if audio is None or len(audio) == 0:
            self._finalizing = False
            self.signals.status.emit("IDLE")
            return

        audio = AudioService.trim_silence(audio)
        self.signals.status.emit("TRANSCRIBING")

        def worker():
            try:
                text = self.transcription_service.transcribe(audio)
                print("FInal:", text)
                if text:
                    self._paste_text(text)
            finally:
                self._finalizing = False
                self.signals.status.emit("IDLE")

        threading.Thread(target=worker, daemon=True).start()
 

    def _paste_text(self, text):
        print("Final:", text)
        if not self.ui:
            print("Not UI")
            return

        pyperclip.copy(text)
        time.sleep(0.2)

        self.ui.write(e.EV_KEY, e.KEY_LEFTCTRL, 1)
        self.ui.write(e.EV_KEY, e.KEY_V, 1)
        self.ui.syn()
        self.ui.write(e.EV_KEY, e.KEY_V, 0)
        self.ui.write(e.EV_KEY, e.KEY_LEFTCTRL, 0)
        self.ui.syn()
 

    def _apply_config(self):
        model = self.model_combo.currentText()
        device = self.device_combo.currentText()
        threads = self.threads_spin.value()

        self.config_manager.update(
            model_size=model,
            device=device,
            cpu_threads=threads
        )

        self.signals.status.emit("LOADING MODEL...")
        self.apply_btn.setEnabled(False)

        def reload_worker():
            try:
                self.transcription_service.reload_model(model, device, threads)
                self.signals.status.emit("IDLE")
            except DeviceNotSupportedError as err:
                QTimer.singleShot(0, lambda: QMessageBox.critical(self, "Error", str(err)))
            finally:
                self.apply_btn.setEnabled(True)
                self.model_info_label.setText(self._model_info())

        threading.Thread(target=reload_worker, daemon=True).start()
