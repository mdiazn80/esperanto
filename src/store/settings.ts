import { create } from "zustand";
import { commands, type Settings_Serialize as Settings } from "@/bindings";
import { notifyError } from "@/lib/notify";

export const DEFAULT_SETTINGS: Settings = {
  uiLanguage: "system",
  theme: "system",
  sourceLanguage: { kind: "fixed", language: "en" },
  targetLanguage: { kind: "none" },
  audioSource: "microphone",
  inputDevice: null,
  keepHistory: true,
  identifySpeakers: false,
};

type SettingsState = {
  settings: Settings;
  loaded: boolean;
  load: () => Promise<void>;
  update: (patch: Partial<Settings>) => Promise<void>;
};

export const useSettings = create<SettingsState>((set, get) => {
  /** Saves run one after another, so an older one can't overwrite a newer one. */
  let queue: Promise<void> = Promise.resolve();
  let pending = 0;

  async function save(previous: Partial<Settings>) {
    try {
      const saved = await commands.setSettings(get().settings);
      // The backend normalizes what it stores: show that, unless newer
      // changes are still waiting to be saved.
      if (pending === 1) set({ settings: saved });
    } catch (error) {
      // Undo only this change, keeping the ones made meanwhile.
      set((state) => ({ settings: { ...state.settings, ...previous } }));
      notifyError(error);
    } finally {
      pending -= 1;
    }
  }

  return {
    settings: DEFAULT_SETTINGS,
    loaded: false,

    load: async () => {
      try {
        set({ settings: await commands.getSettings(), loaded: true });
      } catch (error) {
        set({ loaded: true });
        notifyError(error);
      }
    },

    // Optimistic: the UI updates immediately and rolls back if saving fails.
    update: (patch) => {
      const current = get().settings;
      const previous = Object.fromEntries(
        Object.keys(patch).map((key) => [key, current[key as keyof Settings]]),
      ) as Partial<Settings>;
      set({ settings: { ...current, ...patch } });
      pending += 1;
      queue = queue.then(() => save(previous));
      return queue;
    },
  };
});
