import { describe, expect, it } from "vitest";
import { errorKey } from "@/lib/errors";

describe("errorKey", () => {
  it("maps command error codes to translation keys", () => {
    expect(errorKey({ code: "settingsSaveFailed", detail: "disk full" })).toBe(
      "errors.settingsSaveFailed",
    );
  });

  it("falls back to a generic message", () => {
    expect(errorKey(new Error("boom"))).toBe("errors.unknown");
    expect(errorKey({ code: "somethingElse" })).toBe("errors.unknown");
    expect(errorKey("text")).toBe("errors.unknown");
  });
});
