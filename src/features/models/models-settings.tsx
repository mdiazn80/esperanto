import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { ModelInfo } from "@/bindings";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { Badge } from "@/components/ui/badge";
import {
  DownloadButton,
  DownloadProgress,
} from "@/features/models/download-controls";
import { LicenseLink } from "@/features/models/license-link";
import { formatBytes } from "@/lib/format";
import { useModels } from "@/store/models";

function DeleteButton({ model }: { model: ModelInfo }) {
  const { t } = useTranslation();
  const remove = useModels((s) => s.remove);
  return (
    <ConfirmDeleteButton
      size="sm"
      label={t("common.delete")}
      title={t("models.deleteTitle", { name: model.name })}
      description={t("models.deleteBody")}
      onConfirm={() => void remove(model.id)}
    />
  );
}

function ModelCard({ model }: { model: ModelInfo }) {
  const { t, i18n } = useTranslation();
  const { status } = model;
  const languages = model.languages
    .map((lang) => t(`languages.${lang}`))
    .join(", ");

  return (
    <div className="flex flex-col gap-3 rounded-lg border p-4">
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <h3 className="font-medium">{model.name}</h3>
          {model.recommended && (
            <Badge variant="secondary">{t("models.recommended")}</Badge>
          )}
          {status.state === "installed" && (
            <Badge>{t("models.installed")}</Badge>
          )}
        </div>
        <p className="text-sm text-muted-foreground">
          {t(`models.descriptions.${model.id}`, {
            defaultValue: model.description,
          })}
        </p>
      </div>

      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <dt className="text-muted-foreground">{t("models.size")}</dt>
        <dd>{formatBytes(model.sizeBytes ?? 0, i18n.language)}</dd>
        {languages && (
          <>
            <dt className="text-muted-foreground">{t("models.languages")}</dt>
            <dd>{languages}</dd>
          </>
        )}
        <dt className="text-muted-foreground">{t("models.memory")}</dt>
        <dd>{t("models.memoryValue", { gb: model.minRamGb })}</dd>
        <dt className="text-muted-foreground">{t("models.license")}</dt>
        <dd>
          <LicenseLink model={model} />
        </dd>
      </dl>

      {status.state === "downloading" && <DownloadProgress model={model} />}

      {(status.state === "notInstalled" || status.state === "partial") && (
        <div className="flex flex-col gap-2">
          <p className="text-xs text-muted-foreground">
            {t("models.licenseNotice", { license: model.licenseName })}
          </p>
          <div className="flex items-center gap-2">
            <DownloadButton model={model} />
            {status.state === "partial" && <DeleteButton model={model} />}
          </div>
        </div>
      )}

      {status.state === "installed" && (
        <div>
          <DeleteButton model={model} />
        </div>
      )}
    </div>
  );
}

export function ModelsSettings() {
  const { t, i18n } = useTranslation();
  const overview = useModels((s) => s.overview);
  const load = useModels((s) => s.load);

  // Refresh the disk usage and any download started elsewhere.
  useEffect(() => {
    void load();
  }, [load]);

  if (!overview) return null;
  return (
    <div className="flex flex-col gap-4">
      {overview.models.map((model) => (
        <ModelCard key={model.id} model={model} />
      ))}
      <p className="text-sm text-muted-foreground">
        {t("models.diskUsage", {
          size: formatBytes(overview.diskUsageBytes ?? 0, i18n.language),
        })}
      </p>
    </div>
  );
}
