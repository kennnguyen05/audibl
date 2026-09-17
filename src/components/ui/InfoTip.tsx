// Hover info is allowed on the Advanced page only. Do not import elsewhere.
interface InfoTipProps {
  text: string;
}

export function InfoTip({ text }: InfoTipProps) {
  return (
    <span className="relative inline-flex group">
      <span
        tabIndex={0}
        aria-label={text}
        className="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full border border-muted text-muted text-[9px] leading-none font-semibold outline-none"
      >
        i
      </span>
      <span
        role="tooltip"
        className="pointer-events-none absolute left-5 top-1/2 -translate-y-1/2 z-10 w-64 rounded-md border border-border bg-bg px-2.5 py-1.5 text-xs shadow-lg opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity"
      >
        {text}
      </span>
    </span>
  );
}
