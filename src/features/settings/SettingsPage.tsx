import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Preferences = { subscription_reminders_enabled: boolean; subscription_reminder_days: number };
type Subscription = { id: string; name: string; amount_minor: number; currency_code: string; frequency: string; next_due_date: string; status: string };
const money = (minor: number, currency: string) => new Intl.NumberFormat(undefined, { style: "currency", currency, maximumFractionDigits: 2 }).format(minor / 100);
const defaultPreferences: Preferences = { subscription_reminders_enabled: true, subscription_reminder_days: 7 };

export default function SettingsPage() {
  const [preferences, setPreferences] = useState(defaultPreferences);
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [saving, setSaving] = useState(false);

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
    try { await invoke("set_user_preferences", { preferences }); setMessage("Preferences saved."); }
    catch { setError("Preferences could not be saved."); }
    finally { setSaving(false); }
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
    <section aria-live="polite" className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Upcoming payment reminders</h2>{!preferences.subscription_reminders_enabled ? <p className="mt-3 text-sm text-slate-500">Subscription reminders are turned off.</p> : reminders.length === 0 ? <p className="mt-3 text-sm text-slate-500">No active subscriptions are due within {preferences.subscription_reminder_days} days.</p> : <ul className="mt-3 divide-y">{reminders.map((item) => <li key={item.id} className="flex flex-wrap justify-between gap-2 py-3 text-sm"><span><strong>{item.name}</strong><span className="ml-2 text-slate-500">{item.frequency}</span></span><span>{item.next_due_date} · {money(item.amount_minor, item.currency_code)}</span></li>)}</ul>}</section>
  </section>;
}
