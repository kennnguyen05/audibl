import type { ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonProps {
  onClick: () => void;
  children: ReactNode;
  disabled?: boolean;
  variant?: Variant;
  /** Fills the width of its container, for onboarding and empty states. */
  block?: boolean;
  type?: "button" | "submit";
}

// Primary inverts the palette the way Wispr Flow's black-on-cream pill does:
// on dark, maximum contrast means a near-white fill.
const VARIANTS: Record<Variant, string> = {
  primary: "bg-text text-sidebar hover:bg-white",
  secondary: "border border-border bg-control text-text hover:border-muted",
  ghost: "text-muted hover:text-text hover:bg-control",
  danger: "text-danger hover:bg-danger/10",
};

export function Button({
  onClick,
  children,
  disabled,
  variant = "secondary",
  block,
  type = "button",
}: ButtonProps) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className={`inline-flex h-8 shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-control px-3.5 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-40 ${
        VARIANTS[variant]
      } ${block ? "w-full" : ""}`}
    >
      {children}
    </button>
  );
}

interface IconButtonProps {
  onClick: () => void;
  children: ReactNode;
  label: string;
  disabled?: boolean;
  /** `bare`: the glyph is the whole button — no fill on hover, only colour. */
  variant?: "ghost" | "danger" | "bare";
}

const ICON_VARIANTS = {
  ghost: "hover:bg-control hover:text-text",
  danger: "hover:bg-danger/10 hover:text-danger",
  bare: "hover:text-text",
};

/** Square 28px action, for the row-hover delete and the shortcut reset. */
export function IconButton({
  onClick,
  children,
  label,
  disabled,
  variant = "ghost",
}: IconButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      title={label}
      className={`inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-control text-muted transition-colors disabled:pointer-events-none disabled:opacity-30 ${ICON_VARIANTS[variant]}`}
    >
      {children}
    </button>
  );
}
