import { describe, expect, it } from "vitest";
import { meterFraction } from "@/lib/level";

describe("meterFraction", () => {
  it("maps the dB range to 0–1", () => {
    expect(meterFraction(-80)).toBe(0);
    expect(meterFraction(-60)).toBe(0);
    expect(meterFraction(-30)).toBeCloseTo(0.5);
    expect(meterFraction(0)).toBe(1);
    expect(meterFraction(6)).toBe(1);
  });
});
