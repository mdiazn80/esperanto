import { CloudIcon, CpuIcon, PlusIcon } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { ProfileForm } from "@/features/providers/profile-form";
import { hostOf } from "@/lib/format";
import { activeModel, useModels } from "@/store/models";
import { useProviders } from "@/store/providers";

const LOCAL = "local";
const NEW = "new";

/** One engine to choose: a radio button with an icon and two lines. */
function EngineOption({
  value,
  icon,
  title,
  detail,
  disabled,
  action,
}: {
  value: string;
  icon: ReactNode;
  title: string;
  detail: string;
  disabled?: boolean;
  action?: ReactNode;
}) {
  const id = `engine-${value}`;
  return (
    <div className="flex items-center gap-3 rounded-md border p-3">
      <RadioGroupItem id={id} value={value} disabled={disabled} />
      <Label
        htmlFor={id}
        className="flex min-w-0 flex-1 items-center gap-3 font-normal"
      >
        {icon}
        <span className="flex min-w-0 flex-col">
          <span className="font-medium">{title}</span>
          <span className="truncate text-xs text-muted-foreground">
            {detail}
          </span>
        </span>
      </Label>
      {action}
    </div>
  );
}

export function ProviderSettings() {
  const { t } = useTranslation();
  const view = useProviders((s) => s.view);
  const load = useProviders((s) => s.load);
  const setActive = useProviders((s) => s.setActive);
  const localModel = useModels((s) => activeModel(s.overview));
  // Which profile the form edits: a profile id, NEW, or none.
  const [editing, setEditing] = useState<string | null>(null);

  useEffect(() => {
    void load();
  }, [load]);

  if (!view) return null;
  const editingProfile = view.profiles.find((p) => p.id === editing) ?? null;
  // The edited profile may have been deleted.
  const showForm = editing === NEW || editingProfile !== null;
  const iconClass = "size-4 shrink-0 text-muted-foreground";

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-muted-foreground">{t("providers.intro")}</p>
      <RadioGroup
        value={view.active ?? LOCAL}
        aria-label={t("settings.tabs.provider")}
        onValueChange={(value) => {
          void setActive(value === LOCAL ? null : value);
          setEditing(value === LOCAL ? null : value);
        }}
      >
        <EngineOption
          value={LOCAL}
          icon={<CpuIcon className={iconClass} />}
          title={t("providers.local")}
          detail={
            localModel
              ? t("providers.localModel", { name: localModel.name })
              : t("status.noModel")
          }
        />
        {view.profiles.map((profile) => (
          <EngineOption
            key={profile.id}
            value={profile.id}
            disabled={!profile.hasToken}
            icon={<CloudIcon className={iconClass} />}
            title={profile.name}
            detail={[
              hostOf(profile.baseUrl),
              profile.model,
              ...(profile.hasToken ? [] : [t("providers.noToken")]),
            ].join(" · ")}
            action={
              <Button
                variant="ghost"
                size="sm"
                aria-expanded={editing === profile.id}
                onClick={() =>
                  setEditing(editing === profile.id ? null : profile.id)
                }
              >
                {t("providers.edit")}
              </Button>
            }
          />
        ))}
      </RadioGroup>

      {showForm ? (
        <ProfileForm
          // Remount when switching profiles so the form resets.
          key={editing}
          profile={editingProfile}
          onSaved={(id) => setEditing(id)}
          onDeleted={() => setEditing(null)}
        />
      ) : (
        <Button
          variant="outline"
          className="self-start"
          onClick={() => setEditing(NEW)}
        >
          <PlusIcon />
          {t("providers.add")}
        </Button>
      )}
      <p className="text-xs text-muted-foreground">
        {t("providers.appliesNextSession")}
      </p>
    </div>
  );
}
