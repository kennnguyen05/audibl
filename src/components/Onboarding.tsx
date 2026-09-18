import { useCallback, useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
  requestAccessibilityPermission,
  requestMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import {
  commands,
  type AppLanguage,
  type ModelDownloadFailed,
  type ModelDownloadProgress,
  type ModelStatus,
} from "@/bindings";
import { applySettings, useSettingsStore } from "@/stores/settings";
import { Button } from "@/components/ui/Button";
import { Segmented } from "@/components/ui/Segmented";
import {
  CheckIcon,
  DownloadIcon,
  GlobeIcon,
  KeyboardIcon,
  MicIcon,
} from "@/components/ui/icons";

export type OnboardingStep = "microphone" | "accessibility" | "model";

const STEPS: OnboardingStep[] = ["microphone", "accessibility", "model"];
const MB = 1024 * 1024;

// The outro after Finish: the cards fade out, the slogan fades in alone,
// holds long enough to read, and fades out before the main window arrives.
const FADE_OUT_MS = 500;
const SLOGAN_IN_MS = 700;
const SLOGAN_HOLD_MS = 1800;
const SLOGAN_OUT_MS = 600;

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

type Phase = "steps" | "leaving" | "slogan" | "closing";

interface OnboardingProps {
  initialStep: OnboardingStep;
  onDone: () => void;
}

/**
 * One screen, three stacked cards. The current step's card is open; finished
 * steps collapse to a title and a checkmark, later ones stay dimmed. A step
 * hands over as soon as it is satisfied — only the last one waits for Finish.
 */
export function Onboarding({ initialStep, onDone }: OnboardingProps) {
  const { t } = useTranslation();
  const appLanguage = useSettingsStore((s) => s.settings?.app_language);
  const [step, setStep] = useState<OnboardingStep>(initialStep);
  const [modelReady, setModelReady] = useState(false);
  const [phase, setPhase] = useState<Phase>("steps");
  const [sloganShown, setSloganShown] = useState(false);
  // Bumped when finishing fails, so the model step re-reads the file's status.
  const [modelKey, setModelKey] = useState(0);
  const current = STEPS.indexOf(step);

  // The slogan mounts transparent; flipping it on after the first paint is
  // what lets its fade-in transition run.
  useEffect(() => {
    if (phase !== "slogan") return;
    let id = requestAnimationFrame(() => {
      id = requestAnimationFrame(() => setSloganShown(true));
    });
    return () => cancelAnimationFrame(id);
  }, [phase]);

  // Completing onboarding flips the stored setting, and App swaps to the main
  // window as soon as it sees that, so the outro plays out first.
  const finish = async () => {
    setPhase("leaving");
    // The italic face loads on first use; fetch it during the fade-out so the
    // slogan doesn't swap fonts mid-fade.
    await Promise.all([
      wait(FADE_OUT_MS),
      document.fonts
        .load(`italic 56px "Newsreader Variable"`, t("onboarding.slogan"))
        .catch(() => {}),
    ]);
    setPhase("slogan");
    await wait(SLOGAN_IN_MS + SLOGAN_HOLD_MS);
    setPhase("closing");
    await wait(SLOGAN_OUT_MS);
    const result = await commands.completeOnboarding();
    if (result.status === "ok") {
      await applySettings(Promise.resolve(result.data));
      onDone();
    } else {
      setSloganShown(false);
      setModelKey((k) => k + 1);
      setPhase("steps");
    }
  };

  const advance = useCallback(() => {
    setStep((cur) => {
      const i = STEPS.indexOf(cur);
      return i < STEPS.length - 1 ? STEPS[i + 1] : cur;
    });
  }, []);

  const state = (of: OnboardingStep) => {
    const i = STEPS.indexOf(of);
    if (i < current) return "done" as const;
    if (i > current) return "upcoming" as const;
    return "current" as const;
  };

  return (
    <div className="flex h-full min-h-full flex-col items-center justify-center overflow-y-auto">
      {/* Nothing renders under the traffic lights, so the strip drags. */}
      <div
        data-tauri-drag-region
        className="fixed inset-x-0 top-0 h-11"
      />
      {(phase === "slogan" || phase === "closing") && (
        <p
          aria-live="polite"
          className={`px-9 text-center text-display italic text-text transition-[opacity,translate] ease-out ${
            phase === "closing" ? "duration-[600ms]" : "duration-[700ms]"
          } ${
            sloganShown && phase === "slogan"
              ? "translate-y-0 opacity-100"
              : "translate-y-2 opacity-0"
          }`}
        >
          {t("onboarding.slogan")}
        </p>
      )}

      {(phase === "steps" || phase === "leaving") && (
        <div
          className={`mx-auto w-full max-w-[560px] shrink-0 px-9 py-12 transition-opacity duration-500 ease-out ${
            phase === "leaving" ? "pointer-events-none opacity-0" : ""
          }`}
        >
          <h1 className="mb-7 text-3xl font-bold leading-[1.15] tracking-tight text-text">
            {t("onboarding.headline")}
          </h1>
  
          <div className="space-y-3">
            {/* The language choice comes first: everything below it is read in
                whichever language it sets. Always open, no gating step. */}
            {appLanguage && (
              <StepCard
                icon={<GlobeIcon size={18} />}
                title={t("onboarding.language.title")}
                state="done"
                hideCheck
                right={
                  <Segmented<AppLanguage>
                    label={t("onboarding.language.title")}
                    value={appLanguage}
                    options={[
                      { value: "en", label: "English" },
                      { value: "vi", label: "Tiếng Việt" },
                    ]}
                    onChange={(language) =>
                      applySettings(commands.setAppLanguage(language))
                    }
                  />
                }
              />
            )}
  
            <StepCard
              icon={<MicIcon size={18} />}
              title={t("onboarding.microphone.title")}
              state={state("microphone")}
            >
              <PermissionStep
                body={t("onboarding.microphone.body")}
                action={t("onboarding.microphone.action")}
                check={checkMicrophonePermission}
                request={requestMicrophonePermission}
                onGranted={advance}
              />
            </StepCard>
  
            <StepCard
              icon={<KeyboardIcon size={18} />}
              title={t("onboarding.accessibility.title")}
              state={state("accessibility")}
            >
              <PermissionStep
                body={t("onboarding.accessibility.body")}
                action={t("onboarding.accessibility.action")}
                check={checkAccessibilityPermission}
                request={requestAccessibilityPermission}
                onGranted={advance}
              />
            </StepCard>
  
            <StepCard
              icon={<DownloadIcon size={18} />}
              title={t("onboarding.model.title")}
              state={modelReady ? "done" : state("model")}
              open={step === "model"}
            >
              <ModelStep
                key={modelKey}
                finishing={phase !== "steps"}
                onFinish={finish}
                onReady={setModelReady}
              />
            </StepCard>
          </div>
        </div>
      )}
    </div>
  );
}

interface StepCardProps {
  icon: ReactNode;
  title: string;
  state: "done" | "current" | "upcoming";
  /** Overrides the collapse rule: the last card stays open once it is done. */
  open?: boolean;
  /** Suppresses the checkmark on a "done" card that isn't a granted permission. */
  hideCheck?: boolean;
  /** Trailing control in the header row, e.g. the language card's Segmented. */
  right?: ReactNode;
  children?: ReactNode;
}

function StepCard({
  icon,
  title,
  state,
  open,
  hideCheck,
  right,
  children,
}: StepCardProps) {
  const { t } = useTranslation();
  const expanded = open ?? state === "current";
  return (
    <section
      className={`rounded-card border border-border bg-surface px-5 py-4 ${
        state === "upcoming" ? "opacity-40" : ""
      }`}
    >
      <div className="flex items-center gap-3">
        <span className="shrink-0 text-muted">{icon}</span>
        <h2 className="min-w-0 flex-1 text-base font-semibold text-text">
          {title}
        </h2>
        {state === "done" && !hideCheck && (
          <span
            role="img"
            aria-label={t("onboarding.granted")}
            title={t("onboarding.granted")}
            className="shrink-0 text-success"
          >
            <CheckIcon size={18} />
          </span>
        )}
        {right && <span className="shrink-0">{right}</span>}
      </div>
      {expanded && children && (
        <div className="mt-2.5 ps-[30px]">{children}</div>
      )}
    </section>
  );
}

interface PermissionStepProps {
  body: string;
  action: string;
  check: () => Promise<boolean>;
  request: () => Promise<unknown>;
  onGranted: () => void;
}

function PermissionStep({
  body,
  action,
  check,
  request,
  onGranted,
}: PermissionStepProps) {
  // Poll: the grant happens in System Settings, outside this window, and it
  // moves the flow on by itself.
  useEffect(() => {
    let active = true;
    const poll = async () => {
      const ok = await check().catch(() => false);
      if (active && ok) onGranted();
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, [check, onGranted]);

  return (
    <>
      <p className="text-sm leading-relaxed text-muted">{body}</p>
      <div className="mt-3.5">
        <Button variant="primary" onClick={() => request()}>
          {action}
        </Button>
      </div>
    </>
  );
}

type DownloadView =
  | { kind: "idle"; downloaded: number }
  | { kind: "downloading"; downloaded: number; total: number }
  | { kind: "verifying" }
  | { kind: "failed"; reason: string }
  | { kind: "ready" };

function viewFromStatus(status: ModelStatus): DownloadView {
  switch (status.state) {
    case "ready":
      return { kind: "ready" };
    case "verifying":
      return { kind: "verifying" };
    case "downloading":
      return {
        kind: "downloading",
        downloaded: status.downloaded,
        total: status.total,
      };
    case "partial":
      return { kind: "idle", downloaded: status.downloaded };
    default:
      return { kind: "idle", downloaded: 0 };
  }
}

function ModelStep({
  finishing,
  onFinish,
  onReady,
}: {
  finishing: boolean;
  onFinish: () => void;
  onReady: (ready: boolean) => void;
}) {
  const { t } = useTranslation();
  const [view, setView] = useState<DownloadView | null>(null);

  const refresh = useCallback(async () => {
    setView(viewFromStatus(await commands.getModelStatus()));
  }, []);

  useEffect(() => {
    refresh();
    const unlisteners = [
      listen<ModelDownloadProgress>("model-download-progress", (e) => {
        const { downloaded, total, verifying } = e.payload;
        setView(
          verifying
            ? { kind: "verifying" }
            : { kind: "downloading", downloaded, total },
        );
      }),
      listen<ModelDownloadFailed>("model-download-failed", (e) =>
        setView({ kind: "failed", reason: e.payload.reason }),
      ),
      listen("model-download-complete", () => setView({ kind: "ready" })),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
  }, [refresh]);

  // The card's checkmark follows the file, not the Finish button.
  useEffect(() => {
    onReady(view?.kind === "ready");
  }, [view?.kind, onReady]);

  const start = () => {
    setView({ kind: "downloading", downloaded: 0, total: 0 });
    commands.startModelDownload();
  };

  if (!view) return null;

  return (
    <>
      {/* Once the model is on disk, the download pitch is stale — the ready
          line below says everything that is still true. */}
      {view.kind !== "ready" && (
        <p className="text-sm leading-relaxed text-muted">
          {t("onboarding.model.body", { size: "1.5 GB" })}
        </p>
      )}

      <div className="mt-3.5">
        {view.kind === "idle" && (
          <Button variant="primary" onClick={start}>
            {view.downloaded > 0
              ? t("onboarding.model.resume")
              : t("onboarding.model.download")}
          </Button>
        )}

        {view.kind === "downloading" && (
          <Progress downloaded={view.downloaded} total={view.total} />
        )}

        {view.kind === "verifying" && (
          <p className="text-sm text-muted">{t("onboarding.model.verifying")}</p>
        )}

        {view.kind === "ready" && (
          <p className="text-sm text-muted">{t("onboarding.model.ready")}</p>
        )}

        {view.kind === "failed" && (
          <>
            <p className="mb-3.5 text-sm leading-relaxed text-danger">
              {t(`onboarding.model.errors.${view.reason}`, {
                defaultValue: t("onboarding.model.errors.io"),
              })}
            </p>
            <Button variant="primary" onClick={start}>
              {t("onboarding.model.retry")}
            </Button>
          </>
        )}
      </div>

      <div className="mt-6 flex items-center justify-end gap-3">
        {/* The model is required, so the only way out of this step is to quit. */}
        <Button variant="danger" onClick={() => commands.quitApp()}>
          {t("onboarding.model.quit")}
        </Button>
        <Button
          variant="primary"
          onClick={onFinish}
          disabled={view.kind !== "ready" || finishing}
        >
          {t("onboarding.finish")}
        </Button>
      </div>
    </>
  );
}

function Progress({
  downloaded,
  total,
}: {
  downloaded: number;
  total: number;
}) {
  const { t } = useTranslation();
  const percent = total > 0 ? Math.floor((downloaded / total) * 100) : 0;
  return (
    <div>
      <div className="h-1.5 overflow-hidden rounded-full bg-control">
        <div
          className="h-full rounded-full bg-accent transition-[width]"
          style={{ width: `${percent}%` }}
        />
      </div>
      <div className="mt-2 text-xs text-muted">
        {t("onboarding.model.progress", {
          downloaded: Math.round(downloaded / MB),
          total: Math.round(total / MB),
          percent,
        })}
      </div>
    </div>
  );
}
