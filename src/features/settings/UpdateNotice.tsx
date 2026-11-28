import { useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { Download, X } from "lucide-react";

export default function UpdateNotice() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [installing, setInstalling] = useState(false);
  const [installError, setInstallError] = useState(false);
  const updateRef = useRef<Update | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    let disposed = false;
    void check({ timeout: 10_000 })
      .then(async (candidate) => {
        if (disposed) {
          await Promise.resolve(candidate?.close()).catch(() => {});
          return;
        }
        updateRef.current = candidate;
        setUpdate(candidate);
      })
      .catch(() => {
        // Update checks are best effort. Offline use and failed checks stay quiet.
      });

    return () => {
      disposed = true;
      void Promise.resolve(updateRef.current?.close()).catch(() => {});
    };
  }, []);

  if (!update) return null;

  const dismiss = () => {
    setUpdate(null);
    const current = updateRef.current;
    updateRef.current = null;
    void Promise.resolve(current?.close()).catch(() => {});
  };

  const install = async () => {
    setInstalling(true);
    setInstallError(false);
    try {
      await update.downloadAndInstall();
    } catch {
      setInstallError(true);
      setInstalling(false);
    }
  };

  return (
    <section aria-label="Application update" className="mx-3 mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-md border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-950 sm:mx-5 lg:mx-6">
      <Download size={17} aria-hidden="true" className="shrink-0" />
      <p className="min-w-0 flex-1">
        <span className="font-semibold">ETracker {update.version} is ready.</span>{" "}
        {installing ? "Downloading and installing…" : installError ? "The update could not be installed. Check your connection and try again." : "Install it now or keep using this version."}
      </p>
      {!installing && <button type="button" onClick={() => void install()} className="rounded-sm bg-emerald-800 px-3 py-2 text-xs font-semibold text-white hover:bg-emerald-900">Install and restart</button>}
      {!installing && <button type="button" onClick={dismiss} aria-label="Remind me later" title="Remind me later" className="rounded-sm p-2 text-emerald-900 hover:bg-emerald-100"><X size={16} aria-hidden="true" /></button>}
      {installing && <span role="status" className="text-xs font-medium">Please keep ETracker open until installation starts.</span>}
    </section>
  );
}
