import { useEffect, useState } from "react";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { commands } from "@/bindings";
import { Sidebar, type Page } from "./components/Sidebar";
import { Onboarding, type OnboardingStep } from "./components/Onboarding";
import { General } from "./pages/General";
import { Dictionary } from "./pages/Dictionary";
import { History } from "./pages/History";
import { Advanced } from "./pages/Advanced";
import { initSettingsStore, useSettingsStore } from "./stores/settings";

const PAGES: Record<Page, () => JSX.Element | null> = {
  general: General,
  dictionary: Dictionary,
  history: History,
  advanced: Advanced,
};

/**
 * New users start at the first step. Returning users only see onboarding
 * when a permission was revoked or the model file is missing, starting at
 * the first missing piece.
 */
async function firstMissingStep(
  onboardingComplete: boolean,
): Promise<OnboardingStep | null> {
  if (!onboardingComplete) return "microphone";
  const [mic, ax, model] = await Promise.all([
    checkMicrophonePermission().catch(() => false),
    checkAccessibilityPermission().catch(() => false),
    commands.getModelStatus(),
  ]);
  if (!mic) return "microphone";
  if (!ax) return "accessibility";
  if (model.state !== "ready") return "model";
  return null;
}

function App() {
  const [page, setPage] = useState<Page>("general");
  const settings = useSettingsStore((s) => s.settings);
  const [onboarding, setOnboarding] = useState<OnboardingStep | null | undefined>(
    undefined,
  );

  useEffect(() => {
    initSettingsStore();
  }, []);

  const onboardingComplete = settings?.onboarding_complete;
  useEffect(() => {
    if (onboardingComplete === undefined) return;
    firstMissingStep(onboardingComplete).then((step) => {
      setOnboarding(step);
      if (!step) setPage("general");
    });
  }, [onboardingComplete]);

  // The backend opens the window on the download screen when the shortcut
  // is pressed without a model.
  useEffect(() => {
    const onFocus = () => {
      if (onboardingComplete) {
        firstMissingStep(true).then(setOnboarding);
      }
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [onboardingComplete]);

  if (!settings || onboarding === undefined) return null;

  if (onboarding) {
    return (
      <Onboarding
        key={onboarding}
        initialStep={onboarding}
        onDone={() => firstMissingStep(true).then(setOnboarding)}
      />
    );
  }

  const ActivePage = PAGES[page];
  return (
    <div className="h-full flex">
      <Sidebar active={page} onSelect={setPage} />
      <main className="flex-1 overflow-y-auto px-6 py-5">
        <ActivePage />
      </main>
    </div>
  );
}

export default App;
