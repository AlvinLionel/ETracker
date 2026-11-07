import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { applyTheme } from "./theme";

type Preferences = { subscription_reminders_enabled: boolean; subscription_reminder_days: number; theme: "system" | "light" | "dark" };
type Subscription = { id: string; name: string; amount_minor: number; currency_code: string; frequency: string; next_due_date: string; status: string };
type IntegrityReport = { integrity_messages: string[]; foreign_key_violations: string[] };
type DiagnosticReport = { app_version: string; operating_system: string; architecture: string; sqlite_version: string; schema_version: number; latest_supported_schema_version: number; database_integrity: "passed" | "issues_found"; foreign_key_check: "passed" | "issues_found" };
const money = (minor: number, currency: string) => new Intl.NumberFormat(undefined, { style: "currency", currency, maximumFractionDigits: 2 }).format(minor / 100);
const defaultPreferences: Preferences = { subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system" };

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

  async function save() {
    setSaving(true); setError(""); setMessage("");
    try { await invoke("set_user_preferences", { preferences }); applyTheme(preferences.theme); setMessage("Preferences saved."); }
    catch { setError("Preferences could not be saved."); }
    finally { setSaving(false); }
  }

  async function checkIntegrity() {
    setChecking(true); setError(""); setIntegrity(null);
    try { setIntegrity(await invoke<IntegrityReport>("check_database_integrity")); }
    catch { setError("The local data check could not be completed."); }
    finally { setChecking(false); }
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
      setMessage("Privacy-safe diagnostic report downloaded. It contains software and database health details, not financial records.");
    } catch { setError("A diagnostic report could not be created."); }
    finally { setDiagnosticsBusy(false); }
  }

  async function exportBackup() {
    setBackupBusy(true); setError(""); setMessage("");
    try {
      const encoded = await invoke<string>("export_encrypted_backup", { passphrase: backupPassphrase });
      const binary = atob(encoded);
      const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
      const blob = new Blob([bytes.buffer as ArrayBuffer], { type: "application/octet-stream" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url; link.download = `ETracker_${new Date().toISOString().slice(0, 10)}.etracker-backup`; link.click();
      URL.revokeObjectURL(url);
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

  return <section className="mx-auto max-w-4xl space-y-6">
    <header><p className="text-sm font-medium text-emerald-800">System</p><h1 className="mt-1 text-2xl font-bold">Settings and reminders</h1><p className="mt-1 text-sm text-slate-600">Manage local reminder preferences for subscription payments.</p></header>
    {error && <p role="alert" className="text-sm text-rose-700">{error}</p>}{message && <p role="status" className="text-sm text-emerald-800">{message}</p>}
    <section className="rounded-xl border border-slate-200 bg-white p-5">
      <h2 className="font-semibold">Subscription reminders</h2>
      <label className="mt-4 flex items-start gap-3 text-sm"><input type="checkbox" checked={preferences.subscription_reminders_enabled} onChange={(event) => setPreferences((current) => ({ ...current, subscription_reminders_enabled: event.target.checked }))} /><span><strong className="block">Show upcoming payment reminders</strong><span className="text-slate-600">Reminders appear here when you open the app. ETracker does not send background or operating-system notifications.</span></span></label>
      <label className="mt-4 block text-sm font-medium">Remind me within <span className="sr-only">days</span><input aria-label="Reminder window in days" type="number" min="1" max="30" value={preferences.subscription_reminder_days} onChange={(event) => setPreferences((current) => ({ ...current, subscription_reminder_days: Number(event.target.value) }))} className="ml-2 w-20 rounded-md border border-slate-300 px-2 py-1" /> days</label>
      <button type="button" disabled={saving || preferences.subscription_reminder_days < 1 || preferences.subscription_reminder_days > 30} onClick={save} className="mt-4 rounded-md bg-emerald-700 px-4 py-2 text-sm font-medium text-white disabled:opacity-50">{saving ? "Saving…" : "Save preferences"}</button>
    </section>
    <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Appearance</h2><label className="mt-3 block text-sm font-medium">Color theme<select aria-label="Color theme" value={preferences.theme} onChange={(event) => setPreferences((current) => ({ ...current, theme: event.target.value as Preferences["theme"] }))} className="mt-1 block w-full max-w-xs rounded-md border border-slate-300 bg-white px-3 py-2"><option value="system">Use device setting</option><option value="light">Light</option><option value="dark">Dark</option></select></label><p className="mt-2 text-xs text-slate-500">Theme changes apply after saving.</p></section>
    <section aria-live="polite" className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Upcoming payment reminders</h2>{!preferences.subscription_reminders_enabled ? <p className="mt-3 text-sm text-slate-500">Subscription reminders are turned off.</p> : reminders.length === 0 ? <p className="mt-3 text-sm text-slate-500">No active subscriptions are due within {preferences.subscription_reminder_days} days.</p> : <ul className="mt-3 divide-y">{reminders.map((item) => <li key={item.id} className="flex flex-wrap justify-between gap-2 py-3 text-sm"><span><strong>{item.name}</strong><span className="ml-2 text-slate-500">{item.frequency}</span></span><span>{item.next_due_date} · {money(item.amount_minor, item.currency_code)}</span></li>)}</ul>}</section>
    <section className="space-y-5 rounded-xl border border-slate-200 bg-white p-5"><div><h2 className="font-semibold">Encrypted backup and restore</h2><p className="mt-1 text-sm text-slate-600">Backups contain your financial records and retained statement files. Choose a strong passphrase and store it somewhere separate from the backup. ETracker does not save it and cannot recover a forgotten passphrase.</p></div>
      <div className="space-y-3 rounded-lg bg-slate-50 p-4"><h3 className="text-sm font-semibold">Create backup</h3><label className="block text-sm">Passphrase (at least 12 characters)<input aria-label="Backup passphrase" type="password" autoComplete="new-password" value={backupPassphrase} onChange={(event) => setBackupPassphrase(event.target.value)} className="mt-1 block w-full max-w-md rounded-md border border-slate-300 bg-white px-3 py-2" /></label><label className="block text-sm">Confirm passphrase<input aria-label="Confirm backup passphrase" type="password" autoComplete="new-password" value={backupConfirmation} onChange={(event) => setBackupConfirmation(event.target.value)} className="mt-1 block w-full max-w-md rounded-md border border-slate-300 bg-white px-3 py-2" /></label><button type="button" disabled={backupBusy || backupPassphrase.length < 12 || backupPassphrase !== backupConfirmation} onClick={exportBackup} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-medium text-white disabled:opacity-50">{backupBusy ? "Creating backup…" : "Download encrypted backup"}</button></div>
      <div className="space-y-3 rounded-lg bg-slate-50 p-4"><h3 className="text-sm font-semibold">Restore backup</h3><p className="text-sm text-slate-600">Restoring replaces all current data with the selected backup. The backup is validated before the current database is changed.</p><label className="block text-sm">Encrypted backup file<input aria-label="Encrypted backup file" type="file" accept=".etracker-backup,application/octet-stream" onChange={(event) => { setRestoreFile(event.target.files?.[0] ?? null); setRestoreComplete(false); }} className="mt-1 block w-full max-w-md text-sm" /></label><label className="block text-sm">Backup passphrase<input aria-label="Restore passphrase" type="password" autoComplete="current-password" value={restorePassphrase} onChange={(event) => setRestorePassphrase(event.target.value)} className="mt-1 block w-full max-w-md rounded-md border border-slate-300 bg-white px-3 py-2" /></label><label className="block text-sm">Type RESTORE to confirm replacement<input aria-label="Restore confirmation" value={restoreConfirmation} onChange={(event) => setRestoreConfirmation(event.target.value)} className="mt-1 block w-full max-w-md rounded-md border border-slate-300 bg-white px-3 py-2" /></label><button type="button" disabled={restoreBusy || !restoreFile || restorePassphrase.length < 12 || restoreConfirmation !== "RESTORE"} onClick={restoreBackup} className="rounded-md bg-rose-700 px-4 py-2 text-sm font-medium text-white disabled:opacity-50">{restoreBusy ? "Validating and restoring…" : "Replace current data"}</button>{restoreComplete && <button type="button" onClick={() => window.location.reload()} className="ml-3 rounded-md border border-slate-300 px-4 py-2 text-sm font-medium">Reload ETracker</button>}</div>
    </section>
    <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Local data integrity</h2><p className="mt-1 text-sm text-slate-600">Check SQLite structure and references between saved records.</p><button type="button" disabled={checking} onClick={checkIntegrity} className="mt-3 rounded-md border border-slate-300 px-4 py-2 text-sm font-medium disabled:opacity-50">{checking ? "Checking…" : "Check local data"}</button>{integrity && <div role="status" className="mt-3 text-sm">{integrity.integrity_messages.length === 1 && integrity.integrity_messages[0].toLowerCase() === "ok" && integrity.foreign_key_violations.length === 0 ? <p className="text-emerald-800">The local database passed both checks.</p> : <div className="text-rose-700"><p>Database check found issues. Avoid editing affected records; this report may help diagnose the problem.</p>{integrity.integrity_messages.map((item) => <p key={item}>{item}</p>)}{integrity.foreign_key_violations.map((item) => <p key={item}>{item}</p>)}</div>}</div>}</section>
    <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Diagnostic report</h2><p className="mt-1 text-sm text-slate-600">Download app version, platform, SQLite/schema versions, and redacted database health status. The report excludes financial records and is not sent anywhere.</p><button type="button" disabled={diagnosticsBusy} onClick={exportDiagnostics} className="mt-3 rounded-md border border-slate-300 px-4 py-2 text-sm font-medium disabled:opacity-50">{diagnosticsBusy ? "Preparing report…" : "Download diagnostic report"}</button></section>
  </section>;
}
