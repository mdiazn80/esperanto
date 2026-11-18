import { useEffect, useState } from "react";
import type { Theme } from "@/bindings";

type ResolvedTheme = "light" | "dark";

const DARK_QUERY = "(prefers-color-scheme: dark)";

function systemTheme(): ResolvedTheme {
  return window.matchMedia(DARK_QUERY).matches ? "dark" : "light";
}

/** Resolves the theme setting, following OS changes when set to "system". */
export function useResolvedTheme(theme: Theme): ResolvedTheme {
  const [system, setSystem] = useState<ResolvedTheme>(systemTheme);

  useEffect(() => {
    const media = window.matchMedia(DARK_QUERY);
    const onChange = () => setSystem(systemTheme());
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);

  return theme === "system" ? system : theme;
}

export function applyTheme(theme: ResolvedTheme) {
  const root = document.documentElement;
  root.classList.toggle("dark", theme === "dark");
  root.style.colorScheme = theme;
}
