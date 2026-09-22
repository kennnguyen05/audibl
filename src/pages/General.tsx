import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type ActivationMode } from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { Group, PageTitle, Row } from "@/components/ui/Row";
import { Segmented } from "@/components/ui/Segmented";
import { Select } from "@/components/ui/Select";
import { Toggle } from "@/components/ui/Toggle";
import { Button, TextField } from "@/components/ui/TextField";
import { ResetShortcutButton, ShortcutInput } from "@/components/ShortcutInput";
import { CheckIcon } from "@/components/ui/icons";

const DEFAULT_MIC = "__default__";
const ALL_CHANNELS = "__all__";

export function General() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const [microphones, setMicrophones] = useState<string[]>([]);
  const [channelCount, setChannelCount] = useState(1);
  const [hasKey, setHasKey] = useState(false);
  // Editable field is shown when there's no key yet, or the user chose to
  // replace or remove the saved one. Once a key is saved the field shows a
  // redacted placeholder instead, and the real key never reaches the DOM.
  const [editing, setEditing] = useState(false);
  const [keyInput, setKeyInput] = useState("");
  const [keyError, setKeyError] = useState<string | null>(null);
  const [keySaving, setKeySaving] = useState(false);
  const [cleanWarning, setCleanWarning] = useState(false);

  useEffect(() => {
    commands.getMicrophones().then((r) => {
      if (r.status === "ok") setMicrophones(r.data);
    });
  }, []);

  useEffect(() => {
    commands.hasGroqApiKey().then((has) => {
      setHasKey(has);
      setEditing(!has);
    });
  }, []);

  // Channel choice only appears for multi-channel devices.
  const selectedMicrophone = settings?.selected_microphone;
  useEffect(() => {
    commands.getChannelCount().then((r) => {
      setChannelCount(r.status === "ok" ? r.data : 1);
    });
  }, [selectedMicrophone]);

  if (!settings) return null;

  const keyErrorMessage = (code: string) => {
    switch (code) {
      case "empty_key":
        return t("general.keyEmpty");
      case "invalid_key":
        return t("general.keyInvalid");
      case "unreachable":
        return t("general.keyUnreachable");
      default:
        return t("general.keySaveFailed");
    }
  };

  const saveKey = async () => {
    const trimmed = keyInput.trim();
    if (!trimmed) return;
    setKeyError(null);
    setKeySaving(true);
    const result = await commands.setGroqApiKey(trimmed);
    setKeySaving(false);
    if (result.status === "ok") {
      setKeyInput("");
      setHasKey(true);
      setEditing(false);
      setCleanWarning(false);
    } else {
      setKeyError(keyErrorMessage(result.error));
    }
  };

  const removeKey = async () => {
    setKeyError(null);
    const result = await commands.clearGroqApiKey();
    if (result.status === "ok") {
      applySettings(Promise.resolve(result.data));
      setHasKey(false);
      setEditing(true);
      setKeyInput("");
    } else {
      setKeyError(t("general.keyRemoveFailed"));
    }
  };

  const startReplace = () => {
    setKeyError(null);
    setKeyInput("");
    setEditing(true);
  };

  // Back to the redacted view; the saved key is untouched.
  const cancelReplace = () => {
    setKeyError(null);
    setKeyInput("");
    setEditing(false);
  };

  const setCleanAndReformat = async (enabled: boolean) => {
    const result = await commands.setCleanAndReformat(enabled);
    if (result.status === "ok") {
      applySettings(Promise.resolve(result.data));
      setCleanWarning(false);
    } else {
      setCleanWarning(true);
    }
  };

  const openGroqKeysPage = () => {
    commands.openGroqKeysPage();
  };

  return (
    <div>
      <PageTitle>{t("sidebar.general")}</PageTitle>

      <Group>
        <Row
          label={t("general.transcribeShortcut")}
          caption={t("general.shortcutCaption")}
        >
          <ShortcutInput value={settings.shortcut} />
          <ResetShortcutButton shortcut={settings.shortcut} />
        </Row>
        <Row
          label={t("general.activationMode")}
          caption={t(`general.${settings.activation_mode}Caption`)}
        >
          <Segmented<ActivationMode>
            label={t("general.activationMode")}
            value={settings.activation_mode}
            options={[
              { value: "auto", label: t("general.auto") },
              { value: "hold", label: t("general.hold") },
              { value: "toggle", label: t("general.toggle") },
            ]}
            onChange={(mode) => applySettings(commands.setActivationMode(mode))}
          />
        </Row>

        <Row label={t("general.microphone")}>
          <Select
            label={t("general.microphone")}
            value={settings.selected_microphone ?? DEFAULT_MIC}
            options={[
              { value: DEFAULT_MIC, label: t("general.defaultMicrophone") },
              ...[
                ...microphones,
                // Keep a disconnected selection visible.
                ...(settings.selected_microphone &&
                !microphones.includes(settings.selected_microphone)
                  ? [settings.selected_microphone]
                  : []),
              ].map((name) => ({ value: name, label: name })),
            ]}
            onChange={(value) =>
              applySettings(
                commands.setMicrophone(value === DEFAULT_MIC ? null : value),
              )
            }
          />
        </Row>
        {channelCount > 1 && (
          <Row label={t("general.channel")}>
            <Select
              label={t("general.channel")}
              value={
                settings.selected_channel === null
                  ? ALL_CHANNELS
                  : String(settings.selected_channel)
              }
              options={[
                { value: ALL_CHANNELS, label: t("general.allChannels") },
                ...Array.from({ length: channelCount }, (_, i) => ({
                  value: String(i),
                  label: t("general.channelNumber", { number: i + 1 }),
                })),
              ]}
              onChange={(value) =>
                applySettings(
                  commands.setChannel(
                    value === ALL_CHANNELS ? null : Number(value),
                  ),
                )
              }
            />
          </Row>
        )}
        <Row label={t("general.muteWhileRecording")}>
          <Toggle
            label={t("general.muteWhileRecording")}
            checked={settings.mute_while_recording}
            onChange={(enabled) =>
              applySettings(commands.setMuteWhileRecording(enabled))
            }
          />
        </Row>
      </Group>

      <Group title={t("general.cleanAndReformat")}>
        <Row
          label={t("general.useCleanAndReformat")}
          caption={
            <>
              {t("general.cleanAndReformatCaption")}{" "}
              <button
                type="button"
                className="underline underline-offset-2 transition-colors hover:text-accent"
                onClick={() => setCleanWarning(true)}
              >
                {t("general.showMeHow")}
              </button>
            </>
          }
        >
          <Toggle
            label={t("general.useCleanAndReformat")}
            checked={settings.clean_and_reformat}
            onChange={setCleanAndReformat}
          />
        </Row>
        <Row
          label={
            <span className="flex w-full items-center justify-between gap-4">
              {t("general.groqApiKey")}
              {hasKey ? (
                <span className="flex items-center gap-1 text-sm text-success">
                  <CheckIcon size={14} className="relative -top-0.5" />
                  {t("general.keyStatusSaved")}
                </span>
              ) : (
                <span className="text-sm text-muted">
                  {t("general.keyStatusNotSaved")}
                </span>
              )}
            </span>
          }
          stacked
        >
          <div className="flex items-center gap-2">
            {editing ? (
              <>
                <TextField
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  className="min-w-0 flex-1"
                  value={keyInput}
                  placeholder={t("general.keyPlaceholder")}
                  onChange={(e) => {
                    setKeyInput(e.target.value);
                    setKeyError(null);
                  }}
                  onKeyDown={(e) =>
                    e.key === "Enter" && keyInput.trim() && saveKey()
                  }
                />
                <Button
                  onClick={saveKey}
                  variant="primary"
                  disabled={!keyInput.trim() || keySaving}
                >
                  {t("general.save")}
                </Button>
                {hasKey && (
                  <>
                    <Button onClick={cancelReplace} variant="secondary">
                      {t("general.cancel")}
                    </Button>
                    <Button onClick={removeKey} variant="danger">
                      {t("general.remove")}
                    </Button>
                  </>
                )}
              </>
            ) : (
              <>
                {/* Redacted display only: the real key never reaches the
                    frontend, so this placeholder is a fixed decorative
                    string, not derived from the saved key. Disabled so it
                    can't be focused, selected, or copied. */}
                <TextField
                  type="text"
                  disabled
                  value=""
                  placeholder={t("general.keyRedacted")}
                  aria-label={t("general.groqApiKey")}
                  className="min-w-0 flex-1"
                  style={{ userSelect: "none" }}
                />
                <Button onClick={startReplace} variant="secondary">
                  {t("general.replace")}
                </Button>
                <Button onClick={removeKey} variant="danger">
                  {t("general.remove")}
                </Button>
              </>
            )}
          </div>
          {keyError && (
            <p className="mt-2 text-sm text-danger">{keyError}</p>
          )}
        </Row>
        {/* A card row of its own, so the warning sits under the setting that
            raised it instead of floating outside the group. */}
        {cleanWarning && (
          <p className="animate-fade-in px-5 py-3.5 text-sm leading-snug text-warning">
            {t("general.cleanNeedsKeyBefore")}
            <button
              type="button"
              className="underline underline-offset-2 transition-colors hover:text-accent"
              onClick={openGroqKeysPage}
            >
              {t("general.cleanNeedsKeyLink")}
            </button>
            {t("general.cleanNeedsKeyAfter")}
          </p>
        )}
      </Group>
    </div>
  );
}
