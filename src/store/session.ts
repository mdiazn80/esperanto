import { Channel } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { create } from "zustand";
import {
  commands,
  type AudioSource,
  type ErrorCode,
  type SessionEvent,
} from "@/bindings";
import { errorCode } from "@/lib/errors";
import i18n from "@/lib/i18n";
import { SILENCE_DB } from "@/lib/level";
import { notifyError, notifyErrorCode } from "@/lib/notify";
import {
  applyEvent,
  endSpeaking,
  isVisible,
  type Segment,
} from "@/lib/transcript";
import { useSettings } from "@/store/settings";

type SessionStatus = "idle" | "starting" | "listening" | "stopping";

/** A phrase the user marked to find it again. */
export type Moment = {
  segmentId: number;
  /** When it was marked (Unix ms). */
  markedAt: number;
};

/** What the running session uses, as reported by the backend. */
type ActiveSession = {
  deviceName: string;
  source: AudioSource;
  modelName: string;
  /** Host of the remote gateway; `null` for the local model. */
  remoteHost: string | null;
};

type SessionState = {
  status: SessionStatus;
  /** Set while listening (or stopping). */
  active: ActiveSession | null;
  /** System audio has been completely silent (nothing playing, or no permission). */
  systemSilent: boolean;
  /** Input level in dBFS. */
  level: number;
  /** The last input levels (dBFS), oldest first, for the waveform. */
  levels: number[];
  segments: Segment[];
  /** Latency of the last final transcript, in ms. */
  latencyMs: number | null;
  /** Inference can't keep up with the speech. */
  behind: boolean;
  /** Last error that stopped or prevented a session. */
  error: ErrorCode | null;
  /** Phrases get speaker labels in this session. */
  speakerLabels: boolean;
  /** Names the user gave to speakers in this session, by speaker number. */
  speakerNames: Record<number, string>;
  renameSpeaker: (speaker: number, name: string) => void;
  /** Phrases marked in this session, in the order they were marked. */
  moments: Moment[];
  /** Marks the newest phrase. Returns whether there was one to mark. */
  markMoment: () => boolean;
  /** History id and start (Unix ms) of the last session, for export. */
  lastSession: { historyId: string; startedAt: number } | null;
  start: (deviceId: string | null, source: AudioSource) => Promise<void>;
  stop: () => Promise<void>;
};

/** Errors explained in the transcript area instead of a toast. */
const EXPLAINED_ERRORS: ReadonlySet<ErrorCode> = new Set([
  "microphonePermissionDenied",
  "modelNotInstalled",
  "llamaServerNotFound",
]);

/** At most one "couldn't transcribe" toast per this many ms. */
const INFERENCE_TOAST_INTERVAL = 10_000;

function reportError(code: ErrorCode | null, error?: unknown) {
  if (code && EXPLAINED_ERRORS.has(code)) return;
  if (code) notifyErrorCode(code);
  else notifyError(error);
}

/** How many input levels the waveform keeps (about 1.4 s). */
export const LEVEL_HISTORY = 28;

function silentLevels(): number[] {
  return Array<number>(LEVEL_HISTORY).fill(SILENCE_DB);
}

/** Whether the user is speaking (a segment is still open). */
export function isSpeaking(segments: readonly Segment[]): boolean {
  return segments.some((s) => s.state === "speaking");
}

/** The speaker of the phrase being spoken, if known. */
export function currentSpeaker(segments: readonly Segment[]): number | null {
  return segments.findLast((s) => s.state === "speaking")?.speaker ?? null;
}

/** The speakers heard so far, in order of appearance. */
export function speakersOf(segments: readonly Segment[]): number[] {
  const seen = new Set<number>();
  for (const s of segments) {
    if (s.speaker !== null && isVisible(s)) seen.add(s.speaker);
  }
  return [...seen].sort((a, b) => a - b);
}

export const useSession = create<SessionState>((set, get) => {
  let lastInferenceToast = 0;
  /** Identifies the current session; events of older ones are ignored. */
  let generation = 0;

  function onEvent(event: SessionEvent) {
    switch (event.type) {
      case "level": {
        // Floats arrive as `number | null` (specta allows NaN to be null).
        const level = event.rmsDb ?? SILENCE_DB;
        const levels = [...get().levels.slice(1), level];
        set(
          level > SILENCE_DB
            ? { level, levels, systemSilent: false }
            : { level, levels },
        );
        return;
      }
      case "systemAudioSilent":
        set({ systemSilent: true });
        return;
      case "status":
        set({ behind: event.status === "behind" });
        return;
      case "error":
        set({ error: event.code });
        reportError(event.code);
        // While starting, `start` stops the session once it is up.
        void get().stop();
        return;
      case "inferenceError":
        if (Date.now() - lastInferenceToast > INFERENCE_TOAST_INTERVAL) {
          lastInferenceToast = Date.now();
          notifyErrorCode(event.code);
        }
        break;
      case "final":
        set({ latencyMs: event.latencyMs });
        break;
    }
    set((state) => ({ segments: applyEvent(state.segments, event) }));
  }

  return {
    status: "idle",
    active: null,
    systemSilent: false,
    level: SILENCE_DB,
    levels: silentLevels(),
    segments: [],
    latencyMs: null,
    behind: false,
    error: null,
    lastSession: null,
    speakerLabels: false,
    speakerNames: {},
    moments: [],

    markMoment: () => {
      const last = get().segments.filter(isVisible).at(-1);
      if (!last) return false;
      if (!get().moments.some((m) => m.segmentId === last.id)) {
        set((state) => ({
          moments: [
            ...state.moments,
            { segmentId: last.id, markedAt: Date.now() },
          ],
        }));
      }
      return true;
    },

    renameSpeaker: (speaker, name) =>
      set((state) => ({
        speakerNames: { ...state.speakerNames, [speaker]: name.trim() },
      })),

    start: async (deviceId, source) => {
      if (get().status !== "idle") return;
      const current = ++generation;
      set({
        status: "starting",
        systemSilent: false,
        error: null,
        segments: [],
        latencyMs: null,
        behind: false,
        lastSession: null,
        speakerLabels: false,
        speakerNames: {},
        moments: [],
        levels: silentLevels(),
      });
      const startedAt = Date.now();
      const channel = new Channel<SessionEvent>();
      channel.onmessage = (event) => {
        if (current === generation) onEvent(event);
      };
      try {
        const started = await commands.startSession(deviceId, channel);
        set({
          status: "listening",
          active: {
            deviceName: started.deviceName,
            source,
            modelName: started.modelName,
            remoteHost: started.remoteHost,
          },
          lastSession: { historyId: started.historyId, startedAt },
          speakerLabels: started.speakerLabels,
        });
        if (
          useSettings.getState().settings.identifySpeakers &&
          !started.speakerLabels
        ) {
          toast.info(i18n.t("speakers.modelMissing"));
        }
        if (started.usedDefaultDevice) {
          toast.info(i18n.t("session.usingDefaultInput"));
        }
        // Capture failed while the model was loading.
        if (get().error) await get().stop();
      } catch (error) {
        const code = errorCode(error);
        set({ status: "idle", error: code });
        console.error(error);
        reportError(code, error);
      }
    },

    stop: async () => {
      if (get().status !== "listening") return;
      set({ status: "stopping" });
      try {
        await commands.stopSession();
      } catch (error) {
        notifyError(error);
      } finally {
        set((state) => ({
          status: "idle",
          active: null,
          systemSilent: false,
          level: SILENCE_DB,
          levels: silentLevels(),
          segments: endSpeaking(state.segments),
        }));
      }
    },
  };
});
