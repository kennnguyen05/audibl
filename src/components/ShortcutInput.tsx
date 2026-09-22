import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands, type ShortcutCaptureEvent } from "@/bindings";
import { applySettings } from "@/stores/settings";
import { IconButton } from "@/components/ui/Button";
import { ResetIcon } from "@/components/ui/icons";

// Must match `DEFAULT_SHORTCUT` in src-tauri/src/settings.rs.
const DEFAULT_SHORTCUT = "option+space";

// Keycap symbol/readable-name pairs. Left/right variants (e.g.
// `option_left`) fall back to the base key's symbol below.
const SYMBOLS: Record<string, { symbol: string; name: string }> = {
  command: { symbol: "⌘", name: "Command" },
  cmd: { symbol: "⌘", name: "Command" },
  option: { symbol: "⌥", name: "Option" },
  alt: { symbol: "⌥", name: "Option" },
  ctrl: { symbol: "⌃", name: "Control" },
  control: { symbol: "⌃", name: "Control" },
  shift: { symbol: "⇧", name: "Shift" },
  fn: { symbol: "fn", name: "Fn" },
  space: { symbol: "␣", name: "Space" },
  return: { symbol: "↩", name: "Return" },
  enter: { symbol: "↩", name: "Return" },
  escape: { symbol: "⎋", name: "Escape" },
  esc: { symbol: "⎋", name: "Escape" },
  tab: { symbol: "⇥", name: "Tab" },
  backspace: { symbol: "⌫", name: "Backspace" },
  delete: { symbol: "⌫", name: "Delete" },
  up: { symbol: "↑", name: "Up Arrow" },
  down: { symbol: "↓", name: "Down Arrow" },
  left: { symbol: "←", name: "Left Arrow" },
  right: { symbol: "→", name: "Right Arrow" },
  arrowup: { symbol: "↑", name: "Up Arrow" },
  arrowdown: { symbol: "↓", name: "Down Arrow" },
  arrowleft: { symbol: "←", name: "Left Arrow" },
  arrowright: { symbol: "→", name: "Right Arrow" },
};

/** Strips a `_left`/`_right` suffix so e.g. `option_left` maps to `option`. */
function baseKey(key: string): string {
  return key.replace(/_(left|right)$/, "");
}

interface ShortcutKey {
  /** Stable id for the React key. */
  id: string;
  /** What's shown on the keycap. */
  display: string;
  /** Accessible name for this one key. */
  name: string;
}

function parseShortcut(shortcut: string): ShortcutKey[] {
  return shortcut
    .split("+")
    .map((part, i) => {
      const raw = part.trim().toLowerCase();
      const key = baseKey(raw);
      const known = SYMBOLS[key];
      return {
        id: `${raw}-${i}`,
        display: known ? known.symbol : raw.toUpperCase(),
        name: known ? known.name : raw.toUpperCase(),
      };
    })
    .filter((k) => k.display.length > 0);
}

/** Plain readable text, used for aria-labels and error/preview strings. */
export function formatShortcut(shortcut: string): string {
  return parseShortcut(shortcut)
    .map((k) => k.name)
    .join(" + ");
}

function Keycap({ children, spaced }: { children: string; spaced: boolean }) {
  return (
    <span
      className={`inline-flex h-4 items-center justify-center text-base leading-none ${spaced ? "mx-0.5" : ""}`}
    >
      {children}
    </span>
  );
}

/** Renders a shortcut as symbols flush together, e.g. ⌥⌘⇧␣, matching macOS
 * menu shortcut hints. Multi-character keys (letters, F-keys) get a little
 * breathing room so they don't run into their neighbors. The enclosing
 * button carries the readable aria-label, so this is decorative only. */
function ShortcutKeycaps({ shortcut }: { shortcut: string }) {
  const keys = parseShortcut(shortcut);
  return (
    // The modifier glyphs come from the system font's fallback, whose ink
    // sits a few pixels above the line box's centre; the nudge centres it.
    <span className="flex translate-y-[3px] items-center" aria-hidden="true">
      {keys.map((k) => (
        <Keycap key={k.id} spaced={k.display.length > 1}>
          {k.display}
        </Keycap>
      ))}
    </span>
  );
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
    <div ref={container} className="flex flex-col items-start gap-1">
      <button
        type="button"
        onClick={start}
        aria-label={
          capturing
            ? preview
              ? formatShortcut(preview)
              : t("general.pressKeys")
            : formatShortcut(value)
        }
        className={`glass-control flex h-7 w-fit items-center justify-center rounded-control border px-2.5 transition-colors duration-200 ${
          capturing
            ? "border-accent text-accent"
            : "border-hairline text-text hover:border-muted"
        }`}
      >
        {capturing ? (
          preview ? (
            <ShortcutKeycaps shortcut={preview} />
          ) : (
            <span className="text-sm">{t("general.pressKeys")}</span>
          )
        ) : (
          <ShortcutKeycaps shortcut={value} />
        )}
      </button>
      {error && (
        <span className="text-sm text-danger">{t("general.shortcutError")}</span>
      )}
    </div>
  );
}

interface ResetShortcutButtonProps {
  shortcut: string;
}

/** Icon-only button that resets the shortcut back to the app default. */
export function ResetShortcutButton({ shortcut }: ResetShortcutButtonProps) {
  const { t } = useTranslation();
  const isDefault = shortcut.trim().toLowerCase() === DEFAULT_SHORTCUT;

  const reset = async () => {
    if (isDefault) return;
    const result = await commands.changeShortcut(DEFAULT_SHORTCUT);
    if (result.status === "ok") {
      applySettings(Promise.resolve(result.data));
    }
  };

  return (
    <IconButton
      variant="bare"
      onClick={reset}
      disabled={isDefault}
      label={t("general.resetShortcut")}
    >
      <ResetIcon size={22} />
    </IconButton>
  );
}
