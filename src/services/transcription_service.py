"""
Transcription service using Whisper for speech-to-text.
"""
from faster_whisper import WhisperModel

from constants.config import MODEL_SIZE


class TranscriptionService:
    """Handles speech-to-text transcription using Whisper."""

    def __init__(self):
        """Initialize the Whisper model."""
        print(f"Loading Whisper model: {MODEL_SIZE}")
        self.model = WhisperModel(
            MODEL_SIZE,
            device="cpu",
            compute_type="int8",
            cpu_threads=4,
            num_workers=1
        )

    def transcribe(self, audio):
        """
        Transcribe audio to text.
        
        Args:
            audio: numpy array of audio data
            
        Returns:
            Transcribed text string
        """
        segments, info = self.model.transcribe(
            audio,
            beam_size=1,
            best_of=1,
            temperature=0.0,
            vad_filter=False,
            condition_on_previous_text=False,
        )
        return "".join(s.text for s in segments).strip()
