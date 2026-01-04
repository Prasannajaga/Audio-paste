import time
import threading
import numpy as np
import sounddevice as sd
from collections import deque

from constants.config import SAMPLE_RATE, SILENCE_THRESHOLD, SILENCE_SECONDS


class AudioService:
    def __init__(self, on_silence_callback=None): 
        self.is_recording = False
        self.audio_buffer = deque()
        self.buffer_lock = threading.Lock()
        self.last_voice_time = None
        self.on_silence_callback = on_silence_callback

    def start(self): 
        threading.Thread(target=self._audio_stream, daemon=True).start()
        threading.Thread(target=self._silence_monitor, daemon=True).start()

    def _audio_callback(self, indata, frames, time_info, status):
        if not self.is_recording:
            return

        audio = indata[:, 0]
        rms = np.sqrt(np.mean(audio ** 2))

        with self.buffer_lock:
            self.audio_buffer.extend(audio)

        if rms > SILENCE_THRESHOLD:
            self.last_voice_time = time.time()

    def _audio_stream(self):
        with sd.InputStream(
            samplerate=SAMPLE_RATE,
            channels=1,
            blocksize=1024,
            callback=self._audio_callback,
        ):
            while True:
                time.sleep(1)

    def _silence_monitor(self): 
        while True:
            if not self.is_recording or self.last_voice_time is None:
                time.sleep(0.1)
                continue

            if time.time() - self.last_voice_time >= SILENCE_SECONDS:
                if self.on_silence_callback:
                    self.on_silence_callback()

            time.sleep(0.1)

    def start_recording(self): 
        with self.buffer_lock:
            self.audio_buffer.clear()
        self.last_voice_time = None
        self.is_recording = True

    def stop_recording(self): 
        self.is_recording = False

    def get_audio_data(self): 
        with self.buffer_lock:
            if not self.audio_buffer:
                return None
            audio = np.array(self.audio_buffer, dtype=np.float32)
            self.audio_buffer.clear()
        return audio

    @staticmethod
    def trim_silence(audio, threshold=0.01): 
        rms = np.sqrt(audio ** 2)
        idx = np.where(rms > threshold)[0]
        if len(idx) == 0:
            return audio
        return audio[idx[0]:idx[-1]]
