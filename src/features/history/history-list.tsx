import { CloudIcon, HistoryIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import type {
  HistorySummary_Serialize as HistorySummary,
  Language,
} from "@/bindings";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { formatDateTime, formatDuration } from "@/lib/format";
import { useHistory } from "@/store/history";
import { useSettings } from "@/store/settings";

function languagesOf(
  summary: HistorySummary,
  name: (lang: Language) => string,
): string {
  const spoken = summary.languages.map(name).join(", ");
  const translated = summary.translationLanguages.map(name).join(", ");
  return translated ? `${spoken} → ${translated}` : spoken;
}

function SessionRow({
  summary,
  onOpen,
}: {
  summary: HistorySummary;
  onOpen: (id: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const remove = useHistory((s) => s.remove);
  const locale = i18n.language;
  const date = formatDateTime(summary.startedAt ?? 0, locale);

  return (
    <li className="flex items-start gap-2 rounded-lg border bg-card">
      <button
        type="button"
        className="flex min-w-0 flex-1 flex-col gap-1 rounded-lg p-3 text-left hover:bg-muted/50 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        onClick={() => onOpen(summary.id)}
      >
        <span className="flex flex-wrap items-center gap-x-2 text-sm font-medium">
          <span className="font-mono tabular-nums">{date}</span>
          <span className="font-normal text-muted-foreground">
            {formatDuration(summary.durationMs ?? 0, locale)} ·{" "}
            {languagesOf(summary, (lang) => t(`languages.${lang}`))}
          </span>
          {summary.remoteHost && (
            <span className="flex items-center gap-1 font-normal text-muted-foreground">
              <CloudIcon className="size-3" />
              {summary.remoteHost}
            </span>
          )}
        </span>
        <span className="line-clamp-2 text-sm text-muted-foreground">
          {summary.preview}
        </span>
      </button>
      <ConfirmDeleteButton
        size="sm"
        className="mt-2 mr-2"
        label={t("common.delete")}
        title={t("history.deleteTitle", { date })}
        description={t("history.deleteBody")}
        onConfirm={() => void remove(summary.id)}
      />
    </li>
  );
}

/** Saved sessions, newest first. */
export function HistoryList({ onOpen }: { onOpen: (id: string) => void }) {
  const { t } = useTranslation();
  const sessions = useHistory((s) => s.sessions);
  const keep = useSettings((s) => s.settings.keepHistory);

  if (sessions === null) return null;
  if (sessions.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 py-10 text-center">
        <HistoryIcon
          className="size-8 text-muted-foreground"
          strokeWidth={1.5}
        />
        <p className="text-sm text-muted-foreground">
          {keep ? t("history.empty") : t("history.off")}
        </p>
      </div>
    );
  }
  return (
    <ul className="flex flex-col gap-2">
      {sessions.map((summary) => (
        <SessionRow key={summary.id} summary={summary} onOpen={onOpen} />
      ))}
    </ul>
  );
}
