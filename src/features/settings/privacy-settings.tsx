import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useHistory } from "@/store/history";
import { useSettings } from "@/store/settings";

export function PrivacySettings() {
  const { t } = useTranslation();
  const keep = useSettings((s) => s.settings.keepHistory);
  const update = useSettings((s) => s.update);
  const clear = useHistory((s) => s.clear);

  return (
    <>
      <div className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <Label htmlFor="keep-history">{t("privacy.keepHistory")}</Label>
          <p id="keep-history-hint" className="text-sm text-muted-foreground">
            {t("privacy.keepHistoryHint")}
          </p>
        </div>
        <Switch
          id="keep-history"
          checked={keep}
          aria-describedby="keep-history-hint"
          onCheckedChange={(checked) => void update({ keepHistory: checked })}
        />
      </div>
      <div className="flex flex-col items-start gap-1">
        <ConfirmDeleteButton
          label={t("privacy.deleteAll")}
          title={t("privacy.deleteAllTitle")}
          description={t("privacy.deleteAllBody")}
          onConfirm={() =>
            void clear().then(
              (done) => done && toast.success(t("privacy.deleted")),
            )
          }
        />
      </div>
    </>
  );
}
