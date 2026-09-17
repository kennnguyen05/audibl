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
  const [keyInput, setKeyInput] = useState("");
  const [keyError, setKeyError] = useState<string | null>(null);

  useEffect(() => {
    commands.hasGroqApiKey().then(setHasKey);
  }, []);

  if (!settings) return null;

  const saveKey = async () => {
    setKeyError(null);
    const result = await commands.setGroqApiKey(keyInput);
    if (result.status === "ok") {
      setKeyInput("");
      setHasKey(true);
    } else {
      setKeyError(t("advanced.keySaveFailed"));
    }
  };

  const removeKey = async () => {
    setKeyError(null);
    const result = await commands.clearGroqApiKey();
    if (result.status === "ok") {
      applySettings(Promise.resolve(result.data));
      setHasKey(false);
    } else {
      setKeyError(t("advanced.keyRemoveFailed"));
    }
  };

  const setCleanAndReformat = async (enabled: boolean) => {
    const result = await commands.setCleanAndReformat(enabled);
    if (result.status === "ok") applySettings(Promise.resolve(result.data));
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
            disabled={!hasKey}
            onChange={setCleanAndReformat}
          />
        </Row>
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
          <TextField
            type="password"
            autoComplete="off"
            spellCheck={false}
            className="flex-1"
            value={keyInput}
            placeholder={t("advanced.keyPlaceholder")}
            onChange={(e) => setKeyInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && keyInput.trim() && saveKey()}
          />
          <Button onClick={saveKey} disabled={!keyInput.trim()}>
            {t("advanced.save")}
          </Button>
          <Button onClick={removeKey} disabled={!hasKey} variant="danger">
            {t("advanced.remove")}
          </Button>
        </div>
        {keyError && (
          <div className="px-3 py-2 text-xs text-danger">{keyError}</div>
        )}
      </Group>
    </div>
  );
}
