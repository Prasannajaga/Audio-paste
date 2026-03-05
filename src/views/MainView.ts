import { UI_STRINGS } from "../constants/ui";

type AppStatus = "IDLE" | "LISTENING" | "TRANSCRIBING" | "LOADING";

export class MainView {
    private wave: HTMLElement;
    private statusText: HTMLElement;
    private transcript: HTMLElement;
    private modalOverlay: HTMLElement;
    private applyBtn: HTMLButtonElement;
    private threadVal: HTMLElement;
    private selectedModel = "base.en";
    private threads = 4;
    private audioCtx: AudioContext | null = null;

    constructor() {
        this.wave = document.getElementById("wave")!;
        this.statusText = document.getElementById("status-text")!;
        this.transcript = document.getElementById("transcript")!;
        this.modalOverlay = document.getElementById("modal-overlay")!;
        this.applyBtn = document.getElementById("apply-btn") as HTMLButtonElement;
        this.threadVal = document.getElementById("thread-val")!;

        document.getElementById("settings-btn")!.addEventListener("click", () => this.openModal());
        document.getElementById("modal-close")!.addEventListener("click", () => this.closeModal());
        this.modalOverlay.addEventListener("click", (e) => {
            if (e.target === this.modalOverlay) this.closeModal();
        });

        this.initChips();
        this.initStepper();
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
        }
    }

    public showTranscription(text: string) {
        this.transcript.textContent = text;
        this.transcript.classList.add("visible");
    }

    public bindToggle(handler: () => void) {
        document.getElementById("wave-area")!.onclick = handler;
    }

    public bindApply(handler: (m: string, t: number) => void) {
        this.applyBtn.onclick = () => {
            handler(this.selectedModel, this.threads);
            this.closeModal();
        };
    }

    private openModal() { this.modalOverlay.classList.remove("hidden"); }
    private closeModal() { this.modalOverlay.classList.add("hidden"); }
}
