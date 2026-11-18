import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type AudioInput } from "@/bindings";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  DownloadButton,
  DownloadProgress,
} from "@/features/models/download-controls";
import { speakerModel, useModels } from "@/store/models";
import { useSettings } from "@/store/settings";

/** Speaker labels (D24): the switch, and the model they need. */
function SpeakerSettings() {
  const { t } = useTranslation();
  const enabled = useSettings((s) => s.settings.identifySpeakers);
  const update = useSettings((s) => s.update);
  const model = useModels((s) => speakerModel(s.overview));
  const state = model?.status.state;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-1">
          <Label htmlFor="identify-speakers">{t("speakers.setting")}</Label>
          <p
            id="identify-speakers-hint"
            className="text-sm text-muted-foreground"
          >
            {t("speakers.settingHint")}
          </p>
        </div>
        <Switch
          id="identify-speakers"
          checked={enabled}
          aria-describedby="identify-speakers-hint"
          onCheckedChange={(checked) =>
            void update({ identifySpeakers: checked })
          }
        />
      </div>
      {enabled && model && state === "downloading" && (
        <DownloadProgress model={model} />
      )}
      {enabled &&
        model &&
        (state === "notInstalled" || state === "partial") && (
          <div className="flex flex-col items-start gap-2">
            <p className="text-sm text-muted-foreground">
              {t("speakers.needsModel", { license: model.licenseName })}
            </p>
            <DownloadButton model={model} />
          </div>
        )}
    </div>
  );
}

/** Select value for "follow the system default input". */
const SYSTEM_DEFAULT = "system-default";

export function AudioSettings() {
  const { t } = useTranslation();
  const inputDevice = useSettings((s) => s.settings.inputDevice);
  const update = useSettings((s) => s.update);
  const [inputs, setInputs] = useState<AudioInput[] | null>(null);

  useEffect(() => {
    commands.listAudioInputs().then(setInputs, (error: unknown) => {
      console.error(error);
      setInputs([]);
    });
  }, []);

  const savedIsMissing =
    inputDevice !== null &&
    inputs !== null &&
    !inputs.some((input) => input.id === inputDevice);

  return (
    <>
      <div className="flex flex-col gap-2">
        <Label htmlFor="input-device">{t("settings.inputDevice")}</Label>
        <Select
          value={inputDevice ?? SYSTEM_DEFAULT}
          disabled={inputs === null}
          onValueChange={(value) =>
            void update({
              inputDevice: value === SYSTEM_DEFAULT ? null : value,
            })
          }
        >
          <SelectTrigger
            id="input-device"
            className="w-full"
            aria-describedby="input-device-hint"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={SYSTEM_DEFAULT}>
              {t("settings.systemDefaultInput")}
            </SelectItem>
            {inputs?.map((input) => (
              <SelectItem key={input.id} value={input.id}>
                {input.name}
              </SelectItem>
            ))}
            {savedIsMissing && (
              <SelectItem value={inputDevice}>
                {t("settings.unavailableInput")}
              </SelectItem>
            )}
          </SelectContent>
        </Select>
        <p id="input-device-hint" className="text-sm text-muted-foreground">
          {t("settings.inputDeviceHint")}
        </p>
      </div>
      <SpeakerSettings />
    </>
  );
}
