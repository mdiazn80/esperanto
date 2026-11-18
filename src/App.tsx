import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { SessionPanel } from "@/components/layout/session-panel";
import { SpeakersPanel } from "@/components/layout/speakers-panel";
import { Toolbar } from "@/components/layout/toolbar";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { TranscriptView } from "@/features/transcript/transcript-view";
import { resolveUiLanguage } from "@/lib/i18n";
import { applyTheme, useResolvedTheme } from "@/lib/theme";
import { useModels } from "@/store/models";
import { useProviders } from "@/store/providers";
import { useSettings } from "@/store/settings";

export default function App() {
  const { i18n } = useTranslation();
  const { settings, loaded, load } = useSettings();
  const theme = useResolvedTheme(settings.theme);

  const loadModels = useModels((s) => s.load);
  const loadProviders = useProviders((s) => s.load);

  useEffect(() => {
    void load();
    void loadModels();
    void loadProviders();
  }, [load, loadModels, loadProviders]);

  useEffect(() => applyTheme(theme), [theme]);

  useEffect(() => {
    const language = resolveUiLanguage(
      settings.uiLanguage,
      navigator.languages,
    );
    void i18n.changeLanguage(language);
    document.documentElement.lang = language;
  }, [i18n, settings.uiLanguage]);

  return (
    <TooltipProvider>
      {loaded && (
        <div className="flex h-screen flex-col bg-background text-foreground">
          <Toolbar />
          <div className="flex min-h-0 flex-1">
            <SpeakersPanel />
            <main className="flex min-w-0 flex-1 flex-col">
              <TranscriptView />
            </main>
            <SessionPanel />
          </div>
        </div>
      )}
      <Toaster theme={theme} position="bottom-right" />
    </TooltipProvider>
  );
}
