import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";

type OverlayState = "recording" | "transcribing" | "cleaning";

interface ShowPayload {
  state: OverlayState;
  toggle: boolean;
}

const BARS = 9;

export function Overlay() {
  const { t } = useTranslation();
  const [payload, setPayload] = useState<ShowPayload | null>(null);
  const [visible, setVisible] = useState(false);
  const [levels, setLevels] = useState<number[]>(Array(BARS).fill(0));
  const smoothed = useRef<number[]>(Array(BARS).fill(0));

  useEffect(() => {
    const unlisteners = [
      listen<ShowPayload>("show-overlay", (e) => {
        setPayload(e.payload);
        setVisible(true);
        if (e.payload.state === "recording") {
          smoothed.current = Array(BARS).fill(0);
          setLevels(smoothed.current);
        }
      }),
      listen("hide-overlay", () => setVisible(false)),
      listen<number[]>("mic-level", (e) => {
        // 16 spectrum buckets → BARS bars, eased so the waveform doesn't jitter.
        const buckets = e.payload;
        const step = buckets.length / BARS;
        smoothed.current = smoothed.current.map((prev, i) => {
          const value = buckets[Math.floor(i * step)] ?? 0;
          return prev * 0.6 + value * 0.4;
        });
        setLevels(smoothed.current);
      }),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
  }, []);

  return (
    <div className="h-full w-full flex items-center justify-center">
      <div
        className={`flex items-center gap-2.5 h-9 px-4 rounded-full bg-neutral-900/90 text-white text-xs shadow-lg ring-1 ring-white/10 transition-opacity duration-200 ${
          visible && payload ? "opacity-100" : "opacity-0"
        }`}
      >
        {payload?.state === "recording" ? (
          <>
            <span className="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
            <div className="flex items-center gap-[3px] h-5">
              {levels.map((level, i) => (
                <span
                  key={i}
                  className="w-[3px] rounded-full bg-white/90"
                  style={{ height: `${Math.max(3, Math.min(20, 3 + level * 17))}px` }}
                />
              ))}
            </div>
            {payload.toggle && (
              <span className="text-white/60 whitespace-nowrap">
                {t("overlay.toggleHint")}
              </span>
            )}
          </>
        ) : (
          <>
            <span className="w-3 h-3 rounded-full border-2 border-white/30 border-t-white animate-spin" />
            <span className="whitespace-nowrap">
              {payload?.state === "cleaning"
                ? t("overlay.cleaning")
                : t("overlay.transcribing")}
            </span>
          </>
        )}
      </div>
    </div>
  );
}
