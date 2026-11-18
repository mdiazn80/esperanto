import { DownloadIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ModelInfo } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { formatBytes, formatPercent } from "@/lib/format";
import { useModels } from "@/store/models";

/** Download progress with a pause button, while a model downloads. */
export function DownloadProgress({ model }: { model: ModelInfo }) {
  const { t, i18n } = useTranslation();
  const progress = useModels((s) => s.progress[model.id]);
  const cancel = useModels((s) => s.cancel);
  const locale = i18n.language;
  const fraction =
    progress && progress.total > 0 ? progress.downloaded / progress.total : 0;
  const label = progress
    ? t("models.progress", {
        downloaded: formatBytes(progress.downloaded, locale),
        total: formatBytes(progress.total, locale),
        percent: formatPercent(fraction, locale),
      })
    : t("models.preparing");

  return (
    <div className="flex flex-col gap-2">
      <Progress
        value={fraction * 100}
        aria-label={t("models.downloading", { name: model.name })}
        aria-valuetext={label}
      />
      <div className="flex items-center justify-between text-sm text-muted-foreground">
        <span className="tabular-nums">{label}</span>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void cancel(model.id)}
        >
          {t("models.cancelDownload")}
        </Button>
      </div>
    </div>
  );
}

/** Starts or resumes a model's download. */
export function DownloadButton({ model }: { model: ModelInfo }) {
  const { t, i18n } = useTranslation();
  const download = useModels((s) => s.download);
  const locale = i18n.language;
  const size = formatBytes(model.sizeBytes ?? 0, locale);

  return (
    <Button onClick={() => void download(model.id)}>
      <DownloadIcon />
      {model.status.state === "partial"
        ? t("models.resume", {
            downloaded: formatBytes(model.status.downloadedBytes ?? 0, locale),
            total: size,
          })
        : t("models.download", { size })}
    </Button>
  );
}
