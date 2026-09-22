import { useEffect, useRef, type ReactNode } from "react";
import { Button } from "./Button";

interface ConfirmDialogProps {
  title: string;
  /** The muted line under the title: what the confirm actually costs. */
  body?: ReactNode;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/** A modal asked before an action that cannot be undone. It floats, so unlike
 *  the cards it carries a shadow — the same one the InfoTip popover uses. */
export function ConfirmDialog({
  title,
  body,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef<HTMLDivElement>(null);

  // Focus opens on Cancel, never on the destructive button: the dialog only
  // ever guards something irreversible, so a stray Return or Space must not be
  // what deletes the entry.
  useEffect(() => {
    cancelRef.current?.querySelector("button")?.focus();
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
        return;
      }
      if (e.key !== "Tab") return;
      // The dialog is modal but the page behind it is still in the tab order,
      // so Tab is cycled between its own buttons.
      const buttons = dialogRef.current?.querySelectorAll("button");
      if (!buttons?.length) return;
      const first = buttons[0];
      const last = buttons[buttons.length - 1];
      const active = document.activeElement;
      if (!dialogRef.current?.contains(active)) {
        e.preventDefault();
        first.focus();
      } else if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onCancel]);

  return (
    <div
      className="animate-fade-in fixed inset-0 z-50 flex items-center justify-center bg-sidebar/45 px-6 backdrop-blur-md"
      onClick={onCancel}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className="animate-dialog-enter glass-float w-[340px] rounded-card border border-hairline p-5 shadow-[0_16px_48px_rgba(0,0,0,0.5)]"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="text-base font-semibold text-text">{title}</h2>
        {body && (
          <p className="mt-1.5 text-sm leading-snug text-muted">{body}</p>
        )}
        <div className="mt-5 flex justify-end gap-2">
          {/* The ref sits on the wrapper so Button keeps its plain props API. */}
          <div ref={cancelRef} className="contents">
            <Button variant="ghost" onClick={onCancel}>
              {cancelLabel}
            </Button>
          </div>
          <Button variant="danger" onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
