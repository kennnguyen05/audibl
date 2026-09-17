import { useTranslation } from "react-i18next";
import { commands, type ActivationMode } from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { Group, PageTitle, Row } from "@/components/ui/Row";
import { Segmented } from "@/components/ui/Segmented";
import { Toggle } from "@/components/ui/Toggle";

export function General() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  if (!settings) return null;

  const isToggle = settings.activation_mode === "toggle";

  return (
    <div>
      <PageTitle>{t("sidebar.general")}</PageTitle>

      <Group title={t("general.shortcutGroup")}>
        <Row label={t("general.transcribeShortcut")}>
          <kbd className="rounded border border-border bg-bg px-2 py-0.5 text-xs font-mono">
            {settings.shortcut}
          </kbd>
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
    </div>
  );
}
