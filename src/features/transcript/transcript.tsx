import { ArrowDownIcon, BookmarkIcon } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Language } from "@/bindings";
import { Button } from "@/components/ui/button";
import { formatClock } from "@/lib/format";
import { speakerColor } from "@/lib/speakers";
import { cn } from "@/lib/utils";
import { isVisible, type Segment } from "@/lib/transcript";
import { useSession } from "@/store/session";
import { useSettings } from "@/store/settings";

/** Distance from the bottom (px) still counted as "following live". */
const FOLLOW_THRESHOLD = 48;

/** Blinking block at the end of the text being written. */
function Caret() {
  return (
    <span
      aria-hidden="true"
      className="ml-0.5 inline-block h-[1.1em] w-[0.5em] animate-pulse bg-foreground align-text-bottom"
    />
  );
}

/** The columns in use, as a CSS grid template. */
function gridOf(showSpeaker: boolean, showTranslation: boolean): string {
  return [
    "4.5rem",
    showSpeaker && "8rem",
    "minmax(0,1fr)",
    showTranslation && "minmax(0,1fr)",
  ]
    .filter(Boolean)
    .join(" ");
}

function SpeakerName({ speaker }: { speaker: number | null }) {
  const { t } = useTranslation();
  const custom = useSession((s) =>
    speaker === null ? undefined : s.speakerNames[speaker],
  );
  if (speaker === null) return <span />;
  return (
    <span className={cn("truncate", speakerColor(speaker).text)}>
      {custom || t("speakers.default", { number: speaker + 1 })}
    </span>
  );
}

function Row({
  segment,
  grid,
  showSpeaker,
  showTranslation,
  marked,
}: {
  segment: Segment;
  grid: string;
  showSpeaker: boolean;
  showTranslation: boolean;
  marked: boolean;
}) {
  const { t, i18n } = useTranslation();
  const live = segment.state !== "final";
  const hasPartial = segment.stable !== "" || segment.unstable !== "";
  const bar =
    segment.speaker !== null
      ? speakerColor(segment.speaker).border
      : "border-brand";

  return (
    <div
      id={`segment-${segment.id}`}
      role="row"
      style={{ gridTemplateColumns: grid }}
      className={cn(
        "grid gap-x-6 border-b border-l-2 border-b-border/70 border-l-transparent px-5 py-4 transition-colors hover:bg-muted/40",
        live && bar,
        marked && "bg-brand/5",
      )}
    >
      <span
        role="cell"
        className="flex items-start gap-1.5 font-mono text-sm text-muted-foreground tabular-nums"
      >
        {formatClock(segment.heardAt, i18n.language)}
        {marked && (
          <BookmarkIcon
            aria-label={t("transcript.columns.marked")}
            className="mt-0.5 size-3.5 fill-current text-brand-text"
          />
        )}
      </span>
      {showSpeaker && (
        <span role="cell" className="flex min-w-0 text-[0.95rem]">
          <SpeakerName speaker={segment.speaker} />
        </span>
      )}
      <p
        role="cell"
        lang={segment.language ?? undefined}
        className={cn(
          "text-[0.95rem] leading-relaxed",
          showTranslation ? "text-original" : "text-foreground",
        )}
      >
        {segment.state === "final" ? (
          segment.text
        ) : hasPartial ? (
          <>
            <span>{segment.stable}</span>
            {segment.stable && segment.unstable ? " " : ""}
            <span className="text-muted-foreground italic">
              {segment.unstable}
            </span>
            {segment.state === "speaking" && <Caret />}
          </>
        ) : (
          <span className="text-muted-foreground">…</span>
        )}
      </p>
      {showTranslation && (
        <p
          role="cell"
          lang={segment.translationLanguage ?? undefined}
          className="text-[0.95rem] leading-relaxed text-foreground"
        >
          {segment.translation ??
            (live ? <span className="text-muted-foreground">…</span> : null)}
        </p>
      )}
    </div>
  );
}

/**
 * Announces each finished phrase (and its translation) to screen readers.
 * The visible list isn't live: partial text changes too often to read out.
 */
function Announcer({ segments }: { segments: readonly Segment[] }) {
  const { t } = useTranslation();
  const names = useSession((s) => s.speakerNames);
  const finals = segments.filter((s) => s.state === "final");
  const last = finals[finals.length - 1];
  const speaker =
    last?.speaker != null
      ? names[last.speaker] ||
        t("speakers.default", { number: last.speaker + 1 })
      : null;
  return (
    <div role="log" aria-live="polite" className="sr-only">
      {last && (
        <p key={last.id}>
          {speaker && `${speaker}: `}
          <span lang={last.language ?? undefined}>{last.text}</span>
          {last.translation && (
            <span lang={last.translationLanguage ?? undefined}>
              {" "}
              {last.translation}
            </span>
          )}
        </p>
      )}
    </div>
  );
}

/** A column head's language code: "EN · ", or nothing if it varies. */
function codeOf(languages: (Language | null)[]): string {
  const known = new Set(languages.filter((l) => l !== null));
  return known.size === 1 ? `${[...known][0]} · ` : "";
}

/** The transcript as a table, following the newest text unless the user
 * scrolls up. */
export function Transcript() {
  const { t } = useTranslation();
  const segments = useSession((s) => s.segments);
  const status = useSession((s) => s.status);
  const labels = useSession((s) => s.speakerLabels);
  const moments = useSession((s) => s.moments);
  const translating = useSettings(
    (s) => s.settings.targetLanguage.kind !== "none",
  );
  const visible = segments.filter(isVisible);
  const scroller = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(true);

  const showSpeaker = labels || visible.some((s) => s.speaker !== null);
  const showTranslation =
    visible.some((s) => s.translation) || (status !== "idle" && translating);
  const grid = gridOf(showSpeaker, showTranslation);
  const marked = new Set(moments.map((m) => m.segmentId));

  // Keep the newest text in view while following.
  useLayoutEffect(() => {
    const el = scroller.current;
    if (el && following) el.scrollTop = el.scrollHeight;
  }, [segments, following]);

  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const onScroll = () =>
      setFollowing(
        el.scrollHeight - el.scrollTop - el.clientHeight < FOLLOW_THRESHOLD,
      );
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <div className="relative min-h-0 flex-1">
      <Announcer segments={visible} />
      <div
        ref={scroller}
        className="h-full overflow-y-auto"
        role="table"
        aria-label={t("transcript.label")}
        tabIndex={0}
      >
        <div
          role="row"
          style={{ gridTemplateColumns: grid }}
          className="eyebrow sticky top-0 z-10 grid gap-x-6 border-b border-l-2 border-l-transparent bg-background px-5 py-3"
        >
          <span role="columnheader">{t("transcript.columns.time")}</span>
          {showSpeaker && (
            <span role="columnheader">{t("transcript.columns.who")}</span>
          )}
          <span role="columnheader">
            {codeOf(visible.map((s) => s.language))}
            {t("transcript.columns.original")}
          </span>
          {showTranslation && (
            <span role="columnheader">
              {codeOf(visible.map((s) => s.translationLanguage))}
              {t("transcript.columns.translation")}
            </span>
          )}
        </div>
        <div role="rowgroup">
          {visible.map((segment) => (
            <Row
              key={segment.id}
              segment={segment}
              grid={grid}
              showSpeaker={showSpeaker}
              showTranslation={showTranslation}
              marked={marked.has(segment.id)}
            />
          ))}
        </div>
        {status === "listening" && visible.length === 0 && (
          <p className="animate-pulse p-6 text-center font-mono text-sm text-muted-foreground">
            {t("session.listening")}
          </p>
        )}
      </div>
      {!following && (
        <Button
          size="sm"
          variant="secondary"
          className="absolute bottom-4 left-1/2 -translate-x-1/2 shadow"
          onClick={() => setFollowing(true)}
        >
          <ArrowDownIcon />
          {t("transcript.jumpToLive")}
        </Button>
      )}
    </div>
  );
}
