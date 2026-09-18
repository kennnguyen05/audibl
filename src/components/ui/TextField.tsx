import type { InputHTMLAttributes } from "react";

export function TextField(props: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={`h-8 rounded-control border border-border bg-bg px-3 text-sm text-text transition-colors placeholder:text-muted/70 focus:border-muted disabled:opacity-50 ${
        props.className ?? ""
      }`}
    />
  );
}

export { Button, IconButton } from "./Button";
