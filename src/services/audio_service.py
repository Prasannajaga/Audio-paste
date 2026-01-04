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
        self.state_lock = threading.Lock()
        self.last_voice_time = None
        self.on_silence_callback = on_silence_callback
        self._finalizing = False

    def start(self): 
        threading.Thread(target=self._audio_stream, daemon=True).start()
        threading.Thread(target=self._silence_monitor, daemon=True).start()

    def _audio_callback(self, indata, frames, time_info, status):
        with self.state_lock:
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
            with self.state_lock:
                is_rec = self.is_recording
                last_voice = self.last_voice_time
                finalizing = self._finalizing
            
            if not is_rec or last_voice is None or finalizing:
                time.sleep(0.1)
                continue

            if time.time() - last_voice >= SILENCE_SECONDS:
                print("Silence detected, finalizing...") 
                with self.state_lock:
                    self._finalizing = True
                    self.last_voice_time = None
                
                if self.on_silence_callback: 
                    self.on_silence_callback()

            time.sleep(0.1)

    def start_recording(self): 
        with self.buffer_lock:
            self.audio_buffer.clear()
        with self.state_lock:
            self.last_voice_time = None
            self._finalizing = False
            self.is_recording = True

    def stop_recording(self): 
        with self.state_lock:
            self.is_recording = False
            self._finalizing = False

    def get_audio_data(self): 
        with self.buffer_lock:
            if not self.audio_buffer:
                return None
            audio = np.array(self.audio_buffer, dtype=np.float32)
            self.audio_buffer.clear()
        return audio

    @staticmethod
    def trim_silence(audio, threshold=0.01, window_size=1024): 
        abs_audio = np.abs(audio) 
        mask = abs_audio > threshold
        
        if not np.any(mask):
            return audio
             
        start_idx = np.argmax(mask)
        end_idx = len(mask) - np.argmax(mask[::-1])
        
        return audio[start_idx:end_idx]
