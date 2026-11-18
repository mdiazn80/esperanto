import { Loader2Icon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Guidance, SystemSilentHint } from "@/features/transcript/guidance";
import { Transcript } from "@/features/transcript/transcript";
import { isVisible } from "@/lib/transcript";
import { useSession } from "@/store/session";

/** The main area: guidance when idle, a spinner while starting, or the
 * transcript. */
export function TranscriptView() {
  const { t } = useTranslation();
  const status = useSession((s) => s.status);
  const hasSegments = useSession((s) => s.segments.some(isVisible));

  if (status === "starting") {
    return (
      <div
        role="status"
        className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 p-8 text-center text-muted-foreground"
      >
        <Loader2Icon className="size-8 animate-spin" />
        <p className="text-sm">{t("session.preparing")}</p>
      </div>
    );
  }

  if (status !== "idle" || hasSegments) {
    return (
      <div className="flex min-h-0 flex-1 flex-col">
        {status === "listening" && !hasSegments && <SystemSilentHint />}
        <Transcript />
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
      <Guidance />
    </div>
  );
}
