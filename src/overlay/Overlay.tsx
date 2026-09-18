import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";

type OverlayState = "recording" | "transcribing" | "cleaning";

interface ShowPayload {
  state: OverlayState;
  /// Debug-only screenshot hook (`AUDIBLE_DEV_OVERLAY`): there is no
  /// microphone behind it, so draw a synthetic waveform. Always false in
  /// release builds.
  preview?: boolean;
}

/// The visualizer is one object across all three states: the same fourteen
/// bars, in the same places, only ever changing height and colour. Nothing is
/// swapped in or out, so the user watches a single thing settle from live
/// speech into a resting row while the work finishes.
///
/// It is a waveform, not a spectrum: each bar is one 1/30 s slice of how loud
/// the room was, the newest on the right, so the row scrolls leftwards as
/// speech goes by.
const BARS = 14;
const BAR_WIDTH = 2; // px
const BAR_GAP = 3; // px
const TRACK_HEIGHT = 18; // px — the box the bars grow inside, from the centre
const MIN_BAR = 2; // px — silence
const RESTING_BAR = 4; // px — the even row the bars settle to once we are busy
const TRACK_WIDTH = BARS * BAR_WIDTH + (BARS - 1) * BAR_GAP; // 67px

/// Cleaning is a slower, more deliberate pass than transcribing, and its
/// sweep says so.
const SWEEP_MS: Record<Exclude<OverlayState, "recording">, number> = {
  transcribing: 1500,
  cleaning: 2400,
};

/// Shifts the history one slot left and appends the newest sample. The new
/// sample is eased against the one before it so a single loud frame cannot
/// spike the row; everything older is frozen history and never moves again.
function pushLevel(history: number[], level: number): number[] {
  const previous = history[history.length - 1] ?? 0;
  return [...history.slice(1), previous * 0.35 + level * 0.65];
}

export function Overlay() {
  const { t } = useTranslation();
  const [payload, setPayload] = useState<ShowPayload | null>(null);
  const [visible, setVisible] = useState(false);
  const [levels, setLevels] = useState<number[]>(() => Array(BARS).fill(0));
  // mic-level keeps arriving for a beat after the mic closes; only the
  // recording state is allowed to drive the bars.
  const listening = useRef(false);

  useEffect(() => {
    const unlisteners = [
      listen<ShowPayload>("show-overlay", (e) => {
        setPayload(e.payload);
        setVisible(true);
        listening.current = e.payload.state === "recording";
        if (listening.current) setLevels(Array(BARS).fill(0));
      }),
      listen("hide-overlay", () => {
        setVisible(false);
        listening.current = false;
      }),
      listen<number>("mic-level", (e) => {
        if (!listening.current) return;
        setLevels((history) => pushLevel(history, e.payload));
      }),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
  }, []);

  const state = payload?.state ?? "recording";
  // Narrowed once, so both the label and the sweep speed can key off it.
  const busy = state === "recording" ? null : state;
  const recording = busy === null;

  // Screenshot hook only: stand in for the microphone so the recording state
  // can be reviewed without speaking. Two detuned sines read as speech rather
  // than as a loop, fed through the same history as the real thing.
  const previewing = (payload?.preview ?? false) && recording && visible;
  useEffect(() => {
    if (!previewing) return;
    const start = performance.now();
    const timer = window.setInterval(() => {
      const s = (performance.now() - start) / 1000;
      const level = Math.min(
        1,
        Math.max(0.04, 0.44 + 0.3 * Math.sin(s * 9.4) + 0.22 * Math.sin(s * 5.1)),
      );
      setLevels((history) => pushLevel(history, level));
    }, 33);
    return () => window.clearInterval(timer);
  }, [previewing]);

  const heights = recording
    ? levels.map((level) =>
        Math.round(
          Math.min(
            TRACK_HEIGHT,
            Math.max(MIN_BAR, MIN_BAR + level * (TRACK_HEIGHT - MIN_BAR)),
          ),
        ),
      )
    : Array<number>(BARS).fill(RESTING_BAR);

  // No instructions while recording: the dot and the moving bars already say
  // "listening". Words only appear once there is work to wait for.
  const label =
    busy === "cleaning"
      ? t("overlay.cleaning")
      : busy === "transcribing"
        ? t("overlay.transcribing")
        : null;

  // One row of bars, drawn identically for the base layer and for the amber
  // highlight the sweep reveals — identical geometry is what makes the
  // highlight look like the bars lighting up rather than a shape on top.
  const barRow = (fill: string) => (
    <div
      className="flex items-center justify-between"
      style={{ height: TRACK_HEIGHT, width: TRACK_WIDTH }}
    >
      {heights.map((height, i) => (
        <span
          key={i}
          className={`rounded-full ${fill}`}
          style={{
            width: BAR_WIDTH,
            height,
            // The morph between states is this transition. While recording it
            // is short enough to track the voice; on the way into a busy state
            // the longer duration (plus a left-to-right stagger) is what turns
            // the swap into a settle.
            transitionProperty: "height, background-color",
            transitionTimingFunction: "cubic-bezier(0.22, 1, 0.36, 1)",
            transitionDuration: recording ? "90ms" : "300ms",
            transitionDelay: recording ? "0ms" : `${i * 12}ms`,
          }}
        />
      ))}
    </div>
  );

  return (
    <div className="h-full w-full flex items-center justify-center">
      <div
        role="status"
        aria-live="polite"
        className={[
          "transition-[opacity,transform] duration-200 ease-out",
          visible && payload
            ? "opacity-100 translate-y-0 scale-100"
            : "opacity-0 translate-y-1 scale-[0.97]",
        ].join(" ")}
      >
        {/* The pill hugs its label: a fixed width left dead space next to the
            short Hold-mode text. */}
        <div
          className={[
            "flex items-center gap-2.5 h-10 px-3.5 rounded-full max-w-[270px]",
            // The one place in the app a shadow is correct: this really does
            // float over someone else's window. The hairline keeps the pill's
            // edge visible when that window is dark too.
            "bg-surface ring-1 ring-text/10",
            "shadow-[0_10px_28px_-6px_rgba(0,0,0,0.65),0_2px_8px_-2px_rgba(0,0,0,0.45)]",
          ].join(" ")}
        >
          {/* The recording light: lit while the microphone is open, dark for
              the rest of the session. */}
          <span
            aria-hidden
            className={`h-2 w-2 shrink-0 rounded-full transition-colors duration-200 ${
              recording ? "record-dot bg-record" : "bg-control"
            }`}
          />
          <div
            className="relative shrink-0"
            style={{ height: TRACK_HEIGHT, width: TRACK_WIDTH }}
            aria-hidden
          >
            {barRow(recording ? "bg-accent" : "bg-muted/55")}
            {busy !== null && (
              <div
                className="overlay-sweep absolute inset-0"
                style={{ animationDuration: `${SWEEP_MS[busy]}ms` }}
              >
                {barRow("bg-accent")}
              </div>
            )}
          </div>
          {label && (
            <span className="min-w-0 truncate text-xs text-muted">{label}</span>
          )}
        </div>
      </div>
    </div>
  );
}
