import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listActiveAccounts } from "../accounts/api/accounts";
import { listActiveCategories } from "../transactions/api/transactions";
import { MetricCard } from "../../shared/ui/MetricCard";
import { StatusChip } from "../../shared/ui/StatusChip";
import { formatMoney as formatCurrency } from "../../shared/formatMoney";
import { serializeCsv } from "../analytics/csv";
import { evaluateForecast, generateForecast, getForecastSchedule, listForecasts, setForecastSchedule, type ForecastSnapshot } from "./api/forecasting";
import { ForecastTrajectoryChart } from "./ForecastTrajectoryChart";

const today = new Date().toISOString().slice(0, 10);
const money = (minor: number, currency: string) => formatCurrency(minor, currency, { maximumFractionDigits: 2 });
const monthEnded = (monthStart: string) => { const [year, month] = monthStart.split("-").map(Number); return new Date(year, month, 0).toISOString().slice(0, 10) <= today; };

export default function ForecastingPage({ navigate }: { navigate?: (page: string) => void } = {}) {
  const [forecasts, setForecasts] = useState<ForecastSnapshot[]>([]);
  const [currencies, setCurrencies] = useState<string[]>(["KES"]);
  const [incomeCategories, setIncomeCategories] = useState<{ id: string; name: string }[]>([]);
  const [expenseCategories, setExpenseCategories] = useState<{ id: string; name: string }[]>([]);
  const [incomeCategoryId, setIncomeCategoryId] = useState("");
  const [expenseCategoryId, setExpenseCategoryId] = useState("");
  const [currency, setCurrency] = useState("KES");
  const [months, setMonths] = useState(3);
  const [scheduleEnabled, setScheduleEnabled] = useState(false);
  const [incomeDelta, setIncomeDelta] = useState(0);
  const [expenseDelta, setExpenseDelta] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const refresh = useCallback(async () => { try { setForecasts(await listForecasts()); } catch (reason) { setError(String(reason)); } }, []);
  useEffect(() => {
    void refresh();
    void getForecastSchedule().then((schedule) => { setScheduleEnabled(schedule.enabled); setCurrency(schedule.currency_code); setMonths(schedule.period_months); }).catch((reason) => setError(String(reason)));
    void listActiveAccounts().then((accounts) => { const values = Array.from(new Set(accounts.map((account) => account.currency_code))); if (values.length) { setCurrencies(values); setCurrency((current) => values.includes(current) ? current : values[0]); } }).catch((reason) => setError(String(reason)));
    void Promise.all([listActiveCategories("income"), listActiveCategories("expense")]).then(([income, expense]) => {
      setIncomeCategories(income);
      setExpenseCategories(expense);
      setIncomeCategoryId(income[0]?.id ?? "");
      setExpenseCategoryId(expense[0]?.id ?? "");
    }).catch(() => setError("Categories could not be loaded."));
  }, [refresh]);
  const create = async () => { setBusy(true); setError(""); try { await generateForecast({ currency_code: currency, period_months: months }); await refresh(); } catch (reason) { setError(String(reason)); } finally { setBusy(false); } };
  const evaluate = async (snapshotId: string, monthStart: string) => { setBusy(true); setError(""); try { await evaluateForecast(snapshotId, monthStart); await refresh(); } catch (reason) { setError(String(reason)); } finally { setBusy(false); } };
  const saveSchedule = async () => { setBusy(true); setError(""); try { const generated = await setForecastSchedule({ enabled: scheduleEnabled, currency_code: currency, period_months: months }); if (generated) await refresh(); } catch (reason) { setError(String(reason)); } finally { setBusy(false); } };
  const snapshot = forecasts[0];
  const periods = snapshot?.periods ?? [];
  const lastPeriod = periods[periods.length - 1];
  const monthlyIncome = snapshot?.assumptions.average_income_minor ?? 0;
  const monthlyExpenses = (snapshot?.assumptions.average_non_subscription_expense_minor ?? 0) + (snapshot?.assumptions.normalized_active_subscription_minor ?? 0);
  const baseEnding = lastPeriod?.predicted_ending_balance_minor ?? snapshot?.assumptions.starting_balance_minor ?? 0;
  const scenarioEnding = baseEnding + Math.round((monthlyIncome * incomeDelta / 100 - monthlyExpenses * expenseDelta / 100) * (snapshot?.period_months ?? months));
  const netMonthly = monthlyIncome - monthlyExpenses - (snapshot?.assumptions.average_savings_contribution_minor ?? 0);
  async function promoteToBudgetTemplate() {
    if (!snapshot) return;
    setBusy(true); setError(""); setMessage("");
    try {
      if (snapshot.currency_code !== "KES") throw new Error("Budget templates currently support KES forecasts only.");
      const items = [];
      if (snapshot.assumptions.average_income_minor > 0) {
        if (!incomeCategoryId) throw new Error("Create an income category before promoting this forecast.");
        items.push({ item_type: "income", category_id: incomeCategoryId, label: "Forecast monthly income", amount_minor: snapshot.assumptions.average_income_minor });
      }
      if (monthlyExpenses > 0) {
        if (!expenseCategoryId) throw new Error("Create an expense category before promoting this forecast.");
        items.push({ item_type: "expense", category_id: expenseCategoryId, label: "Forecast monthly expenses", amount_minor: monthlyExpenses });
      }
      if ((snapshot.assumptions.average_savings_contribution_minor ?? 0) > 0) items.push({ item_type: "savings", category_id: null, label: "Forecast monthly savings", amount_minor: snapshot.assumptions.average_savings_contribution_minor });
      if (!items.length) throw new Error("This forecast has no positive monthly amounts to add to a budget.");
      await invoke<string>("create_budget_template", { request: { name: `Forecast ${snapshot.generated_at.slice(0, 10)}`, currency_code: snapshot.currency_code, items } });
      setMessage("A reusable budget template was created from this forecast. It does not change transactions or existing monthly budgets.");
    } catch (reason) { setError(reason instanceof Error ? reason.message : typeof reason === "string" ? reason : "The forecast could not be added to a budget template."); }
    finally { setBusy(false); }
  }
  function exportScenarioAudit() {
    if (!snapshot) return;
    try {
      const scenarioAdjustment = monthlyIncome * incomeDelta / 100 - monthlyExpenses * expenseDelta / 100;
      const rows = [
        ["snapshot_generated_at", "month", "forecast_income_minor", "forecast_expense_minor", "forecast_ending_balance_minor", "income_change_percent", "expense_change_percent", "scenario_ending_balance_minor"],
        ...periods.map((period, index) => [snapshot.generated_at, period.month_start.slice(0, 7), String(period.predicted_income_minor), String(period.predicted_expense_minor + period.predicted_subscription_minor), String(period.predicted_ending_balance_minor), String(incomeDelta), String(expenseDelta), String(Math.round(period.predicted_ending_balance_minor + scenarioAdjustment * (index + 1)))]),
      ];
      const csv = serializeCsv(rows, [0, 1, 5, 6]);
      const url = URL.createObjectURL(new Blob([csv], { type: "text/csv;charset=utf-8" }));
      const link = document.createElement("a"); link.href = url; link.download = `etracker-scenario-audit-${snapshot.generated_at.slice(0, 10)}.csv`; link.click(); URL.revokeObjectURL(url);
    } catch { setError("The scenario report could not be prepared."); }
  }
  return <section className="forecast-screen space-y-5">
    <header className="flex flex-wrap items-end justify-between gap-4"><div><p className="workspace-accent font-mono text-xs font-semibold uppercase tracking-[0.1em]">Ledger / Insights / Forecasting &amp; scenarios</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Financial Forecasting &amp; What-If Ledger Engine</h1><p className="workspace-muted mt-2 max-w-3xl text-sm">Local estimates use completed history, active subscriptions, and saved budget context. Forecasts do not change transactions or balances.</p></div><StatusChip tone="warning">Local deterministic estimate</StatusChip></header>
    <section className="workspace-panel grid items-end gap-3 rounded-lg p-4 sm:grid-cols-[1fr_1fr_auto]">
      <label className="text-xs font-semibold">Currency<select aria-label="Forecast currency" value={currency} onChange={(event) => setCurrency(event.target.value)} className="mt-1 block w-full rounded-md border px-3 py-2 text-sm">{currencies.map((item) => <option key={item}>{item}</option>)}</select></label>
      <fieldset className="min-w-0"><legend className="text-xs font-semibold">Forecast horizon</legend><div className="mt-1 flex flex-wrap gap-1" role="group" aria-label="Forecast length">{[1, 3, 6, 12].map((value) => <button key={value} type="button" aria-pressed={months === value} onClick={() => setMonths(value)} className={`rounded-sm border px-3 py-2 text-xs font-semibold ${months === value ? "border-violet-300 bg-violet-50 text-violet-800" : "border-slate-200 bg-white text-slate-600"}`}>{value} mo</button>)}</div></fieldset>
      <button type="button" disabled={busy} onClick={create} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50">{busy ? "Generating…" : "Generate forecast"}</button>
      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-slate-200 pt-3 sm:col-span-3"><label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={scheduleEnabled} onChange={(event) => setScheduleEnabled(event.target.checked)} /><span><strong>Generate monthly</strong><span className="workspace-muted ml-2 text-xs">When ETracker starts</span></span></label><button type="button" disabled={busy} onClick={saveSchedule} className="rounded-md border border-slate-300 px-3 py-2 text-sm">{busy ? "Saving…" : "Save schedule"}</button></div>
    </section>
    {error && <p role="alert" className="rounded-md bg-red-50 p-3 text-sm text-red-800">{error}</p>}{message && <p role="status" className="rounded-md bg-emerald-50 p-3 text-sm text-emerald-800">{message}</p>}
    {!snapshot ? <p className="workspace-panel rounded-lg p-8 text-center text-sm">No forecast snapshots yet. Generate one to save a view based on current assumptions.</p> : <>
      <div className="grid gap-3 lg:grid-cols-3"><MetricCard label={`Projected ending balance · ${snapshot.period_months} mo`} value={money(baseEnding, snapshot.currency_code)} detail={`History coverage: ${snapshot.confidence}`} /><MetricCard label="Projected monthly surplus" value={money(netMonthly, snapshot.currency_code)} detail="Income less expenses, subscriptions, and savings" tone={netMonthly >= 0 ? "positive" : "negative"} /><MetricCard label="Starting ledger balance" value={money(snapshot.assumptions.starting_balance_minor, snapshot.currency_code)} detail={`${snapshot.assumptions.active_months_with_activity} of 6 history months active`} /></div>
      <section className="workspace-panel rounded-lg p-5"><div className="flex flex-wrap items-end justify-between gap-3"><div><h2 className="text-xl font-semibold">Projected balance and monthly net cash flow</h2><p className="workspace-muted mt-1 text-xs">History coverage: {snapshot.confidence} · {snapshot.assumptions.history_start} through {snapshot.assumptions.history_end} · deterministic estimate, no probability range</p></div><span className="status-chip">{snapshot.currency_code} · {snapshot.period_months}-month forecast</span></div><div className="mt-4"><ForecastTrajectoryChart periods={periods} startingBalance={snapshot.assumptions.starting_balance_minor} historyEnd={snapshot.assumptions.history_end} currency={snapshot.currency_code} /></div></section>
      <div className="grid items-start gap-4 xl:grid-cols-[minmax(0,1.7fr)_minmax(18rem,0.9fr)]"><section className="workspace-panel space-y-4 rounded-lg p-5"><div className="flex items-start justify-between gap-3"><div><h2 className="text-lg font-semibold">Interactive scenario drivers</h2><p className="workspace-muted mt-1 text-xs">Preview-only sensitivity against the saved deterministic snapshot.</p></div><button type="button" onClick={() => { setIncomeDelta(0); setExpenseDelta(0); }} className="rounded-md border border-slate-300 px-3 py-1.5 text-xs">Reset baseline</button></div>
        <ScenarioSlider label="Income change" value={incomeDelta} onChange={setIncomeDelta} min={-40} max={50} />
        <ScenarioSlider label="Expense change" value={expenseDelta} onChange={setExpenseDelta} min={-30} max={30} />
        <div className="rounded-md bg-violet-50 p-4"><p className="text-xs font-medium">Estimated end balance under these drivers</p><strong className="mt-1 block font-mono text-2xl tabular-nums">{money(scenarioEnding, snapshot.currency_code)}</strong><p className="workspace-muted mt-1 text-xs">Baseline: {money(baseEnding, snapshot.currency_code)} · Not saved or generated by a probability engine.</p></div>
        <p className="workspace-muted text-xs">Assumptions: income {money(monthlyIncome, snapshot.currency_code)}/mo; expenses {money(snapshot.assumptions.average_non_subscription_expense_minor, snapshot.currency_code)}/mo; subscriptions {money(snapshot.assumptions.normalized_active_subscription_minor, snapshot.currency_code)}/mo; savings {money(snapshot.assumptions.average_savings_contribution_minor, snapshot.currency_code)}/mo.</p>
      </section><aside className="space-y-4"><section className="workspace-panel rounded-lg p-5"><h2 className="font-semibold">Ledger guardrails</h2><p className="workspace-muted mt-2 text-sm">Generated projections are advisory only; no account or transaction is modified.</p><ul className="mt-3 space-y-2 text-sm"><li className="flex justify-between gap-3"><span>History coverage</span><strong>{snapshot.confidence}</strong></li><li className="flex justify-between gap-3"><span>Observed months</span><strong>{snapshot.assumptions.active_months_with_activity} / 6</strong></li><li className="flex justify-between gap-3"><span>Monthly net cash flow</span><strong className="font-mono">{money(netMonthly, snapshot.currency_code)}</strong></li></ul></section><section className="workspace-panel space-y-3 rounded-lg p-5"><h2 className="font-semibold">Use or export this scenario</h2><p className="workspace-muted text-sm">A budget template copies monthly averages as a starting point. Export saves the current what-if assumptions and period results as a CSV. Neither action changes account balances.</p>{snapshot.currency_code === "KES" && monthlyIncome > 0 && <label className="block text-xs font-medium">Income category<select aria-label="Forecast income category" value={incomeCategoryId} onChange={(event) => setIncomeCategoryId(event.target.value)} className="mt-1 block w-full rounded-md border px-3 py-2 text-sm"><option value="">Choose category</option>{incomeCategories.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>}{snapshot.currency_code === "KES" && monthlyExpenses > 0 && <label className="block text-xs font-medium">Expense category<select aria-label="Forecast expense category" value={expenseCategoryId} onChange={(event) => setExpenseCategoryId(event.target.value)} className="mt-1 block w-full rounded-md border px-3 py-2 text-sm"><option value="">Choose category</option>{expenseCategories.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label>}{snapshot.currency_code !== "KES" && <p className="text-xs text-amber-800">Budget templates currently support KES only.</p>}<button type="button" disabled={busy || snapshot.currency_code !== "KES"} onClick={() => void promoteToBudgetTemplate()} className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm disabled:opacity-50">{busy ? "Creating template…" : "Promote to budget plan"}</button><button type="button" onClick={exportScenarioAudit} className="w-full rounded-md border border-slate-300 px-3 py-2 text-sm">Export simulation audit</button>{navigate && <button type="button" onClick={() => navigate("Budgeting")} className="w-full text-center text-sm font-medium underline">Open Budgeting</button>}</section></aside></div>
      {snapshot.limitations.length > 0 && <section className="workspace-panel rounded-lg p-5"><h2 className="font-semibold">Model limitations</h2><ul className="workspace-muted mt-2 list-inside list-disc space-y-1 text-sm">{snapshot.limitations.map((item) => <li key={item}>{item}</li>)}</ul></section>}
      <section className="workspace-panel overflow-hidden rounded-lg"><div className="border-b border-slate-200 px-4 py-3"><h2 className="text-lg font-semibold">Saved forecast periods</h2><p className="workspace-muted text-xs">Generated {snapshot.generated_at}</p></div><div className="overflow-x-auto"><table className="w-full min-w-[760px] text-left text-sm"><thead className="bg-slate-50 text-xs uppercase"><tr><th className="px-4 py-3">Month</th><th>Income</th><th>Expenses</th><th>Subscriptions</th><th>Savings</th><th>End balance</th><th>Evaluation</th></tr></thead><tbody>{periods.map((period) => <tr key={period.month_start} className="border-t border-slate-100"><td className="px-4 py-3 font-mono">{period.month_start.slice(0, 7)}</td><td>{money(period.predicted_income_minor, snapshot.currency_code)}</td><td>{money(period.predicted_expense_minor, snapshot.currency_code)}</td><td>{money(period.predicted_subscription_minor, snapshot.currency_code)}</td><td>{money(period.predicted_savings_minor, snapshot.currency_code)}</td><td className="font-mono font-semibold">{money(period.predicted_ending_balance_minor, snapshot.currency_code)}</td><td>{period.evaluation ? "Evaluated" : monthEnded(period.month_start) ? <button type="button" disabled={busy} onClick={() => void evaluate(snapshot.id, period.month_start)} className="rounded-md border border-slate-300 px-2 py-1 text-xs">Evaluate</button> : <span className="workspace-muted">Forecast</span>}</td></tr>)}</tbody></table></div></section>
    </>}
  </section>;
}

function ScenarioSlider({ label, value, onChange, min, max }: { label: string; value: number; onChange: (value: number) => void; min: number; max: number }) { return <label className="block rounded-md bg-slate-50 p-3"><span className="flex justify-between gap-3 text-sm"><span>{label}</span><strong className="font-mono">{value > 0 ? "+" : ""}{value}%</strong></span><input aria-label={label} type="range" min={min} max={max} value={value} onChange={(event) => onChange(Number(event.target.value))} className="mt-3 w-full"/><span className="workspace-muted mt-1 flex justify-between text-[11px]"><span>{min}%</span><span>{max}%</span></span></label>; }
