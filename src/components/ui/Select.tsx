import { ChevronDownIcon } from "./icons";

interface SelectProps {
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  label: string;
}

/** The native control is stripped and rebuilt: macOS WebKit imposes its own
 *  min-height and padding on <select>, which ignores height and font classes.
 *  Width tracks the selected option's own label (short options sit tight,
 *  long ones expand) rather than a fixed box sized for the longest one. */
export function Select({ value, options, onChange, label }: SelectProps) {
  const selectedLabel =
    options.find((option) => option.value === value)?.label ?? "";

  return (
    <div className="relative flex h-8 items-center">
      <select
        aria-label={label}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        style={{
          width: `clamp(6rem, ${selectedLabel.length}ch + 2.75rem, 15rem)`,
        }}
        className="glass-control h-8 appearance-none truncate rounded-control border border-hairline py-0 pl-3 pr-8 text-sm leading-8 text-text transition-colors duration-200 hover:border-muted"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      <ChevronDownIcon
        size={14}
        className="pointer-events-none absolute right-2.5 text-muted"
      />
    </div>
  );
}
