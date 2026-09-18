import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { EmptyRow, Group, PageTitle } from "@/components/ui/Row";
import { IconButton } from "@/components/ui/TextField";
import { CheckIcon, CopyIcon, TrashIcon } from "@/components/ui/icons";
import {
  deleteHistoryEntry,
  initHistoryStore,
  useHistoryStore,
} from "@/stores/history";

/** Past this, a transcript is clamped to four lines with a Show more toggle.
 *  No title tooltip: hover info lives on the Advanced page only. */
const CLAMP_AFTER_CHARS = 260;

export function History() {
  const { t } = useTranslation();
  const entries = useHistoryStore((s) => s.entries);
  const [copied, setCopied] = useState<number | null>(null);
  const [expanded, setExpanded] = useState<number[]>([]);

  useEffect(() => {
    initHistoryStore();
  }, []);

  const copy = async (timestamp: number, text: string) => {
    const result = await commands.copyText(text);
    if (result.status === "ok") {
      setCopied(timestamp);
      setTimeout(() => setCopied((c) => (c === timestamp ? null : c)), 1500);
    }
  };

  const toggleExpanded = (timestamp: number) =>
    setExpanded((list) =>
      list.includes(timestamp)
        ? list.filter((id) => id !== timestamp)
        : [...list, timestamp],
    );

  return (
    <div>
      <PageTitle>{t("sidebar.history")}</PageTitle>
      <Group>
        {entries.length === 0 ? (
          <EmptyRow>{t("history.empty")}</EmptyRow>
        ) : (
          entries.map((entry) => {
            const isCopied = copied === entry.timestamp;
            const isExpanded = expanded.includes(entry.timestamp);
            const clampable = entry.text.length > CLAMP_AFTER_CHARS;
            return (
              <div
                key={entry.timestamp}
                className="group flex items-start gap-3 px-5 py-3.5"
              >
                <div className="min-w-0 flex-1">
                  <p
                    className={`select-text whitespace-pre-wrap break-words text-base leading-relaxed text-text ${
                      clampable && !isExpanded ? "line-clamp-4" : ""
                    }`}
                  >
                    {entry.text}
                  </p>
                  {clampable && (
                    <button
                      type="button"
                      onClick={() => toggleExpanded(entry.timestamp)}
                      className="mt-1 text-xs text-muted transition-colors hover:text-text"
                    >
                      {isExpanded ? t("history.showLess") : t("history.showMore")}
                    </button>
                  )}
                  <p className="mt-2 truncate text-xs text-muted">
                    {entry.app_name}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  <IconButton
                    onClick={() => copy(entry.timestamp, entry.text)}
                    label={isCopied ? t("history.copied") : t("history.copy")}
                  >
                    {isCopied ? (
                      <CheckIcon className="text-success" />
                    ) : (
                      <CopyIcon />
                    )}
                  </IconButton>
                  <IconButton
                    variant="danger"
                    onClick={() => deleteHistoryEntry(entry.timestamp)}
                    label={t("history.delete")}
                  >
                    <TrashIcon />
                  </IconButton>
                </div>
              </div>
            );
          })
        )}
      </Group>
    </div>
  );
}
