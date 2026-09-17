import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { PageTitle } from "@/components/ui/Row";
import { Button } from "@/components/ui/TextField";
import {
  deleteHistoryEntry,
  initHistoryStore,
  useHistoryStore,
} from "@/stores/history";

export function History() {
  const { t, i18n } = useTranslation();
  const entries = useHistoryStore((s) => s.entries);
  const [copied, setCopied] = useState<number | null>(null);

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

  const formatTime = (timestamp: number) =>
    new Date(timestamp).toLocaleString(i18n.language, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });

  return (
    <div>
      <PageTitle>{t("sidebar.history")}</PageTitle>
      {entries.length === 0 ? (
        <p className="text-muted">{t("history.empty")}</p>
      ) : (
        <div className="rounded-lg border border-border bg-surface divide-y divide-border">
          {entries.map((entry) => (
            <div key={entry.timestamp} className="px-3 py-2.5">
              <p className="whitespace-pre-wrap break-words select-text">
                {entry.text}
              </p>
              <div className="flex items-center justify-between gap-2 mt-2">
                <span className="text-xs text-muted truncate">
                  {formatTime(entry.timestamp)}
                  {entry.app_name ? ` · ${entry.app_name}` : ""}
                </span>
                <div className="flex gap-2 shrink-0">
                  <Button onClick={() => copy(entry.timestamp, entry.text)}>
                    {copied === entry.timestamp
                      ? t("history.copied")
                      : t("history.copy")}
                  </Button>
                  <Button
                    variant="danger"
                    onClick={() => deleteHistoryEntry(entry.timestamp)}
                  >
                    {t("history.delete")}
                  </Button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
