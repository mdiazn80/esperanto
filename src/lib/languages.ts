import type { Language, SourceLanguage, TargetLanguage } from "@/bindings";

/** Transcription/translation languages supported in v1. */
export const LANGUAGES: readonly Language[] = ["en", "es"];

export type TargetOption = "none" | Language;

/**
 * The spoken language to show. Automatic detection is disabled:
 * the backend always stores a fixed language, so `auto` only appears before
 * the settings load.
 */
export function toSourceOption(source: SourceLanguage): Language {
  return source.kind === "fixed" ? source.language : LANGUAGES[0];
}

export function fromSourceOption(language: Language): SourceLanguage {
  return { kind: "fixed", language };
}

export function toTargetOption(target: TargetLanguage): TargetOption {
  return target.kind === "none" ? "none" : target.language;
}

export function fromTargetOption(option: TargetOption): TargetLanguage {
  return option === "none"
    ? { kind: "none" }
    : { kind: "translate", language: option };
}
