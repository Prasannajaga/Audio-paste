from evdev import ecodes as e

# Hotkey configuration
HOTKEY_COMBO = {e.KEY_LEFTCTRL, e.KEY_LEFTALT, e.KEY_R}

# Audio settings
SAMPLE_RATE = 16000

# Whisper model settings
MODEL_SIZE = "tiny.en"

# Silence detection
SILENCE_THRESHOLD = 0.01   # RMS threshold
SILENCE_SECONDS = 4.0      # Stop after 4s silence
