import { useEffect, useRef, useState } from "react";
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

  // Set while onboarding is on screen, so the main window fades in after its
  // outro but appears at once on an ordinary launch.
  const cameFromOnboarding = useRef(false);

  if (!settings || onboarding === undefined) return null;

  if (onboarding) {
    cameFromOnboarding.current = true;
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
    <div
      className={`flex h-full ${cameFromOnboarding.current ? "animate-enter" : ""}`}
    >
      <Sidebar active={page} onSelect={setPage} />
      {/* The rail never scrolls; the content column is the one scroll region.
          pt-11 clears the traffic lights, which float over the window. */}
      <main className="relative flex-1 overflow-y-auto px-9 pb-10 pt-11">
        {/* Nothing renders under the traffic-light strip, so it is free to
            drag the window. */}
        <div
          data-tauri-drag-region
          className="absolute inset-x-0 top-0 h-11"
        />
        <div className="mx-auto max-w-[620px]">
          <ActivePage />
        </div>
      </main>
    </div>
  );
}

export default App;
