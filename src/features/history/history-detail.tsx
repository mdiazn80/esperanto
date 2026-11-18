import { ArrowLeftIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { HistorySession_Serialize as HistorySession } from "@/bindings";
import { Button } from "@/components/ui/button";
import { ExportDialog } from "@/features/export/export-dialog";
import { formatClock, formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";

/** A saved session, read-only. */
export function HistoryDetail({
  session,
  onBack,
}: {
  session: HistorySession;
  onBack: () => void;
}) {
  const { t, i18n } = useTranslation();
  const translated = session.segments.some((s) => s.translation);
  const grid = translated
    ? "grid-cols-[4rem_minmax(0,1fr)_minmax(0,1fr)]"
    : "grid-cols-[4rem_minmax(0,1fr)]";
  const timeOf = (startMs: number | null) =>
    session.startedAt !== null && startMs !== null
      ? formatClock(session.startedAt + startMs, i18n.language)
      : "";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeftIcon />
          {t("history.back")}
        </Button>
        <span className="text-sm text-muted-foreground">
          {formatDateTime(session.startedAt ?? 0, i18n.language)}
        </span>
        <div className="ml-auto">
          <ExportDialog
            historyId={session.id}
            startedAt={session.startedAt ?? 0}
            hasTranslation={translated}
          />
        </div>
      </div>
      <div role="table" className="flex flex-col">
        <div
          role="row"
          className={cn("eyebrow grid gap-x-4 border-b py-2", grid)}
        >
          <span role="columnheader">{t("transcript.columns.time")}</span>
          <span role="columnheader">{t("transcript.columns.original")}</span>
          {translated && (
            <span role="columnheader">
              {t("transcript.columns.translation")}
            </span>
          )}
        </div>
        {session.segments.map((segment, index) => (
          <div
            key={index}
            role="row"
            className={cn("grid gap-x-4 border-b border-border/70 py-3", grid)}
          >
            <span
              role="cell"
              className="font-mono text-sm text-muted-foreground tabular-nums"
            >
              {timeOf(segment.startMs)}
            </span>
            <p
              role="cell"
              lang={segment.language}
              className={cn(
                "text-sm leading-relaxed",
                translated && "text-original",
              )}
            >
              {segment.text}
            </p>
            {translated && (
              <p
                role="cell"
                lang={segment.translationLanguage ?? undefined}
                className="text-sm leading-relaxed"
              >
                {segment.translation}
              </p>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
