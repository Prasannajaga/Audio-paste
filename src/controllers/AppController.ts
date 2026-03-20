import { ApiService } from "../services/api";
import { MainView } from "../views/MainView";

export class AppController {
    private view: MainView;
    private isRecording = false;
    private isFinalizing = false;

    constructor() {
        this.view = new MainView();

        this.view.bindToggle(() => this.toggleRecording());
        this.view.bindApply((m, t, st, ss) => this.applyConfig(m, t, st, ss));

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

        this.view.setStatus("LOADING");
        ApiService.applyConfig("tiny", "cpu", 4, 0.01, 2)
            .then(() => {
                this.view.setStatus("IDLE");
            })
            .catch((e) => {
                console.error("[AppController] Failed to apply initial config:", e);
                this.view.setStatus("IDLE");
            });
    }

    private async toggleRecording() {
        console.debug("[AppController] toggleRecording, isRecording:", this.isRecording);
        try {
            if (this.isRecording) {
                if (this.isFinalizing) return;
                this.isFinalizing = true;
                await ApiService.stopRecording();
                this.isRecording = false;
                this.view.setStatus("TRANSCRIBING");
                void ApiService.processTranscription().catch((e) => {
                    console.error("[AppController] processTranscription error:", e);
                    this.view.setStatus("IDLE");
                    this.isRecording = false;
                    this.isFinalizing = false;
                });
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
        console.debug("[AppController] handleSilence, isRecording:", this.isRecording, "isFinalizing:", this.isFinalizing);
        if (this.isFinalizing || !this.isRecording) return;
        this.isFinalizing = true;
        this.isRecording = false;

        this.view.setStatus("TRANSCRIBING");

        try {
            await ApiService.stopRecording();
            void ApiService.processTranscription().catch((e) => {
                console.error("[AppController] Transcription error:", e);
                this.view.setStatus("IDLE");
                this.isFinalizing = false;
                this.isRecording = false;
            });
        } catch (e) {
            console.error("[AppController] Transcription error:", e);
            this.view.setStatus("IDLE");
            this.isFinalizing = false;
            this.isRecording = false;
        }
    }

    private async applyConfig(model: string, threads: number, silenceThreshold: number, silenceSeconds: number) {
        this.view.setStatus("LOADING");
        try {
            await ApiService.applyConfig(model, "cpu", threads, silenceThreshold, silenceSeconds);
            this.view.setStatus("IDLE");
        } catch (e) {
            console.error("[AppController] applyConfig error:", e);
            this.view.setStatus("IDLE");
        }
    }
}
