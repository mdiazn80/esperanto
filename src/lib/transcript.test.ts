import { describe, expect, it } from "vitest";
import type { SessionEvent } from "@/bindings";
import { applyEvent, endSpeaking, type Segment } from "@/lib/transcript";

function run(events: SessionEvent[], start: Segment[] = []): Segment[] {
  return events.reduce(applyEvent, start);
}

describe("applyEvent", () => {
  it("follows a segment from speech to translation", () => {
    const segments = run([
      { type: "speechStarted", segmentId: 1 },
      { type: "partial", segmentId: 1, stable: "", unstable: "hola" },
      { type: "partial", segmentId: 1, stable: "hola", unstable: "a todos" },
      {
        type: "final",
        segmentId: 1,
        text: "Hola a todos.",
        language: "es",
        latencyMs: 640,
        startMs: 0,
        endMs: 1000,
      },
      {
        type: "translation",
        segmentId: 1,
        language: "en",
        text: "Hello everyone.",
      },
    ]);
    expect(segments).toHaveLength(1);
    expect(segments[0]).toMatchObject({
      state: "final",
      text: "Hola a todos.",
      stable: "",
      translation: "Hello everyone.",
      latencyMs: 640,
    });
  });

  it("ends the previous segment when a new one starts", () => {
    const segments = run([
      { type: "speechStarted", segmentId: 1 },
      { type: "speechStarted", segmentId: 2 },
    ]);
    expect(segments.map((s) => s.state)).toEqual(["processing", "speaking"]);
  });

  it("ignores partials that arrive after the final text", () => {
    const segments = run([
      { type: "speechStarted", segmentId: 1 },
      {
        type: "final",
        segmentId: 1,
        text: "Done.",
        language: "en",
        latencyMs: 1,
        startMs: 0,
        endMs: 1000,
      },
      { type: "partial", segmentId: 1, stable: "late", unstable: "" },
    ]);
    expect(segments[0].stable).toBe("");
  });

  it("drops discarded speech and hides empty segments", () => {
    const segments = run([
      { type: "speechStarted", segmentId: 1 },
      { type: "speechDiscarded", segmentId: 1 },
      { type: "speechStarted", segmentId: 2 },
      { type: "empty", segmentId: 2 },
    ]);
    expect(segments.map((s) => [s.id, s.state])).toEqual([[2, "empty"]]);
  });

  it("marks speech in progress as processing when stopping", () => {
    const segments = endSpeaking(
      run([{ type: "speechStarted", segmentId: 1 }]),
    );
    expect(segments[0].state).toBe("processing");
  });

  it("labels a segment with its speaker", () => {
    const segments = run([
      { type: "speechStarted", segmentId: 1 },
      { type: "speaker", segmentId: 1, speaker: 2 },
    ]);
    expect(segments[0].speaker).toBe(2);
  });
});

describe("heardAt", () => {
  it("records when a segment started being heard", () => {
    const segments = applyEvent(
      [],
      { type: "speechStarted", segmentId: 1 },
      42,
    );
    expect(segments[0].heardAt).toBe(42);
  });
});
