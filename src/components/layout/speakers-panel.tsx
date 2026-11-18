import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { speakerColor } from "@/lib/speakers";
import { cn } from "@/lib/utils";
import { currentSpeaker, speakersOf, useSession } from "@/store/session";
import { useSettings } from "@/store/settings";

/** One speaker: their color and name; open it to rename them. */
function SpeakerCard({
  speaker,
  open,
  speaking,
  onToggle,
}: {
  speaker: number;
  open: boolean;
  speaking: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();
  const custom = useSession((s) => s.speakerNames[speaker]);
  const rename = useSession((s) => s.renameSpeaker);
  const fallback = t("speakers.default", { number: speaker + 1 });
  const name = custom || fallback;
  const color = speakerColor(speaker);
  const inputId = `speaker-name-${speaker}`;

  return (
    <li
      className={cn(
        "rounded-lg border border-transparent",
        open && "border-border bg-card",
      )}
    >
      <button
        type="button"
        aria-expanded={open}
        title={t("speakers.renameHint")}
        className="flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-left hover:bg-muted/50 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        onClick={onToggle}
      >
        <span
          aria-hidden="true"
          className={cn("size-3 shrink-0 rounded-[3px]", color.swatch)}
        />
        <span className="min-w-0 flex-1 truncate text-sm font-medium">
          {name}
        </span>
        {speaking && (
          <span className={cn("font-mono text-xs", color.text)}>
            {t("speakers.speaking")}
          </span>
        )}
      </button>
      {open && (
        <div className="flex flex-col gap-1.5 px-3 pb-3">
          <Label htmlFor={inputId} className="text-xs text-muted-foreground">
            {t("speakers.name")}
          </Label>
          <Input
            id={inputId}
            autoFocus
            defaultValue={custom ?? ""}
            placeholder={t("speakers.unnamed")}
            aria-label={t("speakers.rename", { name })}
            onBlur={(e) => rename(speaker, e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                rename(speaker, e.currentTarget.value);
                onToggle();
              }
              if (e.key === "Escape") onToggle();
            }}
          />
        </div>
      )}
    </li>
  );
}

/** Left column: who is speaking in this session. */
export function SpeakersPanel() {
  const { t } = useTranslation();
  const segments = useSession((s) => s.segments);
  const status = useSession((s) => s.status);
  const labels = useSession((s) => s.speakerLabels);
  const identify = useSettings((s) => s.settings.identifySpeakers);
  const [open, setOpen] = useState<number | null>(null);
  const speakers = speakersOf(segments);
  const speaking = currentSpeaker(segments);
  const running = status === "listening" || status === "stopping";

  let empty: string | null = null;
  if (speakers.length === 0) {
    empty =
      !identify || (running && !labels)
        ? t("speakers.off")
        : t("speakers.waiting");
  }

  return (
    <aside
      aria-labelledby="speakers-title"
      className="flex w-72 shrink-0 flex-col border-r bg-sidebar"
    >
      <h2 id="speakers-title" className="eyebrow px-4 pt-5 pb-3">
        {t("speakers.title")}
        {speakers.length > 0 && ` · ${speakers.length}`}
      </h2>
      <div className="min-h-0 flex-1 overflow-y-auto px-2">
        {empty ? (
          <p className="px-2 text-sm text-muted-foreground">{empty}</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {speakers.map((speaker) => (
              <SpeakerCard
                key={speaker}
                speaker={speaker}
                open={open === speaker}
                speaking={speaking === speaker}
                onToggle={() =>
                  setOpen((current) => (current === speaker ? null : speaker))
                }
              />
            ))}
          </ul>
        )}
      </div>
      {speakers.length > 0 && (
        <p className="px-4 py-4 text-xs text-muted-foreground">
          {t("speakers.hint")}
        </p>
      )}
    </aside>
  );
}
