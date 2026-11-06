import { useCallback, useEffect, useMemo, useState } from "react";
import { getAnalyticsSummary, type AnalyticsSummary } from "./api/analytics";

const today = new Date().toISOString().slice(0, 10);
const yearAgo = new Date(new Date().setMonth(new Date().getMonth() - 11)).toISOString().slice(0, 7) + "-01";
const money = (minor: number, currency: string) => new Intl.NumberFormat(undefined, { style: "currency", currency, maximumFractionDigits: 2 }).format(minor / 100);

export default function AnalyticsPage() {
  const [from, setFrom] = useState(yearAgo);
  const [to, setTo] = useState(today);
  const [accountId, setAccountId] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [currencyCode, setCurrencyCode] = useState("");
  const [data, setData] = useState<AnalyticsSummary | null>(null);
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
  const currency = currencyCode || data?.currencies[0] || "KES";
  const maxTrend = useMemo(() => Math.max(1, ...(data?.trends.flatMap((x) => [x.income_minor, x.expense_minor]) ?? [])), [data]);
  const exportCsv = () => {
    if (!data) return;
    const rows = [["section","period_or_name","type","amount_minor","currency"], ...data.trends.map((x) => ["trend",x.period,"income",String(x.income_minor),currency]), ...data.trends.map((x) => ["trend",x.period,"expense",String(x.expense_minor),currency]), ...data.category_breakdown.map((x) => ["category",x.name,x.transaction_type,String(x.amount_minor),currency]), ...data.account_comparison.flatMap((x) => [["account",x.name,"income",String(x.income_minor),currency],["account",x.name,"expense",String(x.expense_minor),currency]]), ...data.recurring_expenses.map((x) => ["recurring",x.name,x.transaction_type,String(x.amount_minor),currency])];
    const csv = rows.map((row) => row.map((cell) => `"${cell.replace(/"/g,'""')}"`).join(",")).join("\r\n");
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv;charset=utf-8" }));
    const link = document.createElement("a"); link.href = url; link.download = `etracker-analytics-${today}.csv`; link.click(); URL.revokeObjectURL(url);
  };
  return <section className="mx-auto max-w-6xl space-y-6">
    <header className="flex flex-wrap items-end justify-between gap-4"><div><p className="text-sm font-medium text-emerald-800">Insights</p><h1 className="mt-1 text-2xl font-bold">Analytics</h1><p className="mt-1 text-sm text-slate-600">Historical posted income and expenses. Internal transfers and reversed or voided activity are excluded.</p></div><button type="button" onClick={exportCsv} disabled={!data} className="rounded-lg border border-slate-300 bg-white px-4 py-2 text-sm font-medium disabled:opacity-50">Export CSV</button></header>
    <div className="grid gap-3 rounded-xl border border-slate-200 bg-white p-4 sm:grid-cols-2 lg:grid-cols-5">
      <label className="text-xs font-semibold text-slate-600">From<input aria-label="From date" type="date" value={from} onChange={(e) => setFrom(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm" /></label>
      <label className="text-xs font-semibold text-slate-600">To<input aria-label="To date" type="date" value={to} onChange={(e) => setTo(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm" /></label>
      <label className="text-xs font-semibold text-slate-600">Currency<select aria-label="Currency" value={currencyCode} onChange={(e) => setCurrencyCode(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm"><option value="">Choose currency</option>{data?.currencies.map((c) => <option key={c}>{c}</option>)}</select></label>
      <label className="text-xs font-semibold text-slate-600">Account<select aria-label="Account" value={accountId} onChange={(e) => setAccountId(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm"><option value="">All accounts</option>{data?.accounts.filter((a) => !a.currency_code || a.currency_code === currency).map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}</select></label>
      <label className="text-xs font-semibold text-slate-600">Category<select aria-label="Category" value={categoryId} onChange={(e) => setCategoryId(e.target.value)} className="mt-1 block w-full rounded-md border border-slate-300 px-2 py-2 text-sm"><option value="">All categories</option>{data?.categories.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select></label>
    </div>
    {error && <p role="alert" className="rounded-lg bg-red-50 p-3 text-sm text-red-800">{error}</p>}
    {busy && !data ? <p className="text-sm text-slate-600">Loading analytics…</p> : data && <>
      <div className="grid gap-4 sm:grid-cols-2"><Metric title="Income" value={money(data.total_income_minor,currency)} /><Metric title="Expenses" value={money(data.total_expense_minor,currency)} /></div>
      <div className="grid gap-5 lg:grid-cols-2">
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Income and expense trends</h2><p className="mb-5 mt-1 text-xs text-slate-500">Monthly totals · {currency}</p>{data.trends.length ? <div className="space-y-3">{data.trends.map((point) => <div key={point.period} className="grid grid-cols-[3.5rem_1fr_6rem] items-center gap-3 text-xs"><span className="font-medium">{point.period}</span><div className="space-y-1"><div title={`Income ${money(point.income_minor,currency)}`} className="h-2 rounded bg-emerald-600" style={{ width: `${Math.max(point.income_minor ? 2 : 0,point.income_minor/maxTrend*100)}%` }} /><div title={`Expense ${money(point.expense_minor,currency)}`} className="h-2 rounded bg-rose-500" style={{ width: `${Math.max(point.expense_minor ? 2 : 0,point.expense_minor/maxTrend*100)}%` }} /></div><span className="text-right text-slate-500">{money(point.income_minor-point.expense_minor,currency)}</span></div>)}</div> : <Empty>No posted activity in this range.</Empty>}<div className="mt-4 flex gap-4 text-xs text-slate-600"><span>● Income</span><span className="text-rose-600">● Expenses</span></div></section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Category breakdown</h2><p className="mb-4 mt-1 text-xs text-slate-500">Allocated amounts · {currency}</p>{data.category_breakdown.length ? <div className="space-y-3">{data.category_breakdown.map((item,index) => <div key={`${item.transaction_type}-${item.name}-${index}`} className="flex items-center justify-between gap-3 text-sm"><span><span className={`mr-2 inline-block h-2 w-2 rounded-full ${item.transaction_type === "expense" ? "bg-rose-500" : "bg-emerald-600"}`} />{item.name}<span className="ml-2 text-xs capitalize text-slate-500">{item.transaction_type}</span></span><strong>{money(item.amount_minor,currency)}</strong></div>)}</div> : <Empty>No categorized activity for these filters.</Empty>}</section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Account comparison</h2><p className="mb-4 mt-1 text-xs text-slate-500">Income and expenses · {currency}</p>{data.account_comparison.length ? <div className="space-y-3">{data.account_comparison.map((item) => <div key={item.account_id} className="grid grid-cols-[1fr_auto_auto] gap-4 text-sm"><strong>{item.name}</strong><span className="text-emerald-800">+{money(item.income_minor,currency)}</span><span className="text-rose-700">−{money(item.expense_minor,currency)}</span></div>)}</div> : <Empty>No account activity for these filters.</Empty>}</section>
        <section className="rounded-xl border border-slate-200 bg-white p-5"><h2 className="font-semibold">Repeated expense descriptions</h2><p className="mb-4 mt-1 text-xs text-slate-500">Same description appears at least three times · {currency}</p>{data.recurring_expenses.length ? <div className="space-y-3">{data.recurring_expenses.map((item) => <div key={item.name} className="flex justify-between gap-3 text-sm"><span>{item.name}</span><strong>{money(item.amount_minor,currency)}</strong></div>)}</div> : <Empty>No repeated descriptions found in this range.</Empty>}</section>
      </div>
    </>}
  </section>;
}

function Metric({ title, value }: { title: string; value: string }) { return <div className="rounded-xl border border-slate-200 bg-white p-5"><p className="text-sm text-slate-500">{title}</p><p className="mt-2 text-2xl font-bold tabular-nums">{value}</p></div>; }
function Empty({ children }: { children: React.ReactNode }) { return <p className="rounded-lg bg-slate-50 p-4 text-sm text-slate-500">{children}</p>; }
