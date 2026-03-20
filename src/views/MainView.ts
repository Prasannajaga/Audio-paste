import { UI_STRINGS } from "../constants/ui";

type AppStatus = "IDLE" | "LISTENING" | "TRANSCRIBING" | "LOADING";

export class MainView {
    private wave: HTMLElement;
    private statusText: HTMLElement;
    private statusProgress: HTMLElement;
    private transcript: HTMLElement;
    private copyTranscriptBtn: HTMLButtonElement;
    private modalOverlay: HTMLElement;
    private applyBtn: HTMLButtonElement;
    private threadVal: HTMLElement;
    private silenceThresholdInput: HTMLInputElement;
    private silenceSecondsInput: HTMLInputElement;
    private selectedModel = "base";
    private threads = 4;
    private audioCtx: AudioContext | null = null;

    constructor() {
        this.wave = document.getElementById("wave")!;
        this.statusText = document.getElementById("status-text")!;
        this.statusProgress = document.getElementById("status-progress")!;
        this.transcript = document.getElementById("transcript")!;
        this.copyTranscriptBtn = document.getElementById("copy-transcript-btn") as HTMLButtonElement;
        this.modalOverlay = document.getElementById("modal-overlay")!;
        this.applyBtn = document.getElementById("apply-btn") as HTMLButtonElement;
        this.threadVal = document.getElementById("thread-val")!;
        this.silenceThresholdInput = document.getElementById("silence-threshold") as HTMLInputElement;
        this.silenceSecondsInput = document.getElementById("silence-seconds") as HTMLInputElement;

        document.getElementById("settings-btn")!.addEventListener("click", () => this.openModal());
        document.getElementById("modal-close")!.addEventListener("click", () => this.closeModal());
        this.modalOverlay.addEventListener("click", (e) => {
            if (e.target === this.modalOverlay) this.closeModal();
        });

        this.initChips();
        this.initStepper();
        this.copyTranscriptBtn.addEventListener("click", () => this.copyTranscript());
    }

    private initChips() {
        const chips = document.querySelectorAll<HTMLButtonElement>("#model-chips .chip");
        chips.forEach((chip) => {
            chip.addEventListener("click", () => {
                chips.forEach((c) => c.classList.remove("active"));
                chip.classList.add("active");
                this.selectedModel = chip.dataset.value!;
            });
        });
    }

    private initStepper() {
        document.getElementById("thread-dec")!.addEventListener("click", () => {
            if (this.threads > 1) {
                this.threads--;
                this.threadVal.textContent = this.threads.toString();
            }
        });
        document.getElementById("thread-inc")!.addEventListener("click", () => {
            if (this.threads < 16) {
                this.threads++;
                this.threadVal.textContent = this.threads.toString();
            }
        });
    }

    public playTick() {
        this.beep(1200, 600, 0.06, 0.15);
    }

    public playDone() {
        this.beep(800, 1200, 0.08, 0.12);
        setTimeout(() => this.beep(1200, 1600, 0.08, 0.12), 100);
    }

    private beep(freqStart: number, freqEnd: number, duration: number, volume: number) {
        try {
            if (!this.audioCtx) this.audioCtx = new AudioContext();
            const ctx = this.audioCtx;
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();
            osc.type = "sine";
            osc.frequency.setValueAtTime(freqStart, ctx.currentTime);
            osc.frequency.exponentialRampToValueAtTime(freqEnd, ctx.currentTime + duration);
            gain.gain.setValueAtTime(volume, ctx.currentTime);
            gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + duration + 0.02);
            osc.connect(gain);
            gain.connect(ctx.destination);
            osc.start(ctx.currentTime);
            osc.stop(ctx.currentTime + duration + 0.02);
        } catch (_) {
            /* silent fallback */
        }
    }

    public setStatus(status: AppStatus) {
        this.wave.className = "wave";
        this.statusText.className = "";
        this.statusProgress.className = "status-progress hidden";

        const textMap: Record<AppStatus, string> = {
            IDLE: UI_STRINGS.STATUS_IDLE,
            LISTENING: UI_STRINGS.STATUS_LISTENING,
            TRANSCRIBING: UI_STRINGS.STATUS_TRANSCRIBING,
            LOADING: UI_STRINGS.STATUS_LOADING,
        };

        this.statusText.textContent = textMap[status];

        if (status === "LISTENING") {
            this.wave.classList.add("listening");
            this.statusText.classList.add("listening");
        } else if (status === "TRANSCRIBING" || status === "LOADING") {
            this.wave.classList.add("transcribing");
            this.statusText.classList.add("transcribing");
            if (status === "LOADING") {
                this.statusProgress.classList.remove("hidden");
                this.statusProgress.classList.add("active");
            }
        }
    }

    public showTranscription(text: string) {
        this.transcript.textContent = text;
        this.transcript.classList.add("visible");
    }

    public copyTranscript() {
        const text = this.transcript.textContent?.trim();
        if (!text) return;

        navigator.clipboard?.writeText(text).catch(() => {
            const range = document.createRange();
            range.selectNodeContents(this.transcript);
            const selection = window.getSelection();
            selection?.removeAllRanges();
            selection?.addRange(range);
            document.execCommand("copy");
            selection?.removeAllRanges();
        });
    }

    public bindToggle(handler: () => void) {
        document.getElementById("wave-area")!.onclick = handler;
    }

    public bindApply(handler: (m: string, t: number, st: number, ss: number) => void) {
        this.applyBtn.onclick = () => {
            handler(
                this.selectedModel,
                this.threads,
                Number(this.silenceThresholdInput.value),
                Number(this.silenceSecondsInput.value)
            );
            this.closeModal();
        };
    }

    private openModal() { this.modalOverlay.classList.remove("hidden"); }
    private closeModal() { this.modalOverlay.classList.add("hidden"); }
}
