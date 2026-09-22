import type { InputHTMLAttributes } from "react";

export function TextField(props: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={`glass-well h-8 rounded-control border border-hairline px-3 text-sm text-text transition-colors duration-200 placeholder:text-muted/70 focus:border-muted disabled:opacity-50 ${
        props.className ?? ""
      }`}
    />
  );
}

export { Button, IconButton } from "./Button";
