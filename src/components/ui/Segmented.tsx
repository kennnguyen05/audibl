interface SegmentedProps<T extends string> {
  value: T;
  options: { value: T; label: string; description?: string }[];
  onChange: (value: T) => void;
  label: string;
  /** Full-width buttons that carry a description under each label. */
  size?: "small" | "large";
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
  size = "small",
}: SegmentedProps<T>) {
  const large = size === "large";
  return (
    <div
      role="radiogroup"
      aria-label={label}
      className={
        large
          ? "flex w-full gap-1.5 rounded-card bg-bg p-1.5 ring-1 ring-inset ring-border"
          : "inline-flex gap-0.5 rounded-control bg-bg p-0.5 ring-1 ring-inset ring-border"
      }
    >
      {options.map((option) => {
        const selected = value === option.value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={selected}
            onClick={() => onChange(option.value)}
            className={
              large
                ? // Both states render label and description at the same size,
                  // so selecting never changes the control's height.
                  `flex flex-1 flex-col items-start justify-center gap-0.5 rounded-control px-4 py-2.5 text-left transition-colors ${
                    selected
                      ? "bg-control ring-1 ring-inset ring-border"
                      : "hover:bg-control/40"
                  }`
                : `rounded-[6px] px-3 py-1 text-sm font-medium transition-colors ${
                    selected ? "bg-control text-text" : "text-muted hover:text-text"
                  }`
            }
          >
            {large ? (
              <>
                <span
                  className={`text-base font-semibold ${
                    selected ? "text-text" : "text-muted"
                  }`}
                >
                  {option.label}
                </span>
                {option.description && (
                  <span className="text-sm leading-snug text-muted">
                    {option.description}
                  </span>
                )}
              </>
            ) : (
              option.label
            )}
          </button>
        );
      })}
    </div>
  );
}
