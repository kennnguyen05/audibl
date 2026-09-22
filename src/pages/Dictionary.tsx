import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { commands, type Replacement } from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { EmptyRow, Group, PageTitle } from "@/components/ui/Row";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { Button, IconButton, TextField } from "@/components/ui/TextField";
import {
  CloseIcon,
  PencilIcon,
  PlusCircleIcon,
  TrashIcon,
} from "@/components/ui/icons";

/** The plus hangs in a gutter inside the card's 20px inset, so every row's text
 *  — the add row's label and each entry — starts on the same column: 20px of
 *  padding, an 18px icon, a 12px gap. */
const ROW_TEXT_INSET = "pl-[50px] pr-5";

/** One entry in a dictionary card: content left, edit and delete right. The
 *  actions stay visible at rest — a hover-only affordance hid the only way to
 *  correct a typed entry. */
function EntryRow({
  children,
  onEdit,
  editLabel,
  onRemove,
  removeLabel,
}: {
  children: ReactNode;
  onEdit: () => void;
  editLabel: string;
  onRemove: () => void;
  removeLabel: string;
}) {
  return (
    <div className={`flex items-center gap-3 py-2.5 ${ROW_TEXT_INSET}`}>
      <div className="min-w-0 flex-1 text-base text-text">{children}</div>
      <div className="flex shrink-0 items-center gap-1">
        <IconButton onClick={onEdit} label={editLabel}>
          <PencilIcon />
        </IconButton>
        <IconButton onClick={onRemove} label={removeLabel} variant="danger">
          <TrashIcon />
        </IconButton>
      </div>
    </div>
  );
}

/** The add and edit form, revealed by the card's header action or an entry's
 *  edit button. Distinct fill so it never reads as one of the entries. */
function FormRow({
  children,
  onCancel,
  cancelLabel,
}: {
  children: ReactNode;
  onCancel: () => void;
  cancelLabel: string;
}) {
  return (
    <div className="glass-well flex items-center gap-2 px-5 py-3">
      {children}
      <IconButton onClick={onCancel} label={cancelLabel}>
        <CloseIcon />
      </IconButton>
    </div>
  );
}

/** The add affordance as the card's first row, not a header button: a filled
 *  band in the palette's one near-white, inverted the way `Button`'s primary
 *  is. `min-h-14` is the open form's height (py-3 around a 32px field), so
 *  clicking swaps one for the other without the card resizing. */
function AddRow({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex min-h-14 w-full items-center gap-3 bg-text px-5 py-3 text-left text-base font-medium text-sidebar transition-colors duration-200 hover:bg-white"
    >
      <PlusCircleIcon size={18} className="shrink-0" />
      {label}
    </button>
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
  // Index of the entry being edited in place, or null when none is.
  const [editingWord, setEditingWord] = useState<number | null>(null);
  const [editingReplacement, setEditingReplacement] = useState<number | null>(
    null,
  );
  // The entry whose delete is waiting on the confirmation dialog, held by its
  // content rather than its index: `settings-changed` can reorder the list
  // while the dialog is open, and an index would then name — and delete — a
  // different entry than the one the dialog asked about.
  const [confirm, setConfirm] = useState<
    | { kind: "word"; word: string }
    | { kind: "replacement"; replacement: Replacement }
    | null
  >(null);

  if (!settings) return null;

  const closeWordForm = () => {
    setAddingWord(false);
    setEditingWord(null);
    setWord("");
  };

  const closeReplacementForm = () => {
    setAddingReplacement(false);
    setEditingReplacement(null);
    setTrigger("");
    setValue("");
  };

  const openWordForm = () => {
    closeWordForm();
    setAddingWord(true);
  };

  const openReplacementForm = () => {
    closeReplacementForm();
    setAddingReplacement(true);
  };

  const startEditWord = (index: number) => {
    setAddingWord(false);
    setEditingWord(index);
    setWord(settings.custom_words[index]);
  };

  const startEditReplacement = (index: number) => {
    setAddingReplacement(false);
    setEditingReplacement(index);
    setTrigger(settings.replacements[index].trigger);
    setValue(settings.replacements[index].value);
  };

  const addWord = async () => {
    if (!word.trim()) return;
    await applySettings(
      commands.setCustomWords([...settings.custom_words, word]),
    );
    closeWordForm();
  };

  const saveWord = async (index: number) => {
    if (!word.trim()) return;
    await applySettings(
      commands.setCustomWords(
        settings.custom_words.map((w, i) => (i === index ? word : w)),
      ),
    );
    closeWordForm();
  };

  const removeWord = (word: string) => {
    closeWordForm();
    const index = settings.custom_words.indexOf(word);
    if (index < 0) return;
    return applySettings(
      commands.setCustomWords(
        settings.custom_words.filter((_, i) => i !== index),
      ),
    );
  };

  const addReplacement = async () => {
    if (!trigger.trim() || !value.trim()) return;
    await applySettings(
      commands.setReplacements([
        ...settings.replacements,
        { trigger, value },
      ]),
    );
    closeReplacementForm();
  };

  const saveReplacement = async (index: number) => {
    if (!trigger.trim() || !value.trim()) return;
    await applySettings(
      commands.setReplacements(
        settings.replacements.map((r, i) =>
          i === index ? { trigger, value } : r,
        ),
      ),
    );
    closeReplacementForm();
  };

  const removeReplacement = (target: Replacement) => {
    closeReplacementForm();
    const index = settings.replacements.findIndex(
      (r) => r.trigger === target.trigger && r.value === target.value,
    );
    if (index < 0) return;
    return applySettings(
      commands.setReplacements(
        settings.replacements.filter((_, i) => i !== index),
      ),
    );
  };

  // Checked against the live settings, so an entry removed elsewhere closes the
  // dialog instead of confirming a delete that no longer means anything.
  const confirmWord =
    confirm?.kind === "word" && settings.custom_words.includes(confirm.word)
      ? confirm.word
      : undefined;
  const confirmReplacement =
    confirm?.kind === "replacement" &&
    settings.replacements.some(
      (r) =>
        r.trigger === confirm.replacement.trigger &&
        r.value === confirm.replacement.value,
    )
      ? confirm.replacement
      : undefined;

  return (
    <div>
      <PageTitle>{t("sidebar.dictionary")}</PageTitle>

      <Group title={t("dictionary.customWords")}>
        {addingWord ? (
          <FormRow onCancel={closeWordForm} cancelLabel={t("dictionary.cancel")}>
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
          </FormRow>
        ) : (
          <AddRow label={t("dictionary.addWord")} onClick={openWordForm} />
        )}
        {settings.custom_words.length === 0 && !addingWord ? (
          <EmptyRow>{t("dictionary.customWordsEmpty")}</EmptyRow>
        ) : (
          settings.custom_words.map((w, i) =>
            editingWord === i ? (
              <FormRow
                key={`edit-${i}`}
                onCancel={closeWordForm}
                cancelLabel={t("dictionary.cancel")}
              >
                <TextField
                  className="min-w-0 flex-1"
                  value={word}
                  autoFocus
                  placeholder={t("dictionary.wordPlaceholder")}
                  onChange={(e) => setWord(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") saveWord(i);
                    if (e.key === "Escape") closeWordForm();
                  }}
                />
                <Button
                  variant="primary"
                  onClick={() => saveWord(i)}
                  disabled={!word.trim()}
                >
                  {t("dictionary.save")}
                </Button>
              </FormRow>
            ) : (
              <EntryRow
                key={`${w}-${i}`}
                onEdit={() => startEditWord(i)}
                editLabel={t("dictionary.edit")}
                onRemove={() => setConfirm({ kind: "word", word: w })}
                removeLabel={t("dictionary.remove")}
              >
                <span className="block truncate">{w}</span>
              </EntryRow>
            ),
          )
        )}
      </Group>

      <Group title={t("dictionary.replacements")}>
        {addingReplacement ? (
          <FormRow
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
          </FormRow>
        ) : (
          <AddRow
            label={t("dictionary.addReplacement")}
            onClick={openReplacementForm}
          />
        )}
        {settings.replacements.length === 0 && !addingReplacement ? (
          <EmptyRow>{t("dictionary.replacementsEmpty")}</EmptyRow>
        ) : (
          settings.replacements.map((r, i) =>
            editingReplacement === i ? (
              <FormRow
                key={`edit-${i}`}
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
                    if (e.key === "Enter") saveReplacement(i);
                    if (e.key === "Escape") closeReplacementForm();
                  }}
                />
                <TextField
                  className="min-w-0 flex-1"
                  value={value}
                  placeholder={t("dictionary.valuePlaceholder")}
                  onChange={(e) => setValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") saveReplacement(i);
                    if (e.key === "Escape") closeReplacementForm();
                  }}
                />
                <Button
                  variant="primary"
                  onClick={() => saveReplacement(i)}
                  disabled={!trigger.trim() || !value.trim()}
                >
                  {t("dictionary.save")}
                </Button>
              </FormRow>
            ) : (
              <EntryRow
                key={`${r.trigger}-${i}`}
                onEdit={() => startEditReplacement(i)}
                editLabel={t("dictionary.edit")}
                onRemove={() =>
                  setConfirm({ kind: "replacement", replacement: r })
                }
                removeLabel={t("dictionary.remove")}
              >
                <span className="flex min-w-0 items-center gap-2">
                  <span className="truncate">{r.trigger}</span>
                  <span className="shrink-0 text-muted">→</span>
                  <span className="truncate">{r.value}</span>
                </span>
              </EntryRow>
            ),
          )
        )}
      </Group>

      {confirmWord !== undefined && (
        <ConfirmDialog
          title={t("dictionary.deleteWordTitle", { word: confirmWord })}
          body={t("dictionary.deleteWordBody")}
          confirmLabel={t("dictionary.remove")}
          cancelLabel={t("dictionary.cancel")}
          onConfirm={() => {
            removeWord(confirmWord);
            setConfirm(null);
          }}
          onCancel={() => setConfirm(null)}
        />
      )}

      {confirmReplacement !== undefined && (
        <ConfirmDialog
          title={t("dictionary.deleteReplacementTitle", {
            trigger: confirmReplacement.trigger,
          })}
          body={t("dictionary.deleteReplacementBody", {
            value: confirmReplacement.value,
          })}
          confirmLabel={t("dictionary.remove")}
          cancelLabel={t("dictionary.cancel")}
          onConfirm={() => {
            removeReplacement(confirmReplacement);
            setConfirm(null);
          }}
          onCancel={() => setConfirm(null)}
        />
      )}
    </div>
  );
}
