import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export class ApiService {
    static async startRecording(): Promise<void> {
        console.debug("[ApiService] invoking start_recording");
        await invoke("start_recording");
    }

    static async stopRecording(): Promise<void> {
        console.debug("[ApiService] invoking stop_recording");
        await invoke("stop_recording");
    }

    static async applyConfig(model: string, device: string, threads: number): Promise<void> {
        console.debug("[ApiService] invoking apply_config:", { model, device, threads });
        await invoke("apply_config", { model, device, threads });
    }

    static async processTranscription(): Promise<string> {
        console.debug("[ApiService] invoking process_transcription");
        const result = await invoke<string>("process_transcription");
        return result;
    }

    static onSilenceDetected(callback: () => void) {
        return listen("silence_detected", () => {
            console.debug("[ApiService] event: silence_detected");
            callback();
        });
    }

    static onToggleRecording(callback: () => void) {
        return listen("toggle_recording", () => {
            console.debug("[ApiService] event: toggle_recording");
            callback();
        });
    }

    static onStatusChange(callback: (status: string) => void) {
        return listen<string>("status_change", (event) => {
            console.debug("[ApiService] event: status_change:", event.payload);
            callback(event.payload);
        });
    }

    static onTranscriptionResult(callback: (text: string) => void) {
        return listen<string>("transcription_result", (event) => {
            console.debug("[ApiService] event: transcription_result:", event.payload);
            callback(event.payload);
        });
    }
}
