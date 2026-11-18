import { KeyRoundIcon, Loader2Icon, TriangleAlertIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ProfileView } from "@/bindings";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { TestResult, type TestState } from "@/features/providers/test-result";
import { errorKey } from "@/lib/errors";
import { useProviders } from "@/store/providers";

/** Creates (`profile` null) or edits a remote server profile. */
export function ProfileForm({
  profile,
  onSaved,
  onDeleted,
}: {
  profile: ProfileView | null;
  onSaved: (id: string) => void;
  onDeleted: () => void;
}) {
  const { t } = useTranslation();
  const save = useProviders((s) => s.save);
  const remove = useProviders((s) => s.remove);
  const clearToken = useProviders((s) => s.clearToken);
  const test = useProviders((s) => s.test);
  const [name, setName] = useState(profile?.name ?? "");
  const [baseUrl, setBaseUrl] = useState(profile?.baseUrl ?? "");
  const [model, setModel] = useState(profile?.model ?? "");
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);
  const [testState, setTestState] = useState<TestState>({ kind: "idle" });

  const changed =
    profile === null ||
    name !== profile.name ||
    baseUrl !== profile.baseUrl ||
    model !== profile.model ||
    token !== "";
  const complete = name.trim() && baseUrl.trim() && model.trim();

  async function onSave() {
    setBusy(true);
    const id = await save(
      { id: profile?.id ?? null, name, baseUrl, model },
      token,
    );
    setBusy(false);
    if (id) {
      setToken("");
      setTestState({ kind: "idle" });
      onSaved(id);
    }
  }

  async function onTest() {
    if (!profile) return;
    setTestState({ kind: "running" });
    try {
      setTestState({ kind: "ok", result: await test(profile.id) });
    } catch (error) {
      setTestState({ kind: "failed", message: t(errorKey(error)) });
    }
  }

  async function onDelete() {
    if (!profile) return;
    await remove(profile.id);
    onDeleted();
  }

  return (
    <div className="flex flex-col gap-4 rounded-lg border p-4">
      <div className="flex flex-col gap-2">
        <Label htmlFor="provider-name">{t("providers.name")}</Label>
        <Input
          id="provider-name"
          value={name}
          placeholder={t("providers.namePlaceholder")}
          onChange={(e) => setName(e.target.value)}
        />
      </div>
      <div className="flex flex-col gap-2">
        <Label htmlFor="provider-url">{t("providers.baseUrl")}</Label>
        <Input
          id="provider-url"
          value={baseUrl}
          inputMode="url"
          spellCheck={false}
          placeholder="https://gateway.example.com/v1"
          aria-describedby="provider-url-hint"
          onChange={(e) => setBaseUrl(e.target.value)}
        />
        <p id="provider-url-hint" className="text-xs text-muted-foreground">
          {t("providers.baseUrlHint")}
        </p>
      </div>
      <div className="flex flex-col gap-2">
        <Label htmlFor="provider-model">{t("providers.model")}</Label>
        <Input
          id="provider-model"
          value={model}
          spellCheck={false}
          placeholder="voxtral-mini-3b"
          onChange={(e) => setModel(e.target.value)}
        />
      </div>
      <div className="flex flex-col gap-2">
        <Label htmlFor="provider-token">{t("providers.token")}</Label>
        <Input
          id="provider-token"
          type="password"
          autoComplete="off"
          value={token}
          placeholder={
            profile?.hasToken
              ? t("providers.tokenSaved")
              : t("providers.tokenPlaceholder")
          }
          aria-describedby="provider-token-hint"
          onChange={(e) => setToken(e.target.value)}
        />
        <p
          id="provider-token-hint"
          className="flex items-center gap-1 text-xs text-muted-foreground"
        >
          <KeyRoundIcon className="size-3" />
          {t("providers.tokenHint")}
          {profile?.hasToken && (
            <Button
              variant="link"
              className="h-auto p-0 text-xs"
              onClick={() => void clearToken(profile.id)}
            >
              {t("providers.clearToken")}
            </Button>
          )}
        </p>
      </div>

      {profile?.plainHttp && (
        <p className="flex items-start gap-2 text-sm text-amber-600 dark:text-amber-400">
          <TriangleAlertIcon className="mt-0.5 size-4 shrink-0" />
          {t("providers.plainHttp")}
        </p>
      )}

      <TestResult state={testState} model={profile?.model ?? model} />

      <div className="flex flex-wrap items-center gap-2">
        <Button disabled={!complete || !changed || busy} onClick={onSave}>
          {busy && <Loader2Icon className="animate-spin" />}
          {t("providers.save")}
        </Button>
        {profile && (
          <Button
            variant="outline"
            disabled={
              changed || !profile.hasToken || testState.kind === "running"
            }
            onClick={onTest}
          >
            {t("providers.test")}
          </Button>
        )}
        {profile && (
          <ConfirmDeleteButton
            className="ml-auto"
            label={t("common.delete")}
            title={t("providers.deleteTitle", { name: profile.name })}
            description={t("providers.deleteBody")}
            onConfirm={() => void onDelete()}
          />
        )}
      </div>
    </div>
  );
}
