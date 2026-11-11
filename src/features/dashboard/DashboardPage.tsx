import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getAnalyticsSummary, type AnalyticsSummary } from "../analytics/api/analytics";
import { MonthlyTrendChart } from "../analytics/AnalyticsCharts";
import { formatMoney as formatCurrency } from "../../shared/formatMoney";

type Summary = {
  currency_code: string;
  balance_minor: number;
  income_minor: number;
  expense_minor: number;
  refund_minor: number;
  net_spending_minor: number;
  accounts: { id: string; name: string; currency_code: string; balance_minor: number }[];
  recent_transactions: {
    id: string;
    account_id: string;
    account_name: string;
    transaction_type: string;
    amount_minor: number;
    entry_amount_minor: number;
    currency_code: string;
    occurred_at: string;
    description: string | null;
  }[];
  budgets: {
    id: string;
    name: string;
    currency_code: string;
    ends_on: string;
    planned_expense_minor: number;
    actual_expense_minor: number;
  }[];
  savings_goals: { id: string; name: string; currency_code: string; target_amount_minor: number; contributed_minor: number }[];
  upcoming_subscriptions: { id: string; name: string; currency_code: string; amount_minor: number; next_due_date: string }[];
  import_batches_pending_review: number;
  import_batches_with_discrepancy: number;
};

const money = (minor: number, currency: string) => formatCurrency(minor, currency, { maximumFractionDigits: 2 });

function DashboardPage({ navigate }: { navigate: (page: string) => void }) {
  const [data, setData] = useState<Summary | null>(null);
  const [analytics, setAnalytics] = useState<AnalyticsSummary | null>(null);
  const [analyticsError, setAnalyticsError] = useState("");
  const [error, setError] = useState("");
  const [trendMonths, setTrendMonths] = useState(6);

  useEffect(() => {
    invoke<Summary>("get_dashboard_summary")
      .then(setData)
      .catch(() => setError("Dashboard information could not be loaded."));
  }, []);

  useEffect(() => {
    const end = new Date();
    const start = new Date(end.getFullYear(), end.getMonth() - trendMonths + 1, 1);
    const dateString = (value: Date) => `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, "0")}-${String(value.getDate()).padStart(2, "0")}`;
    getAnalyticsSummary({ from: dateString(start), to: dateString(end), accountId: "", categoryId: "", currencyCode: "KES" })
      .then((result) => { setAnalytics(result); setAnalyticsError(""); })
      .catch(() => { setAnalytics(null); setAnalyticsError("The selected activity chart could not be loaded."); });
  }, [trendMonths]);

  if (error) return <p role="alert" className="text-sm text-rose-700">{error}</p>;
  if (!data) return <p role="status" className="text-sm text-slate-500">Loading overview…</p>;

  const netFlow = data.income_minor - data.expense_minor + data.refund_minor;
  const totalSaved = data.savings_goals.reduce((sum, goal) => sum + goal.contributed_minor, 0);
  const metrics = [
    { title: "Total available balance", value: money(data.balance_minor, "KES"), note: `${data.accounts.length} linked accounts`, tone: "neutral" },
    { title: "Net cash flow this month", value: money(netFlow, "KES"), note: `${money(data.income_minor, "KES")} in · ${money(data.expense_minor, "KES")} out`, tone: netFlow >= 0 ? "positive" : "negative" },
    { title: "Savings and reserves", value: money(totalSaved, "KES"), note: `${data.savings_goals.length} active goals`, tone: "accent" },
    { title: "Gross expenses this month", value: money(data.expense_minor, "KES"), note: `${money(data.net_spending_minor, "KES")} net of refunds`, tone: "negative" },
  ];

  return (
    <section className="dashboard-page space-y-5">
      <header className="page-heading flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="workspace-accent text-xs font-semibold uppercase tracking-[0.12em]">Overview / Ledger</p>
          <h1 className="mt-1 text-2xl font-semibold">Financial dashboard</h1>
          <p className="workspace-muted mt-1 text-sm">Current balances and posted activity from your local ledger.</p>
        </div>
        <div className="flex flex-wrap gap-2">
          <button type="button" onClick={() => navigate("Imports")} className="rounded-md border border-slate-200 px-3 py-2 text-sm font-medium">Import statement</button>
          <button type="button" onClick={() => navigate("Transactions")} className="rounded-md bg-emerald-700 px-3 py-2 text-sm font-semibold text-white">+ Add transaction</button>
        </div>
      </header>

      <div className="grid gap-3 sm:grid-cols-2 2xl:grid-cols-4">
        {metrics.map((metric) => (
          <article key={metric.title} className={`workspace-panel dashboard-metric dashboard-metric-${metric.tone} rounded-lg p-4`}>
            <p className="workspace-muted text-xs font-semibold uppercase tracking-[0.08em]">{metric.title}</p>
            <p className="mt-3 font-mono text-2xl font-semibold tabular-nums">{metric.value}</p>
            <p className="workspace-muted mt-2 text-xs">{metric.note}</p>
          </article>
        ))}
      </div>

      {data.import_batches_pending_review > 0 && <section aria-label="Statement imports to review" className={`workspace-panel flex flex-wrap items-center justify-between gap-4 rounded-lg border p-4 ${data.import_batches_with_discrepancy ? "border-amber-300 bg-amber-50/70" : "border-emerald-200 bg-emerald-50/50"}`}><div><h2 className="font-semibold">Statement imports to review</h2><p className="workspace-muted mt-1 text-sm">{data.import_batches_pending_review} unfinished workspace{data.import_batches_pending_review === 1 ? "" : "s"}{data.import_batches_with_discrepancy ? ` · ${data.import_batches_with_discrepancy} balance ${data.import_batches_with_discrepancy === 1 ? "difference needs" : "differences need"} attention` : ""}. Review these before posting imported activity.</p></div><button type="button" onClick={() => navigate("Imports")} className="rounded-md border border-slate-300 bg-white px-3 py-2 text-sm font-semibold">Review imports</button></section>}

      <div className="grid gap-4 2xl:grid-cols-[minmax(0,1.8fr)_minmax(20rem,0.9fr)]">
        <section className="workspace-panel min-w-0 rounded-lg p-4">
          <div className="flex flex-wrap items-center justify-between gap-3"><div><h2 className="text-lg font-semibold">Cash velocity and flow</h2><p className="workspace-muted mt-1 text-xs">Monthly inflow, gross expenses, refunds and net flow · KES</p></div><div className="flex items-center gap-3"><div className="flex gap-1" role="group" aria-label="Dashboard chart period">{([ [3, "3M"], [6, "6M"], [12, "1Y"] ] as const).map(([months, label]) => <button key={months} type="button" aria-pressed={trendMonths === months} onClick={() => setTrendMonths(months)} className={`rounded-sm border px-2.5 py-1.5 font-mono text-xs font-semibold ${trendMonths === months ? "border-violet-300 bg-violet-50 text-violet-800" : "border-slate-200 text-slate-600"}`}>{label}</button>)}</div><button type="button" onClick={() => navigate("Analytics")} className="workspace-accent text-xs font-semibold">Open analytics</button></div></div>
          {analytics?.trends?.length ? <div className="mt-4"><MonthlyTrendChart points={analytics.trends} currency="KES" /></div> : <p role={analyticsError ? "alert" : "status"} className="workspace-muted mt-4 rounded-lg bg-slate-50 p-4 text-sm">{analyticsError || (analytics ? "No posted activity in this period." : "Loading account activity…")}</p>}
        </section>
        <div className="grid gap-4 md:grid-cols-2 2xl:grid-cols-1">
          <section className="workspace-panel min-w-0 rounded-lg p-4">
            <div className="flex items-center justify-between gap-3"><div><h2 className="text-lg font-semibold">Account liquidity</h2><p className="workspace-muted mt-1 text-xs">Balances by account</p></div><button type="button" onClick={() => navigate("Accounts")} className="workspace-accent text-xs font-semibold">All accounts</button></div>
            {data.accounts.length ? <div className="mt-3">
              {data.accounts.map((account) => (
                <button key={account.id} type="button" onClick={() => navigate("Accounts")} className="workspace-row workspace-divider flex w-full items-center justify-between gap-3 border-b px-2 py-3 text-left text-sm last:border-b-0">
                  <span className="min-w-0"><strong className="block truncate">{account.name}</strong><span className="workspace-muted text-xs">{account.currency_code}</span></span>
                  <strong className="shrink-0 font-mono tabular-nums">{money(account.balance_minor, account.currency_code)}</strong>
                </button>
              ))}
            </div> : <p className="workspace-muted mt-3 text-sm">No accounts yet.</p>}
          </section>
          <section className="workspace-panel min-w-0 rounded-lg p-4">
            <div className="flex items-center justify-between gap-3"><h2 className="text-lg font-semibold">Budget allocation</h2><button type="button" onClick={() => navigate("Budgeting")} className="workspace-accent text-xs font-semibold">Budgeting</button></div>
            {data.budgets.length ? data.budgets.map((budget) => {
            const progress = budget.planned_expense_minor > 0
              ? Math.max(0, Math.min(100, budget.actual_expense_minor / budget.planned_expense_minor * 100))
              : 0;
            return (
              <div key={budget.id} className="mt-4">
                <div className="flex flex-wrap justify-between gap-2 text-sm">
                  <span className="font-medium">{budget.name}</span>
                  <span className="font-mono text-xs tabular-nums">{money(budget.actual_expense_minor, budget.currency_code)} / {money(budget.planned_expense_minor, budget.currency_code)}</span>
                </div>
                <div
                  className="workspace-accent-track mt-2 h-2 overflow-hidden rounded-full"
                  role="progressbar"
                  aria-label={`${budget.name} spending`}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={Math.round(progress)}
                >
                  <div className={`h-full rounded-full ${progress >= 90 ? "bg-rose-700" : "bg-violet-700"}`} style={{ width: `${progress}%` }} />
                </div>
                <p className="workspace-muted mt-1 text-right text-xs">{Math.round(progress)}% used · ends {budget.ends_on}</p>
              </div>
            );
          }) : <p className="workspace-muted mt-3 text-sm">No current budget periods.</p>}
          </section>
        </div>
      </div>

      <div className="grid gap-4 2xl:grid-cols-[minmax(0,1.7fr)_minmax(20rem,0.9fr)]">
        <section className="workspace-panel min-w-0 overflow-hidden rounded-lg">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 px-4 py-3"><div><h2 className="text-lg font-semibold">Ledger journal</h2><p className="workspace-muted text-xs">Latest posted activity</p></div><button type="button" onClick={() => navigate("Transactions")} className="workspace-accent text-xs font-semibold">Open journal</button></div>
          {data.recent_transactions.length ? <div className="divide-y divide-slate-100">
            {data.recent_transactions.map((transaction) => (
              <button key={`${transaction.id}-${transaction.account_id}`} type="button" onClick={() => navigate("Transactions")} className="workspace-row flex w-full items-center justify-between gap-4 px-4 py-3 text-left">
                <span className="min-w-0"><strong className="block truncate text-sm">{transaction.description || transaction.transaction_type}</strong><span className="workspace-muted text-xs">{new Date(transaction.occurred_at).toLocaleDateString()} · {transaction.account_name}</span></span>
                <span className={`shrink-0 font-mono text-sm font-semibold tabular-nums ${transaction.entry_amount_minor < 0 ? "text-rose-700" : "text-emerald-700"}`}>{transaction.entry_amount_minor < 0 ? "−" : "+"}{money(Math.abs(transaction.entry_amount_minor), transaction.currency_code)}</span>
              </button>
            ))}
          </div> : <p className="workspace-muted p-4 text-sm">No posted transactions yet.</p>}
        </section>
        <div className="grid gap-4 md:grid-cols-2 2xl:grid-cols-1">
          <section className="workspace-panel rounded-lg p-4">
            <div className="flex items-center justify-between gap-3"><h2 className="text-lg font-semibold">Savings goals</h2><button type="button" onClick={() => navigate("Savings")} className="workspace-accent text-xs font-semibold">View goals</button></div>
            {data.savings_goals.length ? data.savings_goals.map((goal) => {
              const progress = goal.target_amount_minor > 0 ? Math.min(100, Math.round(goal.contributed_minor / goal.target_amount_minor * 100)) : 0;
              return <div key={goal.id} className="mt-3"><div className="flex justify-between gap-3 text-sm"><span className="truncate">{goal.name}</span><strong className="font-mono text-xs tabular-nums">{progress}%</strong></div><div className="workspace-accent-track mt-2 h-2 overflow-hidden rounded-full"><div className="h-full rounded-full bg-emerald-700" style={{ width: `${progress}%` }} /></div><p className="workspace-muted mt-1 text-xs">{money(goal.contributed_minor, goal.currency_code)} of {money(goal.target_amount_minor, goal.currency_code)}</p></div>;
            }) : <p className="workspace-muted mt-3 text-sm">No savings goals yet.</p>}
          </section>
          <section className="workspace-panel rounded-lg p-4">
            <div className="flex items-center justify-between gap-3"><h2 className="text-lg font-semibold">Upcoming commitments</h2><button type="button" onClick={() => navigate("Subscriptions")} className="workspace-accent text-xs font-semibold">Subscriptions</button></div>
            {data.upcoming_subscriptions.length ? data.upcoming_subscriptions.map((subscription) => <div key={subscription.id} className="workspace-divider flex justify-between gap-3 border-b py-3 text-sm last:border-b-0"><span className="min-w-0"><strong className="block truncate">{subscription.name}</strong><span className="workspace-muted text-xs">Due {subscription.next_due_date}</span></span><strong className="shrink-0 font-mono text-xs tabular-nums">{money(subscription.amount_minor, subscription.currency_code)}</strong></div>) : <p className="workspace-muted mt-3 text-sm">No active subscriptions due in the next 30 days.</p>}
          </section>
        </div>
      </div>
    </section>
  );
}

export default DashboardPage;
