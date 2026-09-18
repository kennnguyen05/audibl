import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { EmptyRow, Group, PageTitle } from "@/components/ui/Row";
import { Button, IconButton, TextField } from "@/components/ui/TextField";
import { CloseIcon, PlusIcon, TrashIcon } from "@/components/ui/icons";

/** One entry in a dictionary card: content left, delete revealed on hover or
 *  keyboard focus, so the resting list stays quiet. */
function EntryRow({
  children,
  onRemove,
  removeLabel,
}: {
  children: ReactNode;
  onRemove: () => void;
  removeLabel: string;
}) {
  return (
    <div className="group flex items-center gap-3 px-5 py-2.5">
      <div className="min-w-0 flex-1 text-base text-text">{children}</div>
      <div className="shrink-0 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
        <IconButton onClick={onRemove} label={removeLabel} variant="danger">
          <TrashIcon />
        </IconButton>
      </div>
    </div>
  );
}

/** The add form, revealed by the card's header action. Distinct fill so it
 *  never reads as one of the entries below it. */
function AddRow({
  children,
  onCancel,
  cancelLabel,
}: {
  children: ReactNode;
  onCancel: () => void;
  cancelLabel: string;
}) {
  return (
    <div className="flex items-center gap-2 bg-control/50 px-5 py-3">
      {children}
      <IconButton onClick={onCancel} label={cancelLabel}>
        <CloseIcon />
      </IconButton>
    </div>
  );
}

export function Dictionary() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const [word, setWord] = useState("");
  const [trigger, setTrigger] = useState("");
  const [value, setValue] = useState("");
  const [addingWord, setAddingWord] = useState(false);
  const [addingReplacement, setAddingReplacement] = useState(false);

  if (!settings) return null;

  const closeWordForm = () => {
    setAddingWord(false);
    setWord("");
  };

  const closeReplacementForm = () => {
    setAddingReplacement(false);
    setTrigger("");
    setValue("");
  };

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

      <Group
        title={t("dictionary.customWords")}
        action={
          addingWord ? undefined : (
            <Button onClick={() => setAddingWord(true)}>
              <PlusIcon size={14} />
              {t("dictionary.addWord")}
            </Button>
          )
        }
      >
        {addingWord && (
          <AddRow onCancel={closeWordForm} cancelLabel={t("dictionary.cancel")}>
            <TextField
              className="min-w-0 flex-1"
              value={word}
              autoFocus
              placeholder={t("dictionary.wordPlaceholder")}
              onChange={(e) => setWord(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") addWord();
                if (e.key === "Escape") closeWordForm();
              }}
            />
            <Button variant="primary" onClick={addWord} disabled={!word.trim()}>
              {t("dictionary.add")}
            </Button>
          </AddRow>
        )}
        {settings.custom_words.length === 0 && !addingWord ? (
          <EmptyRow>{t("dictionary.customWordsEmpty")}</EmptyRow>
        ) : (
          settings.custom_words.map((w, i) => (
            <EntryRow
              key={`${w}-${i}`}
              onRemove={() => removeWord(i)}
              removeLabel={t("dictionary.remove")}
            >
              <span className="block truncate">{w}</span>
            </EntryRow>
          ))
        )}
      </Group>

      <Group
        title={t("dictionary.replacements")}
        action={
          addingReplacement ? undefined : (
            <Button onClick={() => setAddingReplacement(true)}>
              <PlusIcon size={14} />
              {t("dictionary.addReplacement")}
            </Button>
          )
        }
      >
        {addingReplacement && (
          <AddRow
            onCancel={closeReplacementForm}
            cancelLabel={t("dictionary.cancel")}
          >
            <TextField
              className="min-w-0 flex-1"
              value={trigger}
              autoFocus
              placeholder={t("dictionary.triggerPlaceholder")}
              onChange={(e) => setTrigger(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") addReplacement();
                if (e.key === "Escape") closeReplacementForm();
              }}
            />
            <TextField
              className="min-w-0 flex-1"
              value={value}
              placeholder={t("dictionary.valuePlaceholder")}
              onChange={(e) => setValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") addReplacement();
                if (e.key === "Escape") closeReplacementForm();
              }}
            />
            <Button
              variant="primary"
              onClick={addReplacement}
              disabled={!trigger.trim() || !value.trim()}
            >
              {t("dictionary.add")}
            </Button>
          </AddRow>
        )}
        {settings.replacements.length === 0 && !addingReplacement ? (
          <EmptyRow>{t("dictionary.replacementsEmpty")}</EmptyRow>
        ) : (
          settings.replacements.map((r, i) => (
            <EntryRow
              key={`${r.trigger}-${i}`}
              onRemove={() => removeReplacement(i)}
              removeLabel={t("dictionary.remove")}
            >
              <span className="flex min-w-0 items-center gap-2">
                <span className="truncate">{r.trigger}</span>
                <span className="shrink-0 text-muted">→</span>
                <span className="truncate">{r.value}</span>
              </span>
            </EntryRow>
          ))
        )}
      </Group>
    </div>
  );
}
