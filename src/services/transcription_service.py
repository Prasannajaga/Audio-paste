from faster_whisper import WhisperModel, available_models


class DeviceNotSupportedError(Exception):
    pass


class TranscriptionService: 
    
    AVAILABLE_DEVICES = ["cpu", "cuda"]
    
    def __init__(self, model_size: str = "tiny.en", device: str = "cpu", cpu_threads: int = 4): 
        self.model_size = model_size
        self.device = device
        self.cpu_threads = cpu_threads
        self.model = None
        self._load_model()
    
    @staticmethod
    def get_available_models() -> list:
        return available_models()
    
    @staticmethod
    def get_available_devices() -> list:
        return ["cpu"]
    
    def _load_model(self):
        print(f"Loading Whisper model: {self.model_size} on {self.device}")
        try:
            self.model = WhisperModel(
                self.model_size,
                device=self.device,
                compute_type="int8" if self.device == "cpu" else "float16",
                cpu_threads=self.cpu_threads,
                num_workers=1
            )
        except Exception as e:
            error_msg = str(e).lower()
            if "cuda" in error_msg or "gpu" in error_msg or "device" in error_msg:
                raise DeviceNotSupportedError(
                    f"Device '{self.device}' is not available. Please use 'cpu' or install CUDA drivers."
                ) from e
            raise
    
    def reload_model(self, model_size: str, device: str, cpu_threads: int):
        self.model_size = model_size
        self.device = device
        self.cpu_threads = cpu_threads
        self._load_model()

    def transcribe(self, audio): 
        if self.model is None:
            raise RuntimeError("Model not loaded")
            
        segments, info = self.model.transcribe(
            audio,
            beam_size=1,
            best_of=1,
            temperature=0.0,
            vad_filter=False,
            condition_on_previous_text=False,
        )
        return "".join(s.text for s in segments).strip()
