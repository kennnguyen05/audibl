import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands, type ShortcutCaptureEvent } from "@/bindings";
import { applySettings } from "@/stores/settings";

const SYMBOLS: Record<string, string> = {
  command: "⌘",
  cmd: "⌘",
  option: "⌥",
  alt: "⌥",
  ctrl: "⌃",
  control: "⌃",
  shift: "⇧",
  fn: "fn",
  space: "Space",
  return: "↩",
  enter: "↩",
  escape: "Esc",
  tab: "⇥",
  backspace: "⌫",
};

export function formatShortcut(shortcut: string): string {
  return shortcut
    .split("+")
    .map((part) => {
      const key = part.trim().toLowerCase();
      return SYMBOLS[key] ?? key.toUpperCase();
    })
    .join(" ");
}

interface ShortcutInputProps {
  value: string;
}

/**
 * Click, then press the new combination. A keyed combo commits when its key
 * is released; a modifier-only combo commits when all modifiers are released.
 * Clicking elsewhere cancels.
 */
export function ShortcutInput({ value }: ShortcutInputProps) {
  const { t } = useTranslation();
  const [capturing, setCapturing] = useState(false);
  const [preview, setPreview] = useState("");
  const [error, setError] = useState(false);
  const keyed = useRef("");
  const modifierOnly = useRef("");
  const container = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!capturing) return;
    let done = false;

    const finish = async (shortcut: string | null) => {
      if (done) return;
      done = true;
      await commands.stopShortcutCapture();
      if (shortcut) {
        const result = await commands.changeShortcut(shortcut);
        if (result.status === "ok") {
          applySettings(Promise.resolve(result.data));
        } else {
          setError(true);
        }
      }
      setCapturing(false);
      setPreview("");
    };

    const unlisten = listen<ShortcutCaptureEvent>(
      "shortcut-capture-event",
      (event) => {
        const { hotkey_string, is_key_down, key, modifiers } = event.payload;
        if (is_key_down && hotkey_string) {
          if (key) keyed.current = hotkey_string;
          else modifierOnly.current = hotkey_string;
          setPreview(hotkey_string);
        } else if (!is_key_down && key && keyed.current) {
          finish(keyed.current);
        } else if (
          !is_key_down &&
          !key &&
          modifiers.length === 0 &&
          !keyed.current &&
          modifierOnly.current
        ) {
          finish(modifierOnly.current);
        }
      },
    );

    const onClickOutside = (e: MouseEvent) => {
      if (!container.current?.contains(e.target as Node)) finish(null);
    };
    window.addEventListener("click", onClickOutside);

    return () => {
      window.removeEventListener("click", onClickOutside);
      unlisten.then((fn) => fn());
      if (!done) commands.stopShortcutCapture();
    };
  }, [capturing]);

  const start = async () => {
    if (capturing) return;
    setError(false);
    keyed.current = "";
    modifierOnly.current = "";
    const result = await commands.startShortcutCapture();
    if (result.status === "ok") setCapturing(true);
    else setError(true);
  };

  return (
    <div ref={container} className="flex flex-col items-end gap-1">
      <button
        type="button"
        onClick={start}
        className={`min-w-28 rounded-md border px-2.5 py-1 text-xs font-mono ${
          capturing ? "border-accent text-accent" : "border-border bg-bg"
        }`}
      >
        {capturing
          ? preview
            ? formatShortcut(preview)
            : t("general.pressKeys")
          : formatShortcut(value)}
      </button>
      {error && (
        <span className="text-xs text-danger">{t("general.shortcutError")}</span>
      )}
    </div>
  );
}
