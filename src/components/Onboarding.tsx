import { useCallback, useEffect, useState } from "react";
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
  type ModelDownloadFailed,
  type ModelDownloadProgress,
  type ModelStatus,
} from "@/bindings";
import { applySettings } from "@/stores/settings";

export type OnboardingStep = "microphone" | "accessibility" | "model";

const STEPS: OnboardingStep[] = ["microphone", "accessibility", "model"];
const MB = 1024 * 1024;

interface OnboardingProps {
  initialStep: OnboardingStep;
  onDone: () => void;
}

export function Onboarding({ initialStep, onDone }: OnboardingProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<OnboardingStep>(initialStep);
  const index = STEPS.indexOf(step);

  const next = () => setStep(STEPS[index + 1]);

  return (
    <div className="h-full flex items-center justify-center p-8">
      <div className="w-full max-w-md">
        <div className="text-xs text-muted mb-2">
          {t("onboarding.stepOf", { step: index + 1, total: STEPS.length })}
        </div>
        {step === "microphone" && (
          <PermissionStep
            title={t("onboarding.microphone.title")}
            body={t("onboarding.microphone.body")}
            action={t("onboarding.microphone.action")}
            check={checkMicrophonePermission}
            request={requestMicrophonePermission}
            onContinue={next}
          />
        )}
        {step === "accessibility" && (
          <PermissionStep
            title={t("onboarding.accessibility.title")}
            body={t("onboarding.accessibility.body")}
            action={t("onboarding.accessibility.action")}
            check={checkAccessibilityPermission}
            request={requestAccessibilityPermission}
            onContinue={next}
          />
        )}
        {step === "model" && <ModelStep onDone={onDone} />}
      </div>
    </div>
  );
}

interface PermissionStepProps {
  title: string;
  body: string;
  action: string;
  check: () => Promise<boolean>;
  request: () => Promise<unknown>;
  onContinue: () => void;
}

function PermissionStep({
  title,
  body,
  action,
  check,
  request,
  onContinue,
}: PermissionStepProps) {
  const { t } = useTranslation();
  const [granted, setGranted] = useState(false);

  // Poll: the grant happens in System Settings, outside this window.
  useEffect(() => {
    let active = true;
    const poll = async () => {
      const ok = await check().catch(() => false);
      if (active) setGranted(ok);
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, [check]);

  return (
    <div>
      <h1 className="text-xl font-semibold mb-2">{title}</h1>
      <p className="text-muted mb-6">{body}</p>
      {granted ? (
        <div className="flex items-center justify-between">
          <span className="text-sm">✓ {t("onboarding.granted")}</span>
          <PrimaryButton onClick={onContinue}>
            {t("onboarding.continue")}
          </PrimaryButton>
        </div>
      ) : (
        <PrimaryButton onClick={() => request()}>{action}</PrimaryButton>
      )}
    </div>
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

function ModelStep({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const [view, setView] = useState<DownloadView | null>(null);
  const [finishing, setFinishing] = useState(false);

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

  const start = () => {
    setView({ kind: "downloading", downloaded: 0, total: 0 });
    commands.startModelDownload();
  };

  const finish = async () => {
    setFinishing(true);
    const result = await commands.completeOnboarding();
    if (result.status === "ok") {
      await applySettings(Promise.resolve(result.data));
      onDone();
    } else {
      setFinishing(false);
      refresh();
    }
  };

  if (!view) return null;

  return (
    <div>
      <h1 className="text-xl font-semibold mb-2">
        {t("onboarding.model.title")}
      </h1>
      <p className="text-muted mb-6">
        {t("onboarding.model.body", { size: "1.5 GB" })}
      </p>

      {view.kind === "idle" && (
        <PrimaryButton onClick={start}>
          {view.downloaded > 0
            ? t("onboarding.model.resume")
            : t("onboarding.model.download")}
        </PrimaryButton>
      )}

      {view.kind === "downloading" && (
        <Progress downloaded={view.downloaded} total={view.total} />
      )}

      {view.kind === "verifying" && (
        <div className="text-sm">{t("onboarding.model.verifying")}</div>
      )}

      {view.kind === "failed" && (
        <div>
          <p className="text-sm text-danger mb-4">
            {t(`onboarding.model.errors.${view.reason}`, {
              defaultValue: t("onboarding.model.errors.io"),
            })}
          </p>
          <PrimaryButton onClick={start}>
            {t("onboarding.model.retry")}
          </PrimaryButton>
        </div>
      )}

      <div className="flex justify-end mt-8">
        <PrimaryButton
          onClick={finish}
          disabled={view.kind !== "ready" || finishing}
        >
          {t("onboarding.finish")}
        </PrimaryButton>
      </div>
    </div>
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
      <div className="h-2 rounded-full bg-control overflow-hidden mb-2">
        <div
          className="h-full bg-accent transition-[width]"
          style={{ width: `${percent}%` }}
        />
      </div>
      <div className="text-xs text-muted">
        {t("onboarding.model.progress", {
          downloaded: Math.round(downloaded / MB),
          total: Math.round(total / MB),
          percent,
        })}
      </div>
    </div>
  );
}

function PrimaryButton({
  onClick,
  disabled,
  children,
}: {
  onClick: () => void;
  disabled?: boolean;
  children: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="rounded-md bg-accent text-white px-4 py-1.5 text-sm font-medium disabled:opacity-40"
    >
      {children}
    </button>
  );
}
