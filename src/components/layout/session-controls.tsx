import { MicIcon, MonitorSpeakerIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AudioSource, Language } from "@/bindings";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  LANGUAGES,
  fromSourceOption,
  fromTargetOption,
  toSourceOption,
  toTargetOption,
  type TargetOption,
} from "@/lib/languages";
import { useSession } from "@/store/session";
import { useSettings } from "@/store/settings";

/** The supported languages as select items. */
function LanguageItems() {
  const { t } = useTranslation();
  return LANGUAGES.map((lang) => (
    <SelectItem key={lang} value={lang}>
      {t(`languages.${lang}`)}
    </SelectItem>
  ));
}

/** The selected item in brand yellow. `aria-checked` covers the item wrapped
 * in a tooltip, which overrides `data-state`. */
const TOGGLE_ITEM =
  "min-w-0 flex-1 gap-1.5 px-2 text-[0.8rem] aria-checked:bg-primary aria-checked:text-primary-foreground aria-checked:hover:bg-primary/90 data-[state=on]:bg-primary data-[state=on]:text-primary-foreground data-[state=on]:hover:bg-primary/90";

/** Microphone or system audio. Fixed while a session runs. */
export function SourceToggle() {
  const { t } = useTranslation();
  const source = useSettings((s) => s.settings.audioSource);
  const update = useSettings((s) => s.update);
  const status = useSession((s) => s.status);

  return (
    <ToggleGroup
      type="single"
      variant="outline"
      spacing={0}
      value={source}
      disabled={status !== "idle"}
      aria-label={t("toolbar.source")}
      className="w-full"
      onValueChange={(value) => {
        // Radix sends "" when the active item is clicked again: keep it.
        if (value) void update({ audioSource: value as AudioSource });
      }}
    >
      <ToggleGroupItem value="microphone" className={TOGGLE_ITEM}>
        <MicIcon />
        {t("toolbar.sourceMicrophone")}
      </ToggleGroupItem>
      <Tooltip>
        <TooltipTrigger asChild>
          <ToggleGroupItem value="system" className={TOGGLE_ITEM}>
            <MonitorSpeakerIcon />
            {t("toolbar.sourceSystemShort")}
          </ToggleGroupItem>
        </TooltipTrigger>
        <TooltipContent>{t("toolbar.sourceSystem")}</TooltipContent>
      </Tooltip>
    </ToggleGroup>
  );
}

/** Spoken language and translation language. They can change mid-session:
 * the next phrase uses them. */
export function LanguageSelects() {
  const { t } = useTranslation();
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-1.5">
        <Label
          htmlFor="source-language"
          className="text-xs text-muted-foreground"
        >
          {t("toolbar.sourceLanguage")}
        </Label>
        <Select
          value={toSourceOption(settings.sourceLanguage)}
          onValueChange={(value) =>
            void update({
              sourceLanguage: fromSourceOption(value as Language),
            })
          }
        >
          <SelectTrigger id="source-language" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <LanguageItems />
          </SelectContent>
        </Select>
      </div>
      <div className="flex flex-col gap-1.5">
        <Label
          htmlFor="target-language"
          className="text-xs text-muted-foreground"
        >
          {t("toolbar.targetLanguage")}
        </Label>
        <Select
          value={toTargetOption(settings.targetLanguage)}
          onValueChange={(value) =>
            void update({
              targetLanguage: fromTargetOption(value as TargetOption),
            })
          }
        >
          <SelectTrigger id="target-language" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">{t("languages.none")}</SelectItem>
            <LanguageItems />
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}
