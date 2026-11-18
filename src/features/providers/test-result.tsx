import {
  CheckCircle2Icon,
  Loader2Icon,
  TriangleAlertIcon,
  XCircleIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ConnectionTest } from "@/bindings";
import { formatSeconds } from "@/lib/format";

export type TestState =
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "ok"; result: ConnectionTest }
  | { kind: "failed"; message: string };

function Body({ state, model }: { state: TestState; model: string }) {
  const { t, i18n } = useTranslation();
  switch (state.kind) {
    case "idle":
      return null;
    case "running":
      return (
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2Icon className="size-4 animate-spin" />
          {t("providers.testing")}
        </p>
      );
    case "failed":
      return (
        <p className="flex items-start gap-2 text-sm text-destructive">
          <XCircleIcon className="mt-0.5 size-4 shrink-0" />
          {state.message}
        </p>
      );
    case "ok": {
      const { result } = state;
      return (
        <div className="flex flex-col gap-1 text-sm">
          <p className="flex items-center gap-2 text-emerald-600 dark:text-emerald-400">
            <CheckCircle2Icon className="size-4" />
            {t("providers.testOk", {
              seconds: formatSeconds(result.latencyMs ?? 0, i18n.language),
            })}
          </p>
          {result.modelListed === false && (
            <p className="flex items-start gap-2 text-amber-600 dark:text-amber-400">
              <TriangleAlertIcon className="mt-0.5 size-4 shrink-0" />
              {t("providers.modelNotListed", {
                model,
                models: result.models?.join(", ") ?? "",
              })}
            </p>
          )}
        </div>
      );
    }
  }
}

/** The outcome of "Test connection", announced to screen readers. */
export function TestResult({
  state,
  model,
}: {
  state: TestState;
  model: string;
}) {
  return (
    <div role="status">
      <Body state={state} model={model} />
    </div>
  );
}
