import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands, type AppLanguage, type AppSettings } from "@/bindings";

// Auto-discover locale files (en, vi).
const localeModules = import.meta.glob<{ default: Record<string, unknown> }>(
  "./locales/*/translation.json",
  { eager: true },
);

const resources: Record<string, { translation: Record<string, unknown> }> = {};
for (const [path, module] of Object.entries(localeModules)) {
  const code = path.match(/\.\/locales\/(.+)\/translation\.json/)?.[1];
  if (code) resources[code] = { translation: module.default };
}

i18n.use(initReactI18next).init({
  resources,
  lng: "en",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
  react: { useSuspense: false },
});

export const applyLanguage = async (language: AppLanguage) => {
  document.documentElement.lang = language;
  if (i18n.language !== language) await i18n.changeLanguage(language);
};

// The backend picks the first-launch default from the macOS locale, so the
// stored setting is always the source of truth. Both webviews (main window
// and overlay) follow changes live through the settings-changed event.
export const initLanguage = async () => {
  try {
    const settings = await commands.getAppSettings();
    await applyLanguage(settings.app_language);
  } catch (e) {
    console.warn("Failed to load interface language:", e);
  }
  await listen<AppSettings>("settings-changed", (event) => {
    applyLanguage(event.payload.app_language);
  });
};

export default i18n;
