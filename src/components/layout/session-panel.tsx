import { getVersion } from "@tauri-apps/api/app";
import { TriangleAlertIcon } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { formatClock, formatSeconds, hostOf } from "@/lib/format";
import { meterFraction } from "@/lib/level";
import { cn } from "@/lib/utils";
import {
  LanguageSelects,
  SourceToggle,
} from "@/components/layout/session-controls";
import { activeModel, useModels } from "@/store/models";
import { activeRemote, useProviders } from "@/store/providers";
import { useSession } from "@/store/session";
import { useSettings } from "@/store/settings";

/** Below this fraction a bar is drawn as silence. */
const QUIET = 0.04;

/** The input level over the last second or so, newest on the right. */
function Waveform() {
  const { t } = useTranslation();
  const levels = useSession((s) => s.levels);
  const bars = levels.map(meterFraction);
  const fraction = bars.at(-1) ?? 0;

  return (
    <div
      role="meter"
      aria-label={t("session.inputLevel")}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(fraction * 100)}
      className="flex h-14 items-center gap-[3px]"
    >
      {bars.map((value, index) => (
        <span
          key={index}
          aria-hidden="true"
          className={cn(
            "w-1 rounded-full transition-[height] duration-75",
            value > QUIET ? "bg-brand" : "bg-muted-foreground/25",
          )}
          style={{ height: `${Math.max(6, value * 100)}%` }}
        />
      ))}
    </div>
  );
}

function Stat({ value, label }: { value: string; label: string }) {
  return (
    <div className="flex flex-col gap-0.5 rounded-lg bg-card px-3 py-2.5">
      <span className="font-mono text-2xl tabular-nums">{value}</span>
      <span className="text-xs text-muted-foreground">{label}</span>
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2.5">
      <h2 className="eyebrow">{title}</h2>
      {children}
    </section>
  );
}

/** What the input is: the running session's device, or the next source. */
function InputName() {
  const { t } = useTranslation();
  const active = useSession((s) => s.active);
  const source = useSettings((s) => s.settings.audioSource);
  if (active) {
    return active.source === "system"
      ? t("status.systemAudio", { device: active.deviceName })
      : t("panel.microphone", { device: active.deviceName });
  }
  return source === "system"
    ? t("toolbar.sourceSystemShort")
    : t("toolbar.sourceMicrophone");
}

/** The engine: the running session's, or the one the next session uses. */
function Engine() {
  const { t } = useTranslation();
  const active = useSession((s) => s.active);
  const model = useModels((s) => activeModel(s.overview));
  const remote = useProviders((s) => activeRemote(s.view));
  const engine = active
    ? active
    : remote
      ? { remoteHost: hostOf(remote.baseUrl), modelName: remote.model }
      : { remoteHost: null, modelName: model?.name ?? null };

  if (engine.remoteHost) {
    return (
      <div className="flex flex-col gap-1">
        <p className="flex items-center gap-2 text-sm">
          <span aria-hidden="true" className="size-2 rounded-full bg-sky-400" />
          {t("status.remote", { host: engine.remoteHost })}
        </p>
        {engine.modelName && (
          <p className="text-sm text-muted-foreground">{engine.modelName}</p>
        )}
        <p className="text-sm text-muted-foreground">
          {t("status.remoteHint", { host: engine.remoteHost })}
        </p>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-1">
      <p className="flex items-center gap-2 text-sm">
        <span aria-hidden="true" className="size-2 rounded-full bg-live" />
        {engine.modelName
          ? t("panel.local", { model: engine.modelName })
          : t("status.noModel")}
      </p>
      <p className="text-sm text-muted-foreground">{t("panel.localPrivacy")}</p>
    </div>
  );
}

/** Marked phrases; clicking one scrolls the transcript to it. */
function Moments() {
  const { t, i18n } = useTranslation();
  const moments = useSession((s) => s.moments);
  const segments = useSession((s) => s.segments);

  const items = moments.flatMap((moment) => {
    const segment = segments.find((s) => s.id === moment.segmentId);
    if (!segment) return [];
    const text =
      segment.translation ||
      segment.text ||
      [segment.stable, segment.unstable].filter(Boolean).join(" ") ||
      "…";
    return [
      {
        id: segment.id,
        time: formatClock(segment.heardAt, i18n.language),
        text,
      },
    ];
  });

  if (items.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">{t("panel.momentsEmpty")}</p>
    );
  }
  return (
    <ul className="flex flex-col gap-1.5">
      {items.map((item) => (
        <li key={item.id}>
          <button
            type="button"
            className="line-clamp-2 text-left text-sm text-brand-text underline underline-offset-3 hover:no-underline focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
            onClick={() =>
              document
                .getElementById(`segment-${item.id}`)
                ?.scrollIntoView({ behavior: "smooth", block: "center" })
            }
          >
            {t("panel.moment", { time: item.time, text: item.text })}
          </button>
        </li>
      ))}
    </ul>
  );
}

/** Right column: input, figures, engine and marked moments. */
export function SessionPanel() {
  const { t, i18n } = useTranslation();
  const latencyMs = useSession((s) => s.latencyMs);
  const behind = useSession((s) => s.behind);
  const phrases = useSession(
    (s) => s.segments.filter((segment) => segment.state === "final").length,
  );
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    getVersion().then(setVersion, console.error);
  }, []);

  return (
    <aside className="flex w-76 shrink-0 flex-col border-l bg-sidebar">
      <div className="flex min-h-0 flex-1 flex-col gap-7 overflow-y-auto px-4 pt-5 pb-4">
        <Section title={t("toolbar.source")}>
          <SourceToggle />
        </Section>

        <Section title={t("toolbar.languagePair")}>
          <LanguageSelects />
        </Section>

        <Section title={t("panel.input")}>
          <Waveform />
          <p className="truncate text-sm">
            <InputName />
          </p>
          <div className="mt-1 grid grid-cols-2 gap-2">
            <Stat
              value={
                latencyMs === null
                  ? "—"
                  : t("status.latency", {
                      seconds: formatSeconds(latencyMs, i18n.language),
                    })
              }
              label={t("panel.latency")}
            />
            <Stat
              value={new Intl.NumberFormat(i18n.language).format(phrases)}
              label={t("panel.phrases")}
            />
          </div>
          {behind && (
            <p className="flex items-center gap-1.5 text-sm text-amber-600 dark:text-amber-400">
              <TriangleAlertIcon className="size-4" />
              {t("status.behind")}
            </p>
          )}
        </Section>

        <Section title={t("panel.engine")}>
          <Engine />
        </Section>

        <Section title={t("panel.moments")}>
          <Moments />
        </Section>
      </div>
      {version && (
        <p className="px-4 pb-3 font-mono text-xs text-muted-foreground">
          {t("panel.version", { version })}
        </p>
      )}
    </aside>
  );
}
