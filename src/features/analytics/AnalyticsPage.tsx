import { useCallback, useEffect, useState } from "react";
import { getAnalyticsSummary, type AnalyticsSummary } from "./api/analytics";
import { serializeCsv } from "./csv";
import { AccountActivityChart, ExpenseCategoryDonut, MonthlyTrendChart } from "./AnalyticsCharts";
import { formatMoney as formatCurrency } from "../../shared/formatMoney";

const today = new Date().toISOString().slice(0, 10);
const yearAgo = new Date(new Date().setMonth(new Date().getMonth() - 11)).toISOString().slice(0, 7) + "-01";
const money = (minor: number, currency: string) => formatCurrency(minor, currency, { maximumFractionDigits: 2 });
const dateOnly = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;

export default function AnalyticsPage() {
  const [from, setFrom] = useState(yearAgo);
  const [to, setTo] = useState(today);
  const [accountId, setAccountId] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [currencyCode, setCurrencyCode] = useState("");
  const [periodPreset, setPeriodPreset] = useState<"30d" | "6m" | "12m" | "custom">("12m");
  const [data, setData] = useState<AnalyticsSummary | null>(null);
  const [scenarioIncome, setScenarioIncome] = useState("");
  const [scenarioExpenseReduction, setScenarioExpenseReduction] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const refresh = useCallback(async () => {
    setBusy(true); setError("");
    try {
      const result = await getAnalyticsSummary({ from, to, accountId, categoryId, currencyCode });
      setData(result);
      if (!currencyCode && result.currencies.length) setCurrencyCode(result.currencies[0]);
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }, [from, to, accountId, categoryId, currencyCode]);
  useEffect(() => { void refresh(); }, [refresh]);
  function choosePeriod(preset: "30d" | "6m" | "12m") {
    const end = new Date();
    const start = new Date(end);
    if (preset === "30d") start.setDate(start.getDate() - 29);
    else {
      start.setDate(1);
      start.setMonth(start.getMonth() - (preset === "6m" ? 5 : 11));
    }
    setFrom(dateOnly(start));
    setTo(dateOnly(end));
    setPeriodPreset(preset);
  }
  const currency = currencyCode || data?.currencies[0] || "KES";
  const scenarioNet = data ? data.total_income_minor + data.total_refund_minor + Math.max(0, Math.round((Number(scenarioIncome) || 0) * 100)) - Math.max(0, data.total_expense_minor - Math.max(0, Math.round((Number(scenarioExpenseReduction) || 0) * 100))) : 0;
  const exportCsv = () => {
    if (!data) return;
    const rows = [["section","period_or_name","type","amount_minor","currency"], ...data.trends.map((x) => ["trend",x.period,"income",String(x.income_minor),currency]), ...data.trends.map((x) => ["trend",x.period,"expense",String(x.expense_minor),currency]), ...data.trends.map((x) => ["trend",x.period,"refund",String(x.refund_minor),currency]), ...data.category_breakdown.map((x) => ["category",x.name,x.transaction_type,String(x.amount_minor),currency]), ...data.account_comparison.flatMap((x) => [["account",x.name,"income",String(x.income_minor),currency],["account",x.name,"expense",String(x.expense_minor),currency],["account",x.name,"refund",String(x.refund_minor),currency]]), ...data.recurring_expenses.map((x) => ["recurring",x.name,x.transaction_type,String(x.amount_minor),currency]), ...data.unusual_expenses.map((x) => ["unusual",x.occurred_at.slice(0,10),x.category_name,String(x.amount_minor),currency]), ...data.budget_performance.flatMap((x) => [["budget",x.name,"planned expense",String(x.planned_expense_minor),currency],["budget",x.name,"actual expense",String(x.actual_expense_minor),currency],["budget",x.name,"planned savings",String(x.planned_savings_minor),currency],["budget",x.name,"actual savings",String(x.actual_savings_minor),currency]]), ...data.savings_progress.map((x) => ["savings",x.name,"contributed",String(x.contributed_minor),currency])];
    const csv = serializeCsv(rows, [1, 2]);
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv;charset=utf-8" }));
    const link = document.createElement("a"); link.href = url; link.download = `etracker-analytics-${today}.csv`; link.click(); URL.revokeObjectURL(url);
  };
  return <section className="analytics-screen space-y-5">
    <header className="flex flex-wrap items-end justify-between gap-4"><div><p className="workspace-accent font-mono text-xs font-semibold uppercase tracking-[0.1em]">Ledger / Insights / Analytics &amp; Reporting</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Financial Analytics &amp; Ledger Intelligence</h1><p className="workspace-muted mt-2 text-sm">Historical posted activity for the selected period. Transfers and reversed or voided entries are excluded.</p></div><button type="button" onClick={exportCsv} disabled={!data} className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-50">Export statement CSV</button></header>
    <div className="flex flex-wrap items-center justify-between gap-3">
      <div className="flex flex-wrap items-center gap-1" role="group" aria-label="Report period">
        {([ ["30d", "30 days"], ["6m", "6 months"], ["12m", "12 months"], ["custom", "Custom range"] ] as const).map(([value, label]) => (
          <button key={value} type="button" aria-pressed={periodPreset === value} onClick={() => value === "custom" ? setPeriodPreset("custom") : choosePeriod(value)} className={`rounded-sm border px-3 py-2 text-xs font-semibold ${periodPreset === value ? "border-violet-300 bg-violet-50 text-violet-800" : "border-slate-200 bg-white text-slate-600"}`}>{label}</button>
        ))}
      </div>
      <p className="workspace-muted font-mono text-xs tabular-nums">{from} – {to}</p>
    </div>
    <div className="analytics-filters grid gap-3 rounded-lg border border-slate-200 bg-white p-4 sm:grid-cols-2 lg:grid-cols-5">
      <label className="text-xs font-semibold text-slate-600">From<input aria-label="From date" type="date" value={from} onChange={(e) => { setFrom(e.target.value); setPeriodPreset("custom"); }} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm" /></label>
      <label className="text-xs font-semibold text-slate-600">To<input aria-label="To date" type="date" value={to} onChange={(e) => { setTo(e.target.value); setPeriodPreset("custom"); }} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm" /></label>
      <label className="text-xs font-semibold text-slate-600">Currency<select aria-label="Currency" value={currencyCode} onChange={(e) => setCurrencyCode(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm"><option value="">Choose currency</option>{data?.currencies.map((c) => <option key={c}>{c}</option>)}</select></label>
      <label className="text-xs font-semibold text-slate-600">Account<select aria-label="Account" value={accountId} onChange={(e) => setAccountId(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm"><option value="">All accounts</option>{data?.accounts.filter((a) => !a.currency_code || a.currency_code === currency).map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}</select></label>
      <label className="text-xs font-semibold text-slate-600">Category<select aria-label="Category" value={categoryId} onChange={(e) => setCategoryId(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm"><option value="">All categories</option>{data?.categories.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select></label>
    </div>
    {error && <p role="alert" className="rounded-lg bg-red-50 p-3 text-sm text-red-800">{error}</p>}
    {busy && !data ? <p className="text-sm text-slate-600">Loading analytics…</p> : data && <>
      <div className="grid gap-3 sm:grid-cols-2 2xl:grid-cols-4"><Metric title="Total income" value={money(data.total_income_minor,currency)} /><Metric title="Gross expenses" value={money(data.total_expense_minor,currency)} /><Metric title="Refunds" value={money(data.total_refund_minor,currency)} /><Metric title="Net spending" value={money(data.total_net_spending_minor,currency)} /></div>
      <section className="rounded-xl border border-dashed border-emerald-300 bg-emerald-50/50 p-5"><div className="flex flex-wrap items-start justify-between gap-4"><div><h2 className="font-semibold">What-if simulation</h2><p className="mt-1 text-xs text-slate-600">Try changes for this selected period. The estimate is temporary and does not alter budgets or transactions.</p></div><strong className="text-lg">Simulated net: {money(scenarioNet,currency)}</strong></div><div className="mt-4 grid gap-3 sm:grid-cols-2"><label className="text-xs font-semibold text-slate-600">Additional income<input aria-label="Additional simulated income" type="number" min="0" step="0.01" value={scenarioIncome} onChange={(e) => setScenarioIncome(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm" /></label><label className="text-xs font-semibold text-slate-600">Expense reduction<input aria-label="Simulated expense reduction" type="number" min="0" step="0.01" value={scenarioExpenseReduction} onChange={(e) => setScenarioExpenseReduction(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm" /></label></div></section>
      <div className="analytics-report-grid grid gap-5 lg:grid-cols-2">
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="text-lg font-semibold">Monthly income vs expenditure trajectory</h2><p className="mb-5 mt-1 text-xs text-slate-500">Inflows, gross expenses, refunds, net cash flow, and 3-month average · {currency}</p>{data.trends.length ? <MonthlyTrendChart points={data.trends} currency={currency} /> : <Empty>No posted activity in this range.</Empty>}</section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Category breakdown</h2><p className="mb-4 mt-1 text-xs text-slate-500">Expense mix by allocated amount · {currency}</p><ExpenseCategoryDonut categories={data.category_breakdown} currency={currency} />{data.category_breakdown.some((item) => item.transaction_type === "income") && <div className="mt-5 border-t border-slate-200 pt-4"><h3 className="mb-3 text-sm font-semibold">Income categories</h3><ul className="space-y-2">{data.category_breakdown.filter((item) => item.transaction_type === "income").map((item,index) => <li key={`${item.category_id}-${item.name}-${index}`} className="flex justify-between gap-3 text-sm"><span>{item.name}</span><strong>{money(item.amount_minor,currency)}</strong></li>)}</ul></div>}</section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="text-lg font-semibold">Cash flow by account</h2><p className="mb-4 mt-1 text-xs text-slate-500">Posted income, gross expenses and refunds · {currency}</p><AccountActivityChart accounts={data.account_comparison} currency={currency} /></section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Repeated expense descriptions</h2><p className="mb-4 mt-1 text-xs text-slate-500">Same description appears at least three times · {currency}</p>{data.recurring_expenses.length ? <div className="space-y-3">{data.recurring_expenses.map((item) => <div key={item.name} className="flex justify-between gap-3 text-sm"><span>{item.name}</span><strong>{money(item.amount_minor,currency)}</strong></div>)}</div> : <Empty>No repeated descriptions found in this range.</Empty>}</section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Spending well above recent average</h2><p className="mb-4 mt-1 text-xs text-slate-500">At least twice the six-month category average, with three earlier records. Review these entries; this is not a fraud judgment.</p>{data.unusual_expenses.length ? <div className="space-y-3">{data.unusual_expenses.map((item) => <div key={item.transaction_id} className="flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 pb-2 text-sm"><span><strong>{item.description || item.category_name}</strong><span className="ml-2 text-xs text-slate-500">{item.occurred_at.slice(0,10)} · {item.category_name}</span><span className="block text-xs text-slate-500">Average {money(item.recent_average_minor,currency)} across {item.comparison_count} earlier entries</span></span><strong className="text-amber-800">{money(item.amount_minor,currency)}</strong></div>)}</div> : <Empty>No entries meet this review threshold for the selected range.</Empty>}</section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Budget performance</h2><p className="mb-4 mt-1 text-xs text-slate-500">Stored monthly plans compared with posted activity · {currency}</p>{data.budget_performance.length ? <div className="space-y-4">{data.budget_performance.map((budget) => <div key={budget.budget_id} className="border-b border-slate-100 pb-3"><div className="flex justify-between gap-3"><strong>{budget.name}</strong><span className="text-xs text-slate-500">{budget.starts_on}–{budget.ends_on}</span></div><div className="mt-2 grid grid-cols-2 gap-2 text-xs text-slate-600"><span>Income · plan {money(budget.planned_income_minor,currency)} / actual {money(budget.actual_income_minor,currency)}</span><span>Expenses · plan {money(budget.planned_expense_minor,currency)} / actual {money(budget.actual_expense_minor,currency)}</span><span>Savings · plan {money(budget.planned_savings_minor,currency)} / actual {money(budget.actual_savings_minor,currency)}</span></div></div>)}</div> : <Empty>No budget periods overlap this report range.</Empty>}</section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Savings progress</h2><p className="mb-4 mt-1 text-xs text-slate-500">Contributions recorded in this report range · {currency}</p>{data.savings_progress.length ? <div className="space-y-4">{data.savings_progress.map((goal) => <div key={goal.goal_id}><div className="flex justify-between gap-3 text-sm"><strong>{goal.name}</strong><span>{money(goal.contributed_minor,currency)} / {money(goal.target_amount_minor,currency)}</span></div><div className="mt-2 h-2 overflow-hidden rounded bg-slate-100"><div className="h-full rounded bg-emerald-600" style={{ width: `${Math.min(100,goal.contributed_minor/goal.target_amount_minor*100)}%` }} /></div></div>)}</div> : <Empty>No savings contributions match these filters.</Empty>}</section>
      </div>
    </>}
  </section>;
}

function Metric({ title, value }: { title: string; value: string }) { return <div className="rounded-xl border border-slate-200 bg-white p-5"><p className="text-sm text-slate-500">{title}</p><p className="mt-2 text-2xl font-bold tabular-nums">{value}</p></div>; }
function Empty({ children }: { children: React.ReactNode }) { return <p className="rounded-lg bg-slate-50 p-4 text-sm text-slate-500">{children}</p>; }
