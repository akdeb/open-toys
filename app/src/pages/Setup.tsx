import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CheckCircle2, Loader2 } from "lucide-react";
import logoPng from "../assets/logo.png";
import { SETUP_COPY } from "../constants";

interface SetupStatus {
  python_installed: boolean;
  python_version: string | null;
  python_path: string | null;
  venv_exists: boolean;
  venv_path: string | null;
  deps_installed: boolean;
}

type BootstrapStep =
  | "checking"
  | "downloading-python"
  | "creating-venv"
  | "installing-deps"
  | "complete";

export const SetupPage = () => {
  const navigate = useNavigate();
  const [step, setStep] = useState<BootstrapStep>("checking");
  const [, setStatus] = useState<SetupStatus | null>(null);
  const [progress, setProgress] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const activeStepLabel =
    step === "installing-deps"
      ? SETUP_COPY.activeStepTeaching
      : step === "downloading-python"
        ? SETUP_COPY.activeStepDownloadingPython
      : step === "creating-venv" || step === "checking"
        ? SETUP_COPY.activeStepPreparing
        : "Ready";
  const progressPercent =
    step === "complete"
      ? 100
      : step === "installing-deps"
        ? 75
        : step === "creating-venv"
          ? 50
          : step === "downloading-python"
            ? 25
            : 10;

  useEffect(() => {
    const unlisten = listen<string>("setup-progress", (event) => {
      setProgress(event.payload);
    });

    checkStatus();

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const checkStatus = async () => {
    try {
      setStep("checking");
      setError(null);
      const result = await invoke<SetupStatus>("check_setup_status");
      setStatus(result);

      if (result.deps_installed) {
        setStep("complete");
      } else if (result.venv_exists) {
        setStep("installing-deps");
        await installDeps();
      } else {
        await runFullSetup();
      }
    } catch (e: any) {
      setError(e?.message || String(e));
    }
  };

  const runFullSetup = async () => {
    try {
      setError(null);
      setStep("downloading-python");
      setProgress(`${SETUP_COPY.activeStepDownloadingPython}...`);
      await invoke("ensure_python_runtime");

      setStep("creating-venv");
      setProgress(`${SETUP_COPY.activeStepPreparing}...`);
      await invoke("create_python_venv");

      setStep("installing-deps");
      setProgress(`${SETUP_COPY.activeStepTeaching}...`);
      await invoke("install_python_deps");

      setStep("complete");
    } catch (e: any) {
      setError(e?.message || String(e));
    }
  };

  const installDeps = async () => {
    try {
      setError(null);
      setProgress(`${SETUP_COPY.activeStepTeaching}...`);
      await invoke("install_python_deps");

      setStep("complete");
    } catch (e: any) {
      setError(e?.message || String(e));
    }
  };

  const handleContinue = () => {
    navigate("/model-setup", { replace: true });
  };

  return (
    <div className="min-h-screen bg-(--color-retro-bg) flex items-center justify-center p-8">
      <div className="max-w-lg w-full">
        <div className="mb-8">
          <div className="flex items-center justify-center gap-2">
            <img src={logoPng} alt="" className="w-10 h-10" />
            <div className="text-4xl font-black tracking-wider brand-font mt-2">{SETUP_COPY.appName}</div>
            <div className="text-xs">{SETUP_COPY.appSuffix}</div>
            <div />
          </div>
            <div className="text-lg mt-2 text-center font-semibold text-gray-900">{SETUP_COPY.tagline}</div>


          <div className="mt-6 text-center">

            <div className="mt-3 text-gray-700 text-sm leading-relaxed">
              {SETUP_COPY.privacyBlurb}
              <br /><br />
              {SETUP_COPY.durationBlurb}
            </div>
          </div>
        </div>

          <div className="space-y-4">
            <div className="mb-4">
              <div className="h-4 w-full rounded-full bg-gray-100 overflow-hidden">
                <div
                  className="h-full bg-[#00c853] transition-all duration-300"
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
            </div>

              {error && (
                <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded-xl">
                  <div className="text-sm text-red-700 font-mono break-all">{error}</div>
                  <button className="retro-btn mt-3 w-full" onClick={checkStatus}>
                    Retry
                  </button>
                </div>
              )}

              {step === "complete" && (
                <div className="mt-2">
                  <button className="retro-btn w-full" onClick={handleContinue}>
                    Continue →
                  </button>
                </div>
              )}
          </div>

        <div className="mt-6 text-center text-xs text-gray-500 font-mono opacity-60">
          You can keep using your computer while this finishes.
        </div>
      </div>
      {!error && (
        <div className="fixed bottom-6 right-6 pointer-events-none">
          <div className="flex items-start gap-3 rounded-2xl border border-gray-200 bg-white/95 px-4 py-3 shadow-lg">
            {step === "complete" ? (
              <CheckCircle2 className="w-4 h-4 text-white mt-0.5" fill="black" />
            ) : (
              <Loader2 className="w-4 h-4 animate-spin text-gray-500 mt-0.5" />
            )}
            <div className="text-sm">
              <div className="font-semibold text-gray-900">
                {step === "complete" ? SETUP_COPY.toastComplete : activeStepLabel}
              </div>
              {progress && step !== "complete" && (
                <div className="text-xs text-gray-500 mt-1">{progress}</div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
