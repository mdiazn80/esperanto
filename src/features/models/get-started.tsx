import { useTranslation } from "react-i18next";
import {
  DownloadButton,
  DownloadProgress,
} from "@/features/models/download-controls";
import { LicenseLink } from "@/features/models/license-link";
import { formatBytes } from "@/lib/format";
import { recommendedModel, useModels } from "@/store/models";

/** First run: no model installed yet. Offers the recommended one. */
export function GetStarted() {
  const { t, i18n } = useTranslation();
  const model = useModels((s) => recommendedModel(s.overview));
  if (!model) return null;
  const size = formatBytes(model.sizeBytes ?? 0, i18n.language);

  return (
    <div className="flex w-full max-w-md flex-col gap-4 rounded-lg border p-6 text-left">
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-medium">{t("getStarted.title")}</h2>
        <p className="text-sm text-muted-foreground">
          {t("getStarted.body", { name: model.name })}
        </p>
      </div>

      <ol className="flex list-decimal flex-col gap-1 pl-5 text-sm">
        <li>{t("getStarted.stepDownload", { size })}</li>
        <li>{t("getStarted.stepLanguages")}</li>
        <li>{t("getStarted.stepStart")}</li>
      </ol>

      {model.status.state === "downloading" ? (
        <DownloadProgress model={model} />
      ) : (
        <div className="flex flex-col gap-2">
          <DownloadButton model={model} />
          <p className="text-xs text-muted-foreground">
            {t("getStarted.license")}{" "}
            <LicenseLink model={model} className="text-xs" />
          </p>
        </div>
      )}
    </div>
  );
}
