import type { Settings_Serialize as Settings } from "@/bindings";

/** Device id that captures system audio (`SYSTEM_AUDIO` in esperanto-audio). */
export const SYSTEM_AUDIO = "system:output";

/** The device a session should open for the current settings. */
export function deviceIdFor(settings: Settings): string | null {
  return settings.audioSource === "system"
    ? SYSTEM_AUDIO
    : settings.inputDevice;
}
