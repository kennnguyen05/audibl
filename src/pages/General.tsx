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
  const [keySuccess, setKeySuccess] = useState(false);
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

  const isToggle = settings.activation_mode === "toggle";

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
    setKeySuccess(false);
    setKeySaving(true);
    const result = await commands.setGroqApiKey(trimmed);
    setKeySaving(false);
    if (result.status === "ok") {
      setKeyInput("");
      setHasKey(true);
      setEditing(false);
      setCleanWarning(false);
      setKeySuccess(true);
      window.setTimeout(() => setKeySuccess(false), 2500);
    } else {
      setKeyError(keyErrorMessage(result.error));
    }
  };

  const removeKey = async () => {
    setKeyError(null);
    setKeySuccess(false);
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
    setKeySuccess(false);
    setKeyInput("");
    setEditing(true);
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

      <Group title={t("general.shortcutGroup")}>
        <Row label={t("general.transcribeShortcut")}>
          <ShortcutInput value={settings.shortcut} />
          <ResetShortcutButton shortcut={settings.shortcut} />
        </Row>
        <Row
          label={t("general.activationMode")}
          caption={isToggle ? t("general.toggleCaption") : undefined}
        >
          <Segmented<ActivationMode>
            label={t("general.activationMode")}
            value={settings.activation_mode}
            options={[
              { value: "hold", label: t("general.hold") },
              { value: "toggle", label: t("general.toggle") },
            ]}
            onChange={(mode) => applySettings(commands.setActivationMode(mode))}
          />
        </Row>
      </Group>

      <Group title={t("general.soundGroup")}>
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

      <Group title={t("general.cleanGroup")}>
        <Row
          label={t("general.cleanAndReformat")}
          caption={t("general.cleanAndReformatCaption")}
        >
          <Toggle
            label={t("general.cleanAndReformat")}
            checked={settings.clean_and_reformat}
            onChange={setCleanAndReformat}
          />
        </Row>
        <Row
          label={t("general.groqApiKey")}
          caption={t("general.groqApiKeyCaption")}
        >
          <span className={`text-xs ${hasKey ? "" : "text-muted"}`}>
            {hasKey ? t("general.keySaved") : t("general.keyNotSet")}
          </span>
        </Row>
        <div className="px-3 py-2.5 flex gap-2">
          {editing ? (
            <>
              <TextField
                type="password"
                autoComplete="off"
                spellCheck={false}
                className="flex-1"
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
                disabled={!keyInput.trim() || keySaving}
              >
                {t("general.save")}
              </Button>
              {hasKey && (
                <Button onClick={removeKey} variant="danger">
                  {t("general.remove")}
                </Button>
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
                className="flex-1"
                style={{ userSelect: "none" }}
              />
              <Button onClick={startReplace}>{t("general.replace")}</Button>
              <Button onClick={removeKey} variant="danger">
                {t("general.remove")}
              </Button>
            </>
          )}
        </div>
        {keyError && (
          <div className="px-3 py-2 text-xs text-danger">{keyError}</div>
        )}
        {keySuccess && (
          <div className="px-3 py-2 text-xs text-success">
            {t("general.keySaveSuccess")}
          </div>
        )}
      </Group>
      {cleanWarning && (
        <div className="mt-[-16px] mb-6 px-1 text-xs text-warning animate-fade-in">
          {t("general.cleanNeedsKeyBefore")}
          <button
            type="button"
            className="underline hover:text-accent"
            onClick={openGroqKeysPage}
          >
            {t("general.cleanNeedsKeyLink")}
          </button>
          {t("general.cleanNeedsKeyAfter")}
        </div>
      )}
    </div>
  );
}
