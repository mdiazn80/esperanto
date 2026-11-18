import { Channel } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { create } from "zustand";
import {
  commands,
  type DownloadProgress,
  type ModelInfo,
  type ModelsOverview,
} from "@/bindings";
import { errorCode } from "@/lib/errors";
import i18n from "@/lib/i18n";
import { notifyError } from "@/lib/notify";

type Progress = { downloaded: number; total: number };

type ModelsState = {
  overview: ModelsOverview | null;
  /** Live progress of downloads started from this window, by model id. */
  progress: Record<string, Progress>;
  load: () => Promise<void>;
  download: (id: string) => Promise<void>;
  cancel: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
};

function without<T>(record: Record<string, T>, id: string) {
  return Object.fromEntries(
    Object.entries(record).filter(([key]) => key !== id),
  );
}

export const useModels = create<ModelsState>((set, get) => {
  /** Downloads running from this window, to ignore a second start. */
  const running = new Set<string>();

  return {
    overview: null,
    progress: {},

    load: async () => {
      try {
        set({ overview: await commands.listModels() });
      } catch (error) {
        notifyError(error);
      }
    },

    download: async (id) => {
      if (running.has(id)) return;
      running.add(id);
      const channel = new Channel<DownloadProgress>();
      channel.onmessage = (p) =>
        set((state) => ({
          progress: {
            ...state.progress,
            [id]: {
              downloaded: p.downloadedBytes ?? 0,
              total: p.totalBytes ?? 0,
            },
          },
        }));
      // Handle the result right away so a failure is never left unhandled.
      const finished = commands.downloadModel(id, channel).then(
        () => {
          toast.success(i18n.t("models.downloaded"));
        },
        (error: unknown) => {
          if (errorCode(error) !== "downloadCancelled") notifyError(error);
        },
      );
      // Reload so the model shows as downloading.
      await get().load();
      await finished;
      running.delete(id);
      set((state) => ({ progress: without(state.progress, id) }));
      await get().load();
    },

    cancel: async (id) => {
      try {
        await commands.cancelDownload(id);
      } catch (error) {
        notifyError(error);
      }
    },

    remove: async (id) => {
      try {
        await commands.deleteModel(id);
      } catch (error) {
        notifyError(error);
      }
      await get().load();
    },
  };
});

function speechModels(overview: ModelsOverview | null): ModelInfo[] {
  return overview?.models.filter((m) => m.kind === "speech") ?? [];
}

/** The installed speech model the app will use, preferring the recommended one. */
export function activeModel(overview: ModelsOverview | null): ModelInfo | null {
  const installed = speechModels(overview).filter(
    (m) => m.status.state === "installed",
  );
  return installed.find((m) => m.recommended) ?? installed[0] ?? null;
}

/** The speech model to offer first: the recommended one. */
export function recommendedModel(
  overview: ModelsOverview | null,
): ModelInfo | null {
  const models = speechModels(overview);
  return models.find((m) => m.recommended) ?? models[0] ?? null;
}

/** The speaker-embedding model (speaker labels). */
export function speakerModel(
  overview: ModelsOverview | null,
): ModelInfo | null {
  return overview?.models.find((m) => m.kind === "speakers") ?? null;
}
