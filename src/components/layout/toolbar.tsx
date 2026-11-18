import { useEffect, useState } from "react";
import {
  BookmarkPlusIcon,
  Loader2Icon,
  PlayIcon,
  SquareIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { AppLogo } from "@/components/layout/app-logo";
import { ExportDialog } from "@/features/export/export-dialog";
import { HistoryDialog } from "@/features/history/history-dialog";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { deviceIdFor } from "@/lib/audio";
import { formatElapsed } from "@/lib/format";
import { toSourceOption, toTargetOption } from "@/lib/languages";
import { isVisible } from "@/lib/transcript";
import { useSession } from "@/store/session";
import { useSettings } from "@/store/settings";

/** "REC 00:12:34" and what is being transcribed, while listening. */
function Recording() {
  const { t } = useTranslation();
  const active = useSession((s) => s.active);
  const settings = useSettings((s) => s.settings);
  const startedAt = useSession((s) => s.lastSession?.startedAt ?? null);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const elapsed = formatElapsed(startedAt === null ? 0 : now - startedAt);
  const system = active?.source === "system";
  const target = toTargetOption(settings.targetLanguage);

  return (
    <div className="flex min-w-0 items-center gap-4 font-mono text-sm">
      <span
        role="timer"
        aria-label={t("toolbar.recording", { elapsed })}
        className="flex shrink-0 items-center gap-2 rounded-md bg-rec-surface px-2.5 py-1 text-rec tabular-nums"
      >
        <span
          aria-hidden="true"
          className="size-2 animate-pulse rounded-full bg-rec"
        />
        <span aria-hidden="true">REC {elapsed}</span>
      </span>
      <span
        className="tracking-wide text-muted-foreground"
        title={active?.deviceName}
      >
        {system ? t("toolbar.systemShort") : t("toolbar.micShort")}
      </span>
      <span className="tracking-wide text-muted-foreground uppercase">
        {toSourceOption(settings.sourceLanguage)}
        {target !== "none" && <> → {target}</>}
      </span>
    </div>
  );
}

/** The window's top bar; it is also where the window is dragged from. */
export function Toolbar() {
  const { t } = useTranslation();
  const settings = useSettings((s) => s.settings);
  const status = useSession((s) => s.status);
  const start = useSession((s) => s.start);
  const stop = useSession((s) => s.stop);
  const markMoment = useSession((s) => s.markMoment);
  const lastSession = useSession((s) => s.lastSession);
  const hasSegments = useSession((s) => s.segments.some(isVisible));
  const hasTranslation = useSession((s) =>
    s.segments.some((segment) => segment.translation),
  );
  const busy = status === "starting" || status === "stopping";
  const running = status === "listening" || status === "stopping";

  function onMark() {
    if (markMoment()) toast.success(t("toolbar.marked"));
    else toast.info(t("toolbar.nothingToMark"));
  }

  return (
    <header
      data-tauri-drag-region
      className="flex h-14 shrink-0 items-center gap-4 border-b bg-sidebar pr-3 pl-21"
    >
      <div
        data-tauri-drag-region
        className="flex shrink-0 items-center gap-2.5"
      >
        <AppLogo className="size-7" />
        <span className="font-mono text-sm font-medium tracking-[0.12em] uppercase">
          {t("app.name")}
        </span>
      </div>

      {running && <Recording />}

      <div
        data-tauri-drag-region
        className="ml-auto flex shrink-0 items-center gap-2"
      >
        {status === "listening" && (
          <Button variant="outline" size="lg" onClick={onMark}>
            <BookmarkPlusIcon />
            {t("toolbar.mark")}
          </Button>
        )}
        {lastSession && hasSegments && (
          <ExportDialog
            size="lg"
            historyId={lastSession.historyId}
            startedAt={lastSession.startedAt}
            hasTranslation={hasTranslation}
          />
        )}
        {running ? (
          <Button size="lg" disabled={busy} onClick={() => void stop()}>
            <SquareIcon className="fill-current" />
            {t("toolbar.stop")}
          </Button>
        ) : (
          <Button
            size="lg"
            disabled={busy}
            onClick={() =>
              void start(deviceIdFor(settings), settings.audioSource)
            }
          >
            {busy ? (
              <Loader2Icon className="animate-spin" />
            ) : (
              <PlayIcon className="fill-current" />
            )}
            {t("toolbar.start")}
          </Button>
        )}
        <Separator orientation="vertical" className="mx-1 h-6" />
        <HistoryDialog />
        <SettingsDialog />
      </div>
    </header>
  );
}
