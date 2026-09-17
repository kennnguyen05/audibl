import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { commands, type AppSettings } from "@/bindings";

interface SettingsState {
  settings: AppSettings | null;
}

// Zustand mirror of the Rust `AppSettings`. Rust is the source of truth:
// every setter command returns the updated settings, and changes made
// elsewhere (tray, other webview) arrive through `settings-changed`.
export const useSettingsStore = create<SettingsState>(() => ({
  settings: null,
}));

let initialized = false;

export const initSettingsStore = async () => {
  if (initialized) return;
  initialized = true;
  useSettingsStore.setState({ settings: await commands.getAppSettings() });
  await listen<AppSettings>("settings-changed", (event) => {
    useSettingsStore.setState({ settings: event.payload });
  });
};

/** Runs a setter command and stores the settings it returns. */
export const applySettings = async (update: Promise<AppSettings>) => {
  const settings = await update;
  useSettingsStore.setState({ settings });
  return settings;
};
