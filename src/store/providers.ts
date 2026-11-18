import { create } from "zustand";
import {
  commands,
  type ConnectionTest,
  type ProfileInput,
  type ProfileView,
  type ProvidersView,
} from "@/bindings";
import { notifyError } from "@/lib/notify";

type ProvidersState = {
  view: ProvidersView | null;
  load: () => Promise<void>;
  /** Saves a profile and, if given, its token. Returns the saved profile id. */
  save: (profile: ProfileInput, token: string) => Promise<string | null>;
  remove: (id: string) => Promise<void>;
  setActive: (id: string | null) => Promise<void>;
  clearToken: (id: string) => Promise<void>;
  test: (id: string) => Promise<ConnectionTest>;
};

export const useProviders = create<ProvidersState>((set, get) => {
  /** Runs a command that returns the new view, reporting failures. */
  async function apply(command: () => Promise<ProvidersView>) {
    try {
      set({ view: await command() });
    } catch (error) {
      notifyError(error);
    }
  }

  return {
    view: null,

    load: () => apply(commands.listProviders),

    save: async (profile, token) => {
      let id: string;
      try {
        ({ id } = await commands.saveProvider(profile));
      } catch (error) {
        notifyError(error);
        return null;
      }
      try {
        if (token.trim()) await commands.setProviderToken(id, token);
      } catch (error) {
        // The profile itself was saved: keep editing it.
        notifyError(error);
      }
      await get().load();
      return id;
    },

    remove: (id) => apply(() => commands.deleteProvider(id)),

    setActive: (id) => apply(() => commands.setActiveProvider(id)),

    clearToken: async (id) => {
      try {
        await commands.clearProviderToken(id);
      } catch (error) {
        notifyError(error);
      }
      await get().load();
    },

    test: (id) => commands.testProvider(id),
  };
});

/** The active remote profile, or `null` when the local model is used. */
export function activeRemote(view: ProvidersView | null): ProfileView | null {
  if (!view?.active) return null;
  return view.profiles.find((p) => p.id === view.active) ?? null;
}
