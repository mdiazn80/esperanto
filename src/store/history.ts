import { create } from "zustand";
import {
  commands,
  type HistorySession_Serialize as HistorySession,
  type HistorySummary_Serialize as HistorySummary,
} from "@/bindings";
import { notifyError } from "@/lib/notify";

type HistoryState = {
  /** Saved sessions, newest first; `null` until loaded. */
  sessions: HistorySummary[] | null;
  load: () => Promise<void>;
  open: (id: string) => Promise<HistorySession | null>;
  remove: (id: string) => Promise<void>;
  clear: () => Promise<boolean>;
};

export const useHistory = create<HistoryState>((set, get) => ({
  sessions: null,

  load: async () => {
    try {
      set({ sessions: await commands.listHistory() });
    } catch (error) {
      notifyError(error);
    }
  },

  open: async (id) => {
    try {
      return await commands.getHistorySession(id);
    } catch (error) {
      notifyError(error);
      return null;
    }
  },

  remove: async (id) => {
    try {
      await commands.deleteHistorySession(id);
    } catch (error) {
      notifyError(error);
    }
    await get().load();
  },

  clear: async () => {
    try {
      await commands.clearHistory();
      set({ sessions: [] });
      return true;
    } catch (error) {
      notifyError(error);
      await get().load();
      return false;
    }
  },
}));
