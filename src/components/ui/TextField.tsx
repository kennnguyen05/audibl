import type { InputHTMLAttributes } from "react";

export function TextField(props: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={`rounded-md border border-border bg-bg px-2 py-1 text-xs outline-none focus:border-accent ${
        props.className ?? ""
      }`}
    />
  );
}

interface ButtonProps {
  onClick: () => void;
  children: string;
  disabled?: boolean;
  variant?: "default" | "danger";
}

export function Button({ onClick, children, disabled, variant }: ButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`rounded-md border border-border px-2.5 py-1 text-xs hover:bg-control disabled:opacity-40 ${
        variant === "danger" ? "text-danger" : ""
      }`}
    >
      {children}
    </button>
  );
}
