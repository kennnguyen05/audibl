import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

// Hover info lives on the Advanced page. Everywhere else a row that needs
// explaining says so in a caption under its label.
interface InfoTipProps {
  text: string;
  /** "?" reads as instructions to follow; "i" as background. */
  glyph?: "i" | "?";
}

/** Must match the w-64 below: the flip test needs the width before layout. */
const TIP_WIDTH = 256;
/** Space between the glyph and the tooltip, on whichever side it opens. */
const GAP = 8;

interface Placement {
  top: number;
  left: number;
}

export function InfoTip({ text, glyph = "i" }: InfoTipProps) {
  const ref = useRef<HTMLSpanElement>(null);
  const [placement, setPlacement] = useState<Placement | null>(null);

  // The tooltip is rendered into <body> rather than next to the glyph. Every
  // card is a .glass-card, and a backdrop-filter element both clips its
  // overflow (Group adds overflow-hidden for the rounded corners) and becomes
  // the containing block for fixed descendants — so a tooltip parented inside
  // a row gets sliced flat at the card's edge whenever it is taller than its
  // row. Outside the card there is nothing to clip it, and .glass-float's blur
  // is no longer nested in .glass-card's, which bought nothing anyway.
  function open() {
    const rect = ref.current?.getBoundingClientRect();
    if (!rect) return;
    // Opens to the right; near the window edge that would overflow, so it
    // flips to the left instead.
    const flip = rect.right + GAP + TIP_WIDTH > window.innerWidth;
    setPlacement({
      top: rect.top + rect.height / 2,
      left: flip ? rect.left - GAP - TIP_WIDTH : rect.right + GAP,
    });
  }

  // Fixed coordinates are frozen at open time, so anything that moves the
  // glyph afterwards has to dismiss the tooltip. Scroll is captured because
  // the page scrolls in <main>, not on the window.
  useEffect(() => {
    if (!placement) return;
    const close = () => setPlacement(null);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  }, [placement]);

  return (
    <>
      <span
        ref={ref}
        tabIndex={0}
        aria-label={text}
        onPointerEnter={open}
        onPointerLeave={() => setPlacement(null)}
        onFocus={open}
        onBlur={() => setPlacement(null)}
        className="inline-flex h-4 w-4 items-center justify-center rounded-full border border-muted/60 text-[10px] font-semibold leading-none text-muted transition-colors hover:border-text hover:text-text focus-visible:border-text focus-visible:text-text"
      >
        {glyph}
      </span>
      {placement &&
        createPortal(
          <span
            role="tooltip"
            style={{ top: placement.top, left: placement.left }}
            className="animate-fade-in glass-float pointer-events-none fixed z-40 w-64 -translate-y-1/2 rounded-control border border-hairline px-3 py-2 text-sm leading-snug text-text shadow-[0_8px_24px_rgba(0,0,0,0.5)]"
          >
            {text}
          </span>,
          document.body,
        )}
    </>
  );
}
