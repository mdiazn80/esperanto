import { describe, expect, it } from "vitest";
import {
  fromSourceOption,
  fromTargetOption,
  toSourceOption,
  toTargetOption,
} from "@/lib/languages";

describe("language options", () => {
  it("round-trips source languages", () => {
    for (const option of ["en", "es"] as const) {
      expect(toSourceOption(fromSourceOption(option))).toBe(option);
    }
  });

  it("shows a fixed language when the source is auto", () => {
    expect(toSourceOption({ kind: "auto" })).toBe("en");
  });

  it("round-trips target languages", () => {
    for (const option of ["none", "en", "es"] as const) {
      expect(toTargetOption(fromTargetOption(option))).toBe(option);
    }
  });

  it("matches the Rust wire format", () => {
    expect(fromTargetOption("es")).toEqual({
      kind: "translate",
      language: "es",
    });
    expect(fromSourceOption("es")).toEqual({ kind: "fixed", language: "es" });
  });
});
