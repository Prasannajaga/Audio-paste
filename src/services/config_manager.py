import json
from pathlib import Path
from PyQt6.QtCore import QObject, pyqtSignal


class ConfigManager(QObject):
    
    config_changed = pyqtSignal(dict)
    
    CONFIG_DIR = Path.home() / ".audio-paste"
    CONFIG_FILE = CONFIG_DIR / "config.json"
    
    DEFAULTS = {
        "model_size": "tiny.en",
        "device": "cpu",
        "cpu_threads": 4
    }
    
    def __init__(self):
        super().__init__()
        self._config = self.DEFAULTS.copy()
        self._load()
    
    def _load(self):
        if self.CONFIG_FILE.exists():
            try:
                with open(self.CONFIG_FILE, "r") as f:
                    saved = json.load(f)
                    self._config.update(saved)
            except (json.JSONDecodeError, IOError) as e:
                print(f"Error loading config: {e}, using defaults")
    
    def save(self):
        self.CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        try:
            with open(self.CONFIG_FILE, "w") as f:
                json.dump(self._config, f, indent=2)
        except IOError as e:
            print(f"Error saving config: {e}")
    
    @property
    def model_size(self) -> str:
        return self._config["model_size"]
    
    @model_size.setter
    def model_size(self, value: str):
        self._config["model_size"] = value
    
    @property
    def device(self) -> str:
        return self._config["device"]
    
    @device.setter
    def device(self, value: str):
        self._config["device"] = value
    
    @property
    def cpu_threads(self) -> int:
        return self._config["cpu_threads"]
    
    @cpu_threads.setter
    def cpu_threads(self, value: int):
        self._config["cpu_threads"] = max(1, min(16, value))
    
    def get_all(self) -> dict:
        return self._config.copy()
    
    def update(self, **kwargs):
        for key, value in kwargs.items():
            if key in self._config:
                setattr(self, key, value)
        self.save()
        self.config_changed.emit(self.get_all())
