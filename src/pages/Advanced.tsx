import { useTranslation } from "react-i18next";
import { commands, type AppLanguage } from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { Group, PageTitle, Row } from "@/components/ui/Row";
import { Toggle } from "@/components/ui/Toggle";
import { Segmented } from "@/components/ui/Segmented";
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

  if (!settings) return null;

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
    </div>
  );
}
