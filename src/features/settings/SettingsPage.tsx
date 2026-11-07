import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { applyTheme } from "./theme";

type Preferences = { subscription_reminders_enabled: boolean; subscription_reminder_days: number; theme: "system" | "light" | "dark" };
type Subscription = { id: string; name: string; amount_minor: number; currency_code: string; frequency: string; next_due_date: string; status: string };
type IntegrityReport = { integrity_messages: string[]; foreign_key_violations: string[] };
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
    <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Local data integrity</h2><p className="mt-1 text-sm text-slate-600">Check SQLite structure and references between saved records.</p><button type="button" disabled={checking} onClick={checkIntegrity} className="mt-3 rounded-md border border-slate-300 px-4 py-2 text-sm font-medium disabled:opacity-50">{checking ? "Checking…" : "Check local data"}</button>{integrity && <div role="status" className="mt-3 text-sm">{integrity.integrity_messages.length === 1 && integrity.integrity_messages[0].toLowerCase() === "ok" && integrity.foreign_key_violations.length === 0 ? <p className="text-emerald-800">The local database passed both checks.</p> : <div className="text-rose-700"><p>Database check found issues. Avoid editing affected records; this report may help diagnose the problem.</p>{integrity.integrity_messages.map((item) => <p key={item}>{item}</p>)}{integrity.foreign_key_violations.map((item) => <p key={item}>{item}</p>)}</div>}</div>}</section>
  </section>;
}
