import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type AppLanguage } from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { Group, PageTitle, Row } from "@/components/ui/Row";
import { Toggle } from "@/components/ui/Toggle";
import { Segmented } from "@/components/ui/Segmented";
import { Button, TextField } from "@/components/ui/TextField";
import { InfoTip } from "@/components/ui/InfoTip";

// The only page with hover info.
function Label({ text, tip }: { text: string; tip: string }) {
  return (
    <>
      <span>{text}</span>
      <InfoTip text={tip} />
    </>
  );
}

export function Advanced() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
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
    commands.hasGroqApiKey().then((has) => {
      setHasKey(has);
      setEditing(!has);
    });
  }, []);

  if (!settings) return null;

  const keyErrorMessage = (code: string) => {
    switch (code) {
      case "empty_key":
        return t("advanced.keyEmpty");
      case "invalid_key":
        return t("advanced.keyInvalid");
      case "unreachable":
        return t("advanced.keyUnreachable");
      default:
        return t("advanced.keySaveFailed");
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
      setKeyError(t("advanced.keyRemoveFailed"));
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

  return (
    <div>
      <PageTitle>{t("sidebar.advanced")}</PageTitle>

      <Group title={t("advanced.appGroup")}>
        <Row
          label={
            <Label
              text={t("advanced.startHidden")}
              tip={t("advanced.startHiddenTip")}
            />
          }
        >
          <Toggle
            label={t("advanced.startHidden")}
            checked={settings.start_hidden}
            disabled={!settings.show_tray_icon}
            onChange={(enabled) =>
              applySettings(commands.setStartHidden(enabled))
            }
          />
        </Row>
        <Row
          label={
            <Label
              text={t("advanced.launchOnStartup")}
              tip={t("advanced.launchOnStartupTip")}
            />
          }
        >
          <Toggle
            label={t("advanced.launchOnStartup")}
            checked={settings.autostart_enabled}
            onChange={(enabled) => applySettings(commands.setAutostart(enabled))}
          />
        </Row>
        <Row
          label={
            <Label
              text={t("advanced.showMenuBarIcon")}
              tip={t("advanced.showMenuBarIconTip")}
            />
          }
        >
          <Toggle
            label={t("advanced.showMenuBarIcon")}
            checked={settings.show_tray_icon}
            onChange={(enabled) =>
              applySettings(commands.setShowTrayIcon(enabled))
            }
          />
        </Row>
        <Row
          label={
            <Label
              text={t("advanced.removeFillerWords")}
              tip={t("advanced.removeFillerWordsTip")}
            />
          }
        >
          <Toggle
            label={t("advanced.removeFillerWords")}
            checked={settings.remove_filler_words}
            onChange={(enabled) =>
              applySettings(commands.setRemoveFillerWords(enabled))
            }
          />
        </Row>
        <Row
          label={
            <Label
              text={t("advanced.interfaceLanguage")}
              tip={t("advanced.interfaceLanguageTip")}
            />
          }
        >
          <Segmented<AppLanguage>
            label={t("advanced.interfaceLanguage")}
            value={settings.app_language}
            options={[
              { value: "en", label: "English" },
              { value: "vi", label: "Tiếng Việt" },
            ]}
            onChange={(language) =>
              applySettings(commands.setAppLanguage(language))
            }
          />
        </Row>
      </Group>

      <Group title={t("advanced.cleanGroup")}>
        <Row
          label={
            <Label
              text={t("advanced.cleanAndReformat")}
              tip={t("advanced.cleanAndReformatTip")}
            />
          }
        >
          <Toggle
            label={t("advanced.cleanAndReformat")}
            checked={settings.clean_and_reformat}
            onChange={setCleanAndReformat}
          />
        </Row>
        {cleanWarning && (
          <div className="px-3 py-2 text-xs text-warning">
            {t("advanced.cleanNeedsKey")}
          </div>
        )}
        <Row
          label={
            <Label
              text={t("advanced.groqApiKey")}
              tip={t("advanced.groqApiKeyTip")}
            />
          }
        >
          <span className={`text-xs ${hasKey ? "" : "text-muted"}`}>
            {hasKey ? t("advanced.keySaved") : t("advanced.keyNotSet")}
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
                placeholder={t("advanced.keyPlaceholder")}
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
                {t("advanced.save")}
              </Button>
              {hasKey && (
                <Button onClick={removeKey} variant="danger">
                  {t("advanced.remove")}
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
                placeholder={t("advanced.keyRedacted")}
                aria-label={t("advanced.groqApiKey")}
                className="flex-1"
                style={{ userSelect: "none" }}
              />
              <Button onClick={startReplace}>{t("advanced.replace")}</Button>
              <Button onClick={removeKey} variant="danger">
                {t("advanced.remove")}
              </Button>
            </>
          )}
        </div>
        {keyError && (
          <div className="px-3 py-2 text-xs text-danger">{keyError}</div>
        )}
        {keySuccess && (
          <div className="px-3 py-2 text-xs text-success">
            {t("advanced.keySaveSuccess")}
          </div>
        )}
      </Group>
    </div>
  );
}
