import { useRef, useState } from "react";

// Hover info lives on the Advanced page. Everywhere else a row that needs
// explaining says so in a caption under its label.
interface InfoTipProps {
  text: string;
  /** "?" reads as instructions to follow; "i" as background. */
  glyph?: "i" | "?";
}

export function InfoTip({ text, glyph = "i" }: InfoTipProps) {
  const ref = useRef<HTMLSpanElement>(null);
  const [flip, setFlip] = useState(false);

  // The tooltip is 256px wide and opens to the right; near the window edge
  // that would overflow, so it flips to the left instead.
  function measure() {
    const rect = ref.current?.getBoundingClientRect();
    if (rect) setFlip(rect.right + 280 > window.innerWidth);
  }

  return (
    <span
      ref={ref}
      className="group relative inline-flex"
      onPointerEnter={measure}
      onFocus={measure}
    >
      <span
        tabIndex={0}
        aria-label={text}
        className="inline-flex h-4 w-4 items-center justify-center rounded-full border border-muted/60 text-[10px] font-semibold leading-none text-muted transition-colors group-hover:border-text group-hover:text-text"
      >
        {glyph}
      </span>
      <span
        role="tooltip"
        className={`pointer-events-none absolute top-1/2 z-10 w-64 -translate-y-1/2 glass-float rounded-control border border-hairline px-3 py-2 text-sm leading-snug text-text opacity-0 shadow-[0_8px_24px_rgba(0,0,0,0.5)] transition-opacity duration-200 group-focus-within:opacity-100 group-hover:opacity-100 ${
          flip ? "right-6" : "left-6"
        }`}
      >
        {text}
      </span>
    </span>
  );
}
