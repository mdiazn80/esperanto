import type { Language, SessionEvent } from "@/bindings";

type SegmentState =
  /** Being spoken; partial text may be shown. */
  | "speaking"
  /** Speech ended; waiting for the final transcript. */
  | "processing"
  | "final"
  /** Nothing intelligible, or an error: hidden from the transcript. */
  | "empty";

export type Segment = {
  id: number;
  state: SegmentState;
  /** Partial text: agreed by two passes / may still change. */
  stable: string;
  unstable: string;
  text: string;
  language: Language | null;
  translation: string | null;
  translationLanguage: Language | null;
  latencyMs: number | null;
  /** Who spoke it (speaker labels), numbered from 0; `null` if unknown. */
  speaker: number | null;
  /** When speech started (Unix ms), for the time column. */
  heardAt: number;
};

/** Whether a segment is shown (empty ones are hidden). */
export function isVisible(segment: Segment): boolean {
  return segment.state !== "empty";
}

/** Oldest segments are dropped beyond this, to keep the UI fast. */
const MAX_SEGMENTS = 500;

function blank(id: number, heardAt: number): Segment {
  return {
    id,
    state: "speaking",
    stable: "",
    unstable: "",
    text: "",
    language: null,
    translation: null,
    translationLanguage: null,
    latencyMs: null,
    speaker: null,
    heardAt,
  };
}

function update(
  segments: Segment[],
  id: number,
  change: (segment: Segment) => Segment,
): Segment[] {
  const index = segments.findIndex((s) => s.id === id);
  if (index === -1) return segments;
  const next = segments.slice();
  next[index] = change(segments[index]);
  return next;
}

/** Applies a session event to the list of segments (pure but for the
 * default `now`, the time a new segment is heard). */
export function applyEvent(
  segments: Segment[],
  event: SessionEvent,
  now: number = Date.now(),
): Segment[] {
  switch (event.type) {
    case "speechStarted": {
      // The previous segment, if still marked as speaking, has ended.
      const ended = segments.map((s) =>
        s.state === "speaking" ? { ...s, state: "processing" as const } : s,
      );
      return [...ended, blank(event.segmentId, now)].slice(-MAX_SEGMENTS);
    }
    case "speechDiscarded":
      return segments.filter((s) => s.id !== event.segmentId);
    case "partial":
      return update(segments, event.segmentId, (s) =>
        s.state === "speaking" || s.state === "processing"
          ? { ...s, stable: event.stable, unstable: event.unstable }
          : s,
      );
    case "final":
      return update(segments, event.segmentId, (s) => ({
        ...s,
        state: "final",
        text: event.text,
        language: event.language,
        latencyMs: event.latencyMs,
        stable: "",
        unstable: "",
      }));
    case "empty":
    case "inferenceError":
      return update(segments, event.segmentId, (s) =>
        s.state === "final" ? s : { ...s, state: "empty" },
      );
    case "speaker":
      return update(segments, event.segmentId, (s) => ({
        ...s,
        speaker: event.speaker,
      }));
    case "translation":
      return update(segments, event.segmentId, (s) => ({
        ...s,
        translation: event.text,
        translationLanguage: event.language,
      }));
    default:
      return segments;
  }
}

/** Marks the segment being spoken as ended (when the session stops). */
export function endSpeaking(segments: Segment[]): Segment[] {
  return segments.map((s) =>
    s.state === "speaking" ? { ...s, state: "processing" } : s,
  );
}
