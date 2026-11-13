import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { applyAppearancePreferences } from "./theme";
import { formatMoney as formatCurrency } from "../../shared/formatMoney";
import { Bell, FileText, HardDrive, Palette, ShieldCheck } from "lucide-react";

type Preferences = { subscription_reminders_enabled: boolean; subscription_reminder_days: number; theme: "system" | "light" | "dark"; grid_opacity: number; row_density: "compact" | "standard" | "comfortable" };
type Subscription = { id: string; name: string; amount_minor: number; currency_code: string; frequency: string; next_due_date: string; status: string };
type IntegrityReport = { integrity_messages: string[]; foreign_key_violations: string[] };
type DiagnosticReport = { app_version: string; operating_system: string; architecture: string; sqlite_version: string; schema_version: number; latest_supported_schema_version: number; database_integrity: "passed" | "issues_found"; foreign_key_check: "passed" | "issues_found"; diagnostic_retention_days: number; recent_events: { event_code: string; occurred_at: string }[] };
const money = (minor: number, currency: string) => formatCurrency(minor, currency, { maximumFractionDigits: 2 });
const defaultPreferences: Preferences = { subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 5, row_density: "standard" };
const settingsSections = [
  { label: "Appearance", target: "Appearance", icon: Palette },
  { label: "Reminders", target: "Subscription reminders", icon: Bell },
  { label: "Backup & Restore", target: "Encrypted backup", icon: HardDrive },
  { label: "Data Health", target: "Local data integrity", icon: ShieldCheck },
  { label: "Diagnostics", target: "Diagnostic report", icon: FileText },
];

function scrollToSettingsSection(target: string) {
  const heading = Array.from(document.querySelectorAll<HTMLHeadingElement>(".settings-content h2"))
    .find((item) => item.textContent?.trim().toLowerCase().includes(target.toLowerCase()));
  heading?.scrollIntoView?.({ behavior: "smooth", block: "start" });
}

export default function SettingsPage() {
  const [preferences, setPreferences] = useState(defaultPreferences);
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [saving, setSaving] = useState(false);
  const [integrity, setIntegrity] = useState<IntegrityReport | null>(null);
  const [checking, setChecking] = useState(false);
  const [backupPassphrase, setBackupPassphrase] = useState("");
  const [backupConfirmation, setBackupConfirmation] = useState("");
  const [restorePassphrase, setRestorePassphrase] = useState("");
  const [restoreFile, setRestoreFile] = useState<File | null>(null);
  const [restoreConfirmation, setRestoreConfirmation] = useState("");
  const [backupBusy, setBackupBusy] = useState(false);
  const [restoreBusy, setRestoreBusy] = useState(false);
  const [restoreComplete, setRestoreComplete] = useState(false);
  const [diagnosticsBusy, setDiagnosticsBusy] = useState(false);
  const [attachmentCleanupBusy, setAttachmentCleanupBusy] = useState(false);
  const [activeSection, setActiveSection] = useState("Reminders");

  useEffect(() => {
    Promise.all([
      invoke<Preferences>("get_user_preferences"),
      invoke<Subscription[]>("list_subscriptions"),
    ]).then(([saved, items]) => { setPreferences(saved); setSubscriptions(items); })
      .catch(() => setError("Settings could not be loaded."));
  }, []);

  const reminders = useMemo(() => {
    const today = new Date(); today.setHours(0, 0, 0, 0);
    const from = new Date(today); from.setDate(from.getDate() - 30);
    const until = new Date(today); until.setDate(until.getDate() + preferences.subscription_reminder_days);
    return subscriptions.filter((item) => { const due = new Date(`${item.next_due_date}T00:00:00`); return item.status === "active" && due >= from && due <= until; })
      .sort((a, b) => a.next_due_date.localeCompare(b.next_due_date));
  }, [preferences.subscription_reminder_days, subscriptions]);

  useEffect(() => {
    const media = typeof window.matchMedia === "function" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
    const updatePreview = () => {
      applyAppearancePreferences(preferences.theme, preferences.grid_opacity, preferences.row_density);
    };
    updatePreview();
    media?.addEventListener("change", updatePreview);
    return () => {
      media?.removeEventListener("change", updatePreview);
    };
  }, [preferences.grid_opacity, preferences.theme, preferences.row_density]);

  async function save() {
    setSaving(true); setError(""); setMessage("");
    try { await invoke("set_user_preferences", { preferences }); applyAppearancePreferences(preferences.theme, preferences.grid_opacity, preferences.row_density); setMessage("Preferences saved."); }
    catch { setError("Preferences could not be saved."); }
    finally { setSaving(false); }
  }

  async function checkIntegrity() {
    setChecking(true); setError(""); setIntegrity(null);
    try { setIntegrity(await invoke<IntegrityReport>("check_database_integrity")); }
    catch { setError("The local data check could not be completed."); }
    finally { setChecking(false); }
  }

  async function cleanupTemporaryAttachments() {
    setAttachmentCleanupBusy(true); setError(""); setMessage("");
    try {
      await invoke("cleanup_temporary_attachment_views");
      setMessage("Temporary decrypted document copies were removed.");
    } catch {
      setError("Temporary document copies could not all be removed. Close any documents open in another app and try again.");
    } finally {
      setAttachmentCleanupBusy(false);
    }
  }

  async function exportDiagnostics() {
    setDiagnosticsBusy(true); setError(""); setMessage("");
    try {
      const report = await invoke<DiagnosticReport>("get_diagnostic_report");
      const blob = new Blob([JSON.stringify({ generated_at: new Date().toISOString(), ...report }, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url; link.download = `ETracker_diagnostics_${new Date().toISOString().slice(0, 10)}.json`; link.click();
      URL.revokeObjectURL(url);
      setMessage("Support report saved. It includes app and database health plus redacted app events; it does not include financial records and is not sent automatically.");
    } catch { setError("A diagnostic report could not be created."); }
    finally { setDiagnosticsBusy(false); }
  }

  async function exportBackup() {
    setBackupBusy(true); setError(""); setMessage("");
    try {
      const encoded = await invoke<string>("export_encrypted_backup", { passphrase: backupPassphrase });
      const binary = atob(encoded);
      const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
      const filename = `ETracker_${new Date().toISOString().slice(0, 10)}.etracker-backup`;
      if (import.meta.env.MODE === "e2e") {
        const blob = new Blob([bytes.buffer as ArrayBuffer], { type: "application/octet-stream" });
        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");
        link.href = url; link.download = filename; link.click();
        URL.revokeObjectURL(url);
      } else {
        const path = await saveDialog({
          defaultPath: filename,
          filters: [{ name: "ETracker encrypted backup", extensions: ["etracker-backup"] }],
        });
        if (!path) {
          setMessage("Backup save cancelled; no file was written.");
          return;
        }
        await writeFile(path, bytes);
      }
      setMessage("Encrypted backup created. Keep your passphrase separately; it cannot be recovered.");
      setBackupPassphrase(""); setBackupConfirmation("");
    } catch { setError("Backup creation failed. Check the passphrase and available storage, then try again."); }
    finally { setBackupBusy(false); }
  }

  async function restoreBackup() {
    if (!restoreFile) return;
    setRestoreBusy(true); setError(""); setMessage("");
    try {
      const bytes = new Uint8Array(await restoreFile.arrayBuffer());
      let binary = "";
      for (let offset = 0; offset < bytes.length; offset += 0x8000) {
        binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
      }
      await invoke("restore_encrypted_backup", { backupBase64: btoa(binary), passphrase: restorePassphrase });
      setRestoreComplete(true);
      setMessage("Database restored. Reload ETracker to view the restored data.");
      setRestorePassphrase(""); setRestoreConfirmation("");
    } catch { setError("Restore failed. Check the passphrase and selected file. If replacement was interrupted, run the local data check before making further edits."); }
    finally { setRestoreBusy(false); }
  }

  return <section className="settings-screen space-y-5">
    <header className="flex flex-wrap items-end justify-between gap-4"><div><p className="workspace-accent font-mono text-xs font-semibold uppercase tracking-[0.1em]">System / Preferences · Local data</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Application settings</h1><p className="workspace-muted mt-2 text-sm">Appearance, in-app reminders, encrypted backups, and local data health.</p></div><button type="button" disabled={saving} onClick={save} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50">{saving ? "Saving…" : "Save changes"}</button></header>
    {error && <p role="alert" className="text-sm text-rose-700">{error}</p>}{message && <p role="status" className="text-sm text-emerald-800">{message}</p>}
    <div className="settings-layout grid gap-5 lg:grid-cols-[14rem_minmax(0,1fr)]">
      <nav aria-label="Settings sections" className="settings-index h-fit rounded-lg border border-slate-200 bg-white p-3 lg:sticky lg:top-4">
        <p className="px-2 py-2 font-mono text-[10px] font-semibold uppercase tracking-[0.1em] text-slate-500">Configuration</p>
        <div className="space-y-1">{settingsSections.map(({ label, target, icon: Icon }) => <button key={label} type="button" aria-current={activeSection === label ? "location" : undefined} onClick={() => { setActiveSection(label); scrollToSettingsSection(target); }} className={`settings-index-link flex min-h-10 w-full items-center gap-3 rounded-sm px-2.5 py-2 text-left text-sm font-medium text-slate-600 ${activeSection === label ? "settings-index-link-active" : ""}`}><Icon size={16} aria-hidden="true"/><span>{label}</span></button>)}</div>
        <div className="mt-4 border-t border-slate-200 px-2 pt-3"><p className="text-xs font-semibold">Local workspace</p><p className="workspace-muted mt-1 text-xs">Backup files are saved where you choose.</p></div>
      </nav>
      <div className="settings-content min-w-0 space-y-5">
    <section className="rounded-xl border border-slate-200 bg-white p-5">
      <h2 className="font-semibold">Subscription reminders</h2>
      <label className="mt-4 flex items-start gap-3 text-sm"><input type="checkbox" checked={preferences.subscription_reminders_enabled} onChange={(event) => setPreferences((current) => ({ ...current, subscription_reminders_enabled: event.target.checked }))} /><span><strong className="block">Show upcoming payment reminders</strong><span className="text-slate-600">Reminders appear here when you open the app. ETracker does not send background or operating-system notifications.</span></span></label>
      <label className="mt-4 block text-sm font-medium">Remind me within <span className="sr-only">days</span><input aria-label="Reminder window in days" type="number" min="1" max="30" value={preferences.subscription_reminder_days} onChange={(event) => setPreferences((current) => ({ ...current, subscription_reminder_days: Number(event.target.value) }))} className="ml-2 w-20 rounded-md border border-slate-300 px-2 py-1" /> days</label>
    </section>
    <section className="workspace-panel rounded-lg p-5">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div><h2 className="text-lg font-semibold">Appearance</h2><p className="workspace-muted mt-1 text-sm">Theme is saved with your preferences. Grid opacity and row density preview only for this session.</p></div>
        <span className="status-chip">Saved on this device</span>
      </div>
      <div className="grid gap-3 md:grid-cols-3" role="group" aria-label="Color theme">
        {([ ["light", "Light Ledger", "Bright canvas with accounting grid"], ["dark", "Dark Vault", "Charcoal surfaces with low-contrast grid"], ["system", "System Adaptive", "Follow the operating system theme"] ] as const).map(([theme, title, description]) => (
          <button key={theme} type="button" aria-pressed={preferences.theme === theme} onClick={() => setPreferences((current) => ({ ...current, theme }))} className={`theme-option rounded-lg border p-4 text-left ${preferences.theme === theme ? "theme-option-active" : ""}`}>
            <div className={`theme-swatch theme-swatch-${theme}`} aria-hidden="true"><span/><i/><b/></div>
            <div className="mt-3 flex items-center justify-between gap-3"><strong>{title}</strong><span className="theme-radio" aria-hidden="true">{preferences.theme === theme ? "●" : "○"}</span></div>
            <p className="workspace-muted mt-1 text-xs">{description}</p>
          </button>
        ))}
      </div>
      <label className="settings-control-row mt-4 flex flex-wrap items-center gap-3 rounded-md bg-slate-50 p-3 text-sm">
        <span>Accounting grid opacity</span>
        <input aria-label="Accounting grid opacity" type="range" min="0" max="12" value={preferences.grid_opacity} onChange={(event) => setPreferences((current) => ({ ...current, grid_opacity: Number(event.target.value) }))} className="min-w-36 flex-1" />
        <span className="font-mono tabular-nums">{preferences.grid_opacity}%</span>
        <span className="grid-preview" aria-hidden="true" style={{ backgroundSize: `${8 + preferences.grid_opacity}px ${8 + preferences.grid_opacity}px` }} />
      </label>
      <div className="mt-3 flex flex-wrap items-center justify-between gap-3 rounded-md bg-slate-50 p-3">
        <div><strong className="text-sm">UI row density</strong><p className="workspace-muted text-xs">Preview it here, then use Save changes to keep it for next time.</p></div>
        <div className="flex flex-wrap gap-1" role="group" aria-label="UI row density">
          {([ ["compact", 36, "Compact"], ["standard", 44, "Standard"], ["comfortable", 52, "Comfortable"] ] as const).map(([density, height, label]) => (
            <button key={density} type="button" aria-pressed={preferences.row_density === density} onClick={() => setPreferences((current) => ({ ...current, row_density: density }))} className={`rounded-sm px-3 py-2 text-xs ${preferences.row_density === density ? "bg-violet-700 text-white" : "border border-slate-200"}`}>{label} · {height}px</button>
          ))}
        </div>
      </div>
    </section>
    <section aria-live="polite" className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Upcoming payment reminders</h2>{!preferences.subscription_reminders_enabled ? <p className="mt-3 text-sm text-slate-500">Subscription reminders are turned off.</p> : reminders.length === 0 ? <p className="mt-3 text-sm text-slate-500">No active subscriptions are due within {preferences.subscription_reminder_days} days.</p> : <ul className="mt-3 divide-y">{reminders.map((item) => <li key={item.id} className="flex flex-wrap justify-between gap-2 py-3 text-sm"><span><strong>{item.name}</strong><span className="ml-2 text-slate-500">{item.frequency}</span></span><span>{item.next_due_date} · {money(item.amount_minor, item.currency_code)}</span></li>)}</ul>}</section>
    <section className="space-y-5 rounded-xl border border-slate-200 bg-white p-5"><div><h2 className="font-semibold">Encrypted backup and restore</h2><p className="mt-1 text-sm text-slate-600">Backups contain your financial records and retained statement files. Choose a strong passphrase and store it somewhere separate from the backup. ETracker does not save it and cannot recover a forgotten passphrase.</p></div>
      <div className="space-y-3 rounded-lg bg-slate-50 p-4"><h3 className="text-sm font-semibold">Create backup</h3><label className="block text-sm">Passphrase (at least 12 characters)<input aria-label="Backup passphrase" type="password" autoComplete="new-password" value={backupPassphrase} onChange={(event) => setBackupPassphrase(event.target.value)} className="mt-1 block w-full max-w-md rounded-md border border-slate-300 bg-white px-3 py-2" /></label><label className="block text-sm">Confirm passphrase<input aria-label="Confirm backup passphrase" type="password" autoComplete="new-password" value={backupConfirmation} onChange={(event) => setBackupConfirmation(event.target.value)} className="mt-1 block w-full max-w-md rounded-md border border-slate-300 bg-white px-3 py-2" /></label><button type="button" disabled={backupBusy || backupPassphrase.length < 12 || backupPassphrase !== backupConfirmation} onClick={exportBackup} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-medium text-white disabled:opacity-50">{backupBusy ? "Creating backup…" : import.meta.env.MODE === "e2e" ? "Download encrypted backup" : "Save encrypted backup"}</button></div>
      <div className="space-y-3 rounded-lg bg-slate-50 p-4"><h3 className="text-sm font-semibold">Restore backup</h3><p className="text-sm text-slate-600">Restoring replaces all current data with the selected backup. The backup is validated before the current database is changed.</p><label className="block text-sm">Encrypted backup file<input aria-label="Encrypted backup file" type="file" accept=".etracker-backup,application/octet-stream" onChange={(event) => { setRestoreFile(event.target.files?.[0] ?? null); setRestoreComplete(false); }} className="mt-1 block w-full max-w-md text-sm" /></label><label className="block text-sm">Backup passphrase<input aria-label="Restore passphrase" type="password" autoComplete="current-password" value={restorePassphrase} onChange={(event) => setRestorePassphrase(event.target.value)} className="mt-1 block w-full max-w-md rounded-md border border-slate-300 bg-white px-3 py-2" /></label><label className="block text-sm">Type RESTORE to confirm replacement<input aria-label="Restore confirmation" value={restoreConfirmation} onChange={(event) => setRestoreConfirmation(event.target.value)} className="mt-1 block w-full max-w-md rounded-md border border-slate-300 bg-white px-3 py-2" /></label><button type="button" disabled={restoreBusy || !restoreFile || restorePassphrase.length < 12 || restoreConfirmation !== "RESTORE"} onClick={restoreBackup} className="rounded-md bg-rose-700 px-4 py-2 text-sm font-medium text-white disabled:opacity-50">{restoreBusy ? "Validating and restoring…" : "Replace current data"}</button>{restoreComplete && <button type="button" onClick={() => window.location.reload()} className="ml-3 rounded-md border border-slate-300 px-4 py-2 text-sm font-medium">Reload ETracker</button>}</div>
    </section>
    <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Temporary document copies</h2><p className="mt-1 text-sm text-slate-600">Opening an encrypted attachment in another app creates a temporary decrypted copy on this device. ETracker removes these copies the next time it starts. Close any open documents before removing copies now.</p><button type="button" disabled={attachmentCleanupBusy} onClick={cleanupTemporaryAttachments} className="mt-3 rounded-md border border-slate-300 px-4 py-2 text-sm font-medium disabled:opacity-50">{attachmentCleanupBusy ? "Removing temporary copies…" : "Remove temporary copies now"}</button></section>
    <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Check your saved data</h2><p className="mt-1 text-sm text-slate-600">This quick check looks for problems reading your local records or links between them. It does not change or upload your data.</p><button type="button" disabled={checking} onClick={checkIntegrity} className="mt-3 rounded-md border border-slate-300 px-4 py-2 text-sm font-medium disabled:opacity-50">{checking ? "Checking your data…" : "Check saved data"}</button>{integrity && <div role="status" className="mt-3 text-sm">{integrity.integrity_messages.length === 1 && integrity.integrity_messages[0].toLowerCase() === "ok" && integrity.foreign_key_violations.length === 0 ? <p className="text-emerald-800">Your saved data passed the check. No problems were found.</p> : <div className="text-rose-700"><p>We found a problem with some saved data. Avoid editing affected records and contact support for help.</p><details className="mt-2"><summary className="cursor-pointer font-medium">View technical details</summary><div className="mt-2 space-y-1 break-all text-xs">{integrity.integrity_messages.map((item) => <p key={item}>{item}</p>)}{integrity.foreign_key_violations.map((item) => <p key={item}>{item}</p>)}</div></details></div>}</div>}</section>
    <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Share app information with support</h2><p className="mt-1 text-sm text-slate-600">Save a report with app details, a basic data check, and up to 100 recent app events from the last 30 days. Events use fixed labels only and do not contain transactions, balances, account names, file names, or statement contents. ETracker does not send the report; you decide whether to share the downloaded file.</p><button type="button" disabled={diagnosticsBusy} onClick={exportDiagnostics} className="mt-3 rounded-md border border-slate-300 px-4 py-2 text-sm font-medium disabled:opacity-50">{diagnosticsBusy ? "Preparing support report…" : "Save support report"}</button></section>
      </div>
    </div>
  </section>;
}
