import {
  CaptionsIcon,
  DownloadIcon,
  MicOffIcon,
  ServerOffIcon,
  VolumeXIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { GetStarted } from "@/features/models/get-started";
import { runCommand } from "@/lib/notify";
import { activeModel, useModels } from "@/store/models";
import { activeRemote, useProviders } from "@/store/providers";
import { useSession } from "@/store/session";

function Hint({
  icon,
  title,
  children,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
}) {
  return (
    <Alert className="max-w-md text-left">
      {icon}
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>{children}</AlertDescription>
    </Alert>
  );
}

function SettingsButton({
  label,
  open,
}: {
  label: string;
  open: () => Promise<unknown>;
}) {
  return (
    <Button
      variant="outline"
      size="sm"
      className="mt-2"
      onClick={() => runCommand(open())}
    >
      {label}
    </Button>
  );
}

/** What to do next when idle: fix the last error, get a model, or start. */
export function Guidance() {
  const { t } = useTranslation();
  const error = useSession((s) => s.error);
  const overview = useModels((s) => s.overview);
  const hasModel = activeModel(overview) !== null;
  const remote = useProviders((s) => activeRemote(s.view));

  if (error === "microphonePermissionDenied") {
    return (
      <Hint icon={<MicOffIcon />} title={t("session.permissionTitle")}>
        <p>{t("session.permissionBody")}</p>
        <SettingsButton
          label={t("session.openMicrophoneSettings")}
          open={commands.openMicrophoneSettings}
        />
      </Hint>
    );
  }
  // First run, or the model was deleted: offer to download it right here.
  if (overview !== null && !hasModel && !remote) {
    return <GetStarted />;
  }
  if (error === "modelNotInstalled" && !hasModel) {
    return (
      <Hint icon={<DownloadIcon />} title={t("session.noModelTitle")}>
        {t("session.noModelBody")}
      </Hint>
    );
  }
  if (error === "llamaServerNotFound") {
    return (
      <Hint icon={<ServerOffIcon />} title={t("session.serverMissingTitle")}>
        <p>{t("session.serverMissingBody")}</p>
        <code className="mt-1 rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
          brew install llama.cpp
        </code>
      </Hint>
    );
  }
  return (
    <>
      <CaptionsIcon
        className="size-10 text-muted-foreground"
        strokeWidth={1.5}
      />
      <h2 className="text-lg font-medium">{t("transcript.emptyTitle")}</h2>
      <p className="max-w-md text-sm text-muted-foreground">
        {t("transcript.emptyBody")}
      </p>
    </>
  );
}

/** Shown when system audio stays completely silent: nothing is playing, or
 * macOS is withholding it because the permission was denied. */
export function SystemSilentHint() {
  const { t } = useTranslation();
  const silent = useSession((s) => s.systemSilent);
  if (!silent) return null;
  return (
    <div className="flex justify-center p-6 pb-0">
      <Hint icon={<VolumeXIcon />} title={t("session.systemSilentTitle")}>
        <p>{t("session.systemSilentBody")}</p>
        <SettingsButton
          label={t("session.openSystemAudioSettings")}
          open={commands.openSystemAudioSettings}
        />
      </Hint>
    </div>
  );
}
