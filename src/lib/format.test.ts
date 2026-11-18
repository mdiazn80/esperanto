import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatClock,
  formatElapsed,
  formatDuration,
  formatPercent,
  formatSeconds,
  hostOf,
} from "@/lib/format";

describe("formatBytes", () => {
  it("uses decimal units and the locale's number format", () => {
    expect(formatBytes(3_188_716_000, "en")).toBe("3.2 GB");
    expect(formatBytes(3_188_716_000, "es")).toBe("3,2 GB");
    expect(formatBytes(715_714_080, "en")).toBe("715.7 MB");
    expect(formatBytes(512, "en")).toBe("512 byte");
  });
});

describe("formatPercent", () => {
  it("rounds to whole percents", () => {
    expect(formatPercent(0.374, "en")).toBe("37%");
  });
});

describe("formatSeconds", () => {
  it("shows one decimal in the locale's format", () => {
    expect(formatSeconds(1234, "en")).toBe("1.2");
    expect(formatSeconds(1234, "es")).toBe("1,2");
    expect(formatSeconds(2000, "en")).toBe("2.0");
  });
});

describe("hostOf", () => {
  it("extracts the host, keeping the port", () => {
    expect(hostOf("https://gateway.example.com/v1")).toBe(
      "gateway.example.com",
    );
    expect(hostOf("http://192.168.1.5:8080/v1")).toBe("192.168.1.5:8080");
  });

  it("returns invalid URLs unchanged", () => {
    expect(hostOf("not a url")).toBe("not a url");
  });
});

describe("formatDuration", () => {
  it("rounds to the largest useful unit", () => {
    expect(formatDuration(45_400, "en")).toBe("45 sec");
    expect(formatDuration(12 * 60_000 + 20_000, "en")).toBe("12 min");
    expect(formatDuration(65 * 60_000, "en")).toBe("1 hr 5 min");
    expect(formatDuration(120 * 60_000, "en")).toBe("2 hr");
  });
});

describe("formatElapsed", () => {
  it("formats a stopwatch with hours, minutes and seconds", () => {
    expect(formatElapsed(0)).toBe("00:00:00");
    expect(formatElapsed(754_900)).toBe("00:12:34");
    expect(formatElapsed(3_723_000)).toBe("01:02:03");
  });

  it("never goes below zero", () => {
    expect(formatElapsed(-5000)).toBe("00:00:00");
  });
});

describe("formatClock", () => {
  it("shows hours and minutes", () => {
    const ms = new Date(2026, 10, 18, 9, 5).getTime();
    expect(formatClock(ms, "es")).toBe("09:05");
  });
});
