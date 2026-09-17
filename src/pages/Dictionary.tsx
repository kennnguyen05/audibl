import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { Group, PageTitle } from "@/components/ui/Row";
import { Button, TextField } from "@/components/ui/TextField";

export function Dictionary() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const [word, setWord] = useState("");
  const [trigger, setTrigger] = useState("");
  const [value, setValue] = useState("");

  if (!settings) return null;

  const addWord = async () => {
    if (!word.trim()) return;
    await applySettings(
      commands.setCustomWords([...settings.custom_words, word]),
    );
    setWord("");
  };

  const removeWord = (index: number) =>
    applySettings(
      commands.setCustomWords(
        settings.custom_words.filter((_, i) => i !== index),
      ),
    );

  const addReplacement = async () => {
    if (!trigger.trim() || !value.trim()) return;
    await applySettings(
      commands.setReplacements([
        ...settings.replacements,
        { trigger, value },
      ]),
    );
    setTrigger("");
    setValue("");
  };

  const removeReplacement = (index: number) =>
    applySettings(
      commands.setReplacements(
        settings.replacements.filter((_, i) => i !== index),
      ),
    );

  return (
    <div>
      <PageTitle>{t("sidebar.dictionary")}</PageTitle>

      <Group title={t("dictionary.customWords")}>
        <div className="px-3 py-2.5 flex gap-2">
          <TextField
            className="flex-1"
            value={word}
            placeholder={t("dictionary.wordPlaceholder")}
            onChange={(e) => setWord(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addWord()}
          />
          <Button onClick={addWord} disabled={!word.trim()}>
            {t("dictionary.add")}
          </Button>
        </div>
        {settings.custom_words.length > 0 && (
          <div className="px-3 py-2.5 flex flex-wrap gap-1.5">
            {settings.custom_words.map((w, i) => (
              <span
                key={w}
                className="inline-flex items-center gap-1 rounded-full bg-control pl-2.5 pr-1 py-0.5 text-xs"
              >
                {w}
                <button
                  type="button"
                  aria-label={t("dictionary.remove")}
                  onClick={() => removeWord(i)}
                  className="w-4 h-4 rounded-full text-muted hover:text-text"
                >
                  ×
                </button>
              </span>
            ))}
          </div>
        )}
      </Group>

      <Group title={t("dictionary.replacements")}>
        <div className="px-3 py-2.5 flex gap-2">
          <TextField
            className="flex-1 min-w-0"
            value={trigger}
            placeholder={t("dictionary.triggerPlaceholder")}
            onChange={(e) => setTrigger(e.target.value)}
          />
          <TextField
            className="flex-1 min-w-0"
            value={value}
            placeholder={t("dictionary.valuePlaceholder")}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addReplacement()}
          />
          <Button
            onClick={addReplacement}
            disabled={!trigger.trim() || !value.trim()}
          >
            {t("dictionary.add")}
          </Button>
        </div>
        {settings.replacements.map((r, i) => (
          <div
            key={r.trigger}
            className="px-3 py-2 flex items-center gap-2 text-xs"
          >
            <span className="flex-1 min-w-0 truncate">{r.trigger}</span>
            <span className="text-muted">→</span>
            <span className="flex-1 min-w-0 truncate font-mono">{r.value}</span>
            <Button onClick={() => removeReplacement(i)} variant="danger">
              {t("dictionary.remove")}
            </Button>
          </div>
        ))}
      </Group>
    </div>
  );
}
