import { useTranslation } from "react-i18next";
import type { Theme, UiLanguage } from "@/bindings";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { UI_LANGUAGES } from "@/lib/i18n";
import { useSettings } from "@/store/settings";

const UI_LANGUAGE_OPTIONS: readonly UiLanguage[] = ["system", ...UI_LANGUAGES];
const THEME_OPTIONS: readonly Theme[] = ["system", "light", "dark"];

export function GeneralSettings() {
  const { t } = useTranslation();
  const uiLanguage = useSettings((s) => s.settings.uiLanguage);
  const theme = useSettings((s) => s.settings.theme);
  const update = useSettings((s) => s.update);

  return (
    <>
      <div className="flex flex-col gap-2">
        <Label htmlFor="ui-language">{t("settings.uiLanguage")}</Label>
        <Select
          value={uiLanguage}
          onValueChange={(value) =>
            void update({ uiLanguage: value as UiLanguage })
          }
        >
          <SelectTrigger
            id="ui-language"
            className="w-full"
            aria-describedby="ui-language-hint"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {UI_LANGUAGE_OPTIONS.map((option) => (
              <SelectItem key={option} value={option}>
                {t(`uiLanguages.${option}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p id="ui-language-hint" className="text-sm text-muted-foreground">
          {t("settings.uiLanguageHint")}
        </p>
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="theme">{t("settings.theme")}</Label>
        <Select
          value={theme}
          onValueChange={(value) => void update({ theme: value as Theme })}
        >
          <SelectTrigger id="theme" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {THEME_OPTIONS.map((option) => (
              <SelectItem key={option} value={option}>
                {t(`settings.themes.${option}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </>
  );
}
