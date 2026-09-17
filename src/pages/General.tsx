import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type ActivationMode } from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { Group, PageTitle, Row } from "@/components/ui/Row";
import { Segmented } from "@/components/ui/Segmented";
import { Select } from "@/components/ui/Select";
import { Toggle } from "@/components/ui/Toggle";
import { ShortcutInput } from "@/components/ShortcutInput";

const DEFAULT_MIC = "__default__";
const ALL_CHANNELS = "__all__";

export function General() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const [microphones, setMicrophones] = useState<string[]>([]);
  const [channelCount, setChannelCount] = useState(1);

  useEffect(() => {
    commands.getMicrophones().then((r) => {
      if (r.status === "ok") setMicrophones(r.data);
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

  return (
    <div>
      <PageTitle>{t("sidebar.general")}</PageTitle>

      <Group title={t("general.shortcutGroup")}>
        <Row label={t("general.transcribeShortcut")}>
          <ShortcutInput value={settings.shortcut} />
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
    </div>
  );
}
