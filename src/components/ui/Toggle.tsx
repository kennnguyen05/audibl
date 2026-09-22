interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  label: string;
}

export function Toggle({ checked, onChange, disabled, label }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-10 shrink-0 rounded-full transition-colors duration-200 disabled:opacity-40 ${
        checked ? "bg-accent" : "glass-control ring-1 ring-inset ring-hairline"
      }`}
    >
      {/* The on track is the near-white accent, so the knob has to be the
          darkest token to read at all — a hole punched in the pill. That dark
          knob then reads smaller than the light one at the same size
          (irradiation), so it is drawn 18px against the off state's 16px to
          look like one circle sliding across. Both keep a 3-4px gap to the
          track's near edge, and top-1/2 centres whichever size is showing. */}
      <span
        className={`absolute top-1/2 left-[3px] -translate-y-1/2 rounded-full transition-[translate,width,height,background-color] duration-150 ${
          checked
            ? "h-[18px] w-[18px] translate-x-4 bg-sidebar"
            : "h-4 w-4 translate-x-px bg-muted"
        }`}
      />
    </button>
  );
}
