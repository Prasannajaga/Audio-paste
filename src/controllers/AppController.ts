import { ApiService } from "../services/api";
import { MainView } from "../views/MainView";

export class AppController {
    private view: MainView;
    private isRecording = false;
    private isFinalizing = false;

    constructor() {
        this.view = new MainView();

        this.view.bindToggle(() => this.toggleRecording());
        this.view.bindApply((m, t) => this.applyConfig(m, t));

        ApiService.onSilenceDetected(() => this.handleSilence());
        ApiService.onToggleRecording(() => this.toggleRecording());
        ApiService.onStatusChange((status) => {
            console.debug("[AppController] status_change event:", status);
            this.view.setStatus(status as "IDLE" | "LISTENING" | "TRANSCRIBING" | "LOADING");
            if (status === "IDLE") {
                this.isRecording = false;
                this.isFinalizing = false;
            }
        });
        ApiService.onTranscriptionResult((text) => {
            console.debug("[AppController] transcription_result:", text);
            if (text.trim()) {
                this.view.showTranscription(text);
            }
        });

        this.view.setStatus("IDLE");
        ApiService.applyConfig("base.en", "cpu", 4).catch((e) => {
            console.error("[AppController] Failed to apply initial config:", e);
        });
    }

    private async toggleRecording() {
        console.debug("[AppController] toggleRecording, isRecording:", this.isRecording);
        try {
            if (this.isRecording) {
                await ApiService.stopRecording();
                this.isRecording = false;
                this.isFinalizing = false;
                this.view.setStatus("IDLE");
            } else {
                this.view.playTick();
                await ApiService.startRecording();
                this.isRecording = true;
                this.isFinalizing = false;
                this.view.setStatus("LISTENING");
            }
        } catch (e) {
            console.error("[AppController] toggleRecording error:", e);
            this.isRecording = false;
            this.isFinalizing = false;
            this.view.setStatus("IDLE");
        }
    }

    private async handleSilence() {
        console.debug("[AppController] handleSilence, isFinalizing:", this.isFinalizing);
        if (this.isFinalizing) return;
        this.isFinalizing = true;
        this.isRecording = false;

        this.view.setStatus("TRANSCRIBING");

        try {
            const text = await ApiService.processTranscription();
            if (text.trim()) {
                this.view.showTranscription(text);
                this.view.playDone();
            }
        } catch (e) {
            console.error("[AppController] Transcription error:", e);
        } finally {
            this.view.setStatus("IDLE");
            this.isFinalizing = false;
            this.isRecording = false;
        }
    }

    private async applyConfig(model: string, threads: number) {
        this.view.setStatus("LOADING");
        try {
            await ApiService.applyConfig(model, "cpu", threads);
            this.view.setStatus("IDLE");
        } catch (e) {
            console.error("[AppController] applyConfig error:", e);
            this.view.setStatus("IDLE");
        }
    }
}
