import { useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AudiblMark,
  BookIcon,
  ClockIcon,
  GearIcon,
  SlidersIcon,
} from "./ui/icons";

export type Page = "general" | "dictionary" | "history" | "advanced";

const PAGES: { page: Page; Icon: typeof SlidersIcon }[] = [
  { page: "general", Icon: SlidersIcon },
  { page: "dictionary", Icon: BookIcon },
  { page: "history", Icon: ClockIcon },
  { page: "advanced", Icon: GearIcon },
];

interface SidebarProps {
  active: Page;
  onSelect: (page: Page) => void;
}

export function Sidebar({ active, onSelect }: SidebarProps) {
  const { t, i18n } = useTranslation();
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  /** The active tab's box, measured rather than assumed: the Vietnamese
   *  labels set their own metrics, and a hardcoded row height would drift. */
  const [pill, setPill] = useState<{ top: number; height: number } | null>(null);

  useLayoutEffect(() => {
    const el = tabs.current[PAGES.findIndex((p) => p.page === active)];
    if (el) setPill({ top: el.offsetTop, height: el.offsetHeight });
  }, [active, i18n.language]);

  return (
    // The window has no titlebar, so the rail runs to the top edge and the
    // pt-11 inset clears the traffic lights floating over it.
    <nav
      data-tauri-drag-region
      className="glass-rail flex w-[180px] shrink-0 flex-col border-r border-hairline px-3 pb-4 pt-11"
    >
      {/* The block is 60px tall — the PageTitle's 36px line plus its mb-6 —
          so the first tab's top lines up with the top of the first card on
          every page. The lockup is centered in the rail; the name sits one
          notch under the page title beside it (text-wordmark, 26px). The
          mark's bars fill its whole box, so its size is that type's cap
          height — Newsreader's "A" is 1372/2000 em, i.e. 18px — and the
          waveform ends up exactly as tall as the A beside it.

          items-baseline on the inner row is what sits the mark on the type's
          baseline: an SVG has no baseline of its own, so the flexbox
          synthesizes one from the bottom of its box, which is exactly the
          alignment wanted. The outer row keeps that group centered in the
          60px block, lifted 4px off centre — the block's height is what keeps
          the first tab in line with the first card, so the nudge is a
          transform and costs no layout. */}
      <div
        data-tauri-drag-region
        className="flex h-[60px] items-center justify-center text-text"
      >
        <div className="flex -translate-y-1 items-baseline gap-2.5">
          <AudiblMark size={18} className="pointer-events-none shrink-0" />
          <span className="pointer-events-none text-wordmark leading-none tracking-tight">
            Audibl
          </span>
        </div>
      </div>

      <div className="relative flex flex-col gap-0.5">
        {/* One pill for all four tabs, slid into place. Hidden until the first
            measurement lands, so it never animates in from the top. */}
        {pill && (
          <span
            aria-hidden
            className="glass-control pointer-events-none absolute inset-x-0 top-0 rounded-control ring-1 ring-inset ring-hairline transition-[transform,height] duration-[260ms] ease-[cubic-bezier(0.22,1,0.36,1)]"
            style={{
              transform: `translateY(${pill.top}px)`,
              height: `${pill.height}px`,
            }}
          />
        )}
        {PAGES.map(({ page, Icon }, i) => {
          const selected = active === page;
          return (
            <button
              key={page}
              ref={(el) => {
                tabs.current[i] = el;
              }}
              type="button"
              onClick={() => onSelect(page)}
              aria-current={selected ? "page" : undefined}
              className={`group relative flex items-center gap-2.5 rounded-control px-2.5 py-2 text-left text-base transition-colors duration-200 ${
                selected
                  ? "font-medium text-text"
                  : "text-muted hover:bg-control/30 hover:text-text"
              }`}
            >
              <Icon
                size={16}
                className={`transition-colors duration-200 ${
                  selected ? "text-text" : "text-muted group-hover:text-text"
                }`}
              />
              {t(`sidebar.${page}`)}
            </button>
          );
        })}
      </div>

      <div className="mt-auto px-2.5 text-xs text-muted/70">
        {t("sidebar.slogan")}
      </div>
    </nav>
  );
}
