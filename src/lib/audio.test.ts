import { describe, expect, it } from "vitest";
import { deviceIdFor, SYSTEM_AUDIO } from "@/lib/audio";
import { DEFAULT_SETTINGS } from "@/store/settings";

describe("deviceIdFor", () => {
  it("uses the chosen microphone, or the system default", () => {
    expect(deviceIdFor(DEFAULT_SETTINGS)).toBeNull();
    expect(deviceIdFor({ ...DEFAULT_SETTINGS, inputDevice: "mic-1" })).toBe(
      "mic-1",
    );
  });

  it("captures system audio regardless of the microphone", () => {
    expect(
      deviceIdFor({
        ...DEFAULT_SETTINGS,
        audioSource: "system",
        inputDevice: "mic-1",
      }),
    ).toBe(SYSTEM_AUDIO);
  });
});
