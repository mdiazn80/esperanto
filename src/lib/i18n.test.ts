import { describe, expect, it } from "vitest";
import { resolveUiLanguage, resources } from "@/lib/i18n";

function keysOf(value: unknown, prefix = ""): string[] {
  if (typeof value !== "object" || value === null) return [prefix];
  return Object.entries(value).flatMap(([key, child]) =>
    keysOf(child, prefix ? `${prefix}.${key}` : key),
  );
}

describe("resolveUiLanguage", () => {
  it("uses the explicit setting", () => {
    expect(resolveUiLanguage("es", ["en-US"])).toBe("es");
  });

  it("follows the first supported system language", () => {
    expect(resolveUiLanguage("system", ["fr-FR", "es-ES", "en"])).toBe("es");
  });

  it("falls back to English", () => {
    expect(resolveUiLanguage("system", ["fr-FR", "de"])).toBe("en");
    expect(resolveUiLanguage("system", [])).toBe("en");
  });
});

describe("locales", () => {
  it("every locale has exactly the same keys as English", () => {
    const english = keysOf(resources.en.common).sort();
    for (const [lang, { common }] of Object.entries(resources)) {
      expect(keysOf(common).sort(), `locale "${lang}"`).toEqual(english);
    }
  });
});
