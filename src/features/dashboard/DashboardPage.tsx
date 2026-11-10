import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getAnalyticsSummary, type AnalyticsSummary } from "../analytics/api/analytics";
import { ExpenseCategoryDonut, MonthlyTrendChart } from "../analytics/AnalyticsCharts";

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
};

const money = (minor: number, currency: string) =>
  new Intl.NumberFormat(undefined, {
    style: "currency",
    currency,
    maximumFractionDigits: 2,
  }).format(minor / 100);

function DashboardPage({ navigate }: { navigate: (page: string) => void }) {
  const [data, setData] = useState<Summary | null>(null);
  const [analytics, setAnalytics] = useState<AnalyticsSummary | null>(null);
  const [analyticsError, setAnalyticsError] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<Summary>("get_dashboard_summary")
      .then(setData)
      .catch(() => setError("Dashboard information could not be loaded."));
    const end = new Date();
    const start = new Date(end.getFullYear(), end.getMonth() - 5, 1);
    const dateString = (value: Date) => `${value.getFullYear()}-${String(value.getMonth() + 1).padStart(2, "0")}-${String(value.getDate()).padStart(2, "0")}`;
    getAnalyticsSummary({ from: dateString(start), to: dateString(end), accountId: "", categoryId: "", currencyCode: "KES" })
      .then(setAnalytics)
      .catch(() => setAnalyticsError("The six-month activity charts could not be loaded."));
  }, []);

  if (error) return <p role="alert" className="text-sm text-rose-700">{error}</p>;
  if (!data) return <p role="status" className="text-sm text-slate-500">Loading overview…</p>;

  const metrics = [
    ["Account balances", money(data.balance_minor, "KES")],
    ["Income this month", money(data.income_minor, "KES")],
    ["Gross expenses", money(data.expense_minor, "KES")],
    ["Net spending", money(data.net_spending_minor, "KES")],
  ];

  return (
    <section className="mx-auto max-w-6xl space-y-6">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="workspace-accent text-xs font-semibold uppercase tracking-[0.16em]">Your household ledger</p>
          <h1 className="mt-1 text-3xl font-semibold tracking-tight">A clear view of your money</h1>
          <p className="workspace-muted mt-2 text-sm">A thoughtful summary of this month and the accounts you look after.</p>
        </div>
        <button type="button" onClick={() => navigate("Imports")} className="rounded-lg bg-emerald-700 px-4 py-2 text-sm font-semibold text-white hover:bg-emerald-800">
          Import a statement
        </button>
      </header>

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {metrics.map(([title, value]) => (
          <article key={title} className="workspace-panel rounded-xl p-5">
            <p className="workspace-muted text-sm">{title}</p>
            <p className="mt-2 text-2xl font-semibold tabular-nums">{value}</p>
          </article>
        ))}
      </div>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1.7fr)_minmax(18rem,1fr)]">
        <section className="workspace-panel min-w-0 rounded-xl p-5">
          <div className="flex flex-wrap items-baseline justify-between gap-2"><h2 className="font-semibold">Money in and out</h2><p className="workspace-muted text-xs">Last six months · income, spending and net flow</p></div>
          {analytics?.trends?.length ? <div className="mt-4"><MonthlyTrendChart points={analytics.trends} currency="KES" /></div> : <p role={analyticsError ? "alert" : "status"} className="workspace-muted mt-4 rounded-lg bg-slate-50 p-4 text-sm">{analyticsError || "Loading account activity…"}</p>}
        </section>
        <section className="workspace-panel min-w-0 rounded-xl p-5">
          <div><h2 className="font-semibold">Where spending goes</h2><p className="workspace-muted mt-1 text-xs">Gross expenses by category over the selected six-month range</p></div>
          {analytics ? <div className="mt-3"><ExpenseCategoryDonut categories={analytics.category_breakdown ?? []} currency="KES" /></div> : <p role={analyticsError ? "alert" : "status"} className="workspace-muted mt-4 rounded-lg bg-slate-50 p-4 text-sm">{analyticsError || "Loading category activity…"}</p>}
        </section>
      </div>

      {data.refund_minor > 0 && (
        <p className="rounded-lg border border-sky-800/15 bg-sky-800/5 px-4 py-3 text-sm text-sky-800">
          Refunds this month: <strong>{money(data.refund_minor, "KES")}</strong>
        </p>
      )}

      <div className="grid gap-5 lg:grid-cols-2">
        <section className="workspace-panel rounded-xl p-5">
          <div className="flex items-center justify-between gap-3">
            <h2 className="font-semibold">Accounts</h2>
            <button type="button" onClick={() => navigate("Accounts")} className="workspace-accent text-xs font-semibold hover:underline">
              View accounts
            </button>
          </div>
          {data.accounts.length ? (
            <div className="mt-2">
              {data.accounts.map((account) => (
                <button
                  key={account.id}
                  type="button"
                  onClick={() => navigate("Accounts")}
                  className="workspace-divider flex w-full justify-between gap-4 border-b py-3 text-left text-sm last:border-b-0"
                >
                  <span>{account.name} <span className="workspace-muted">· {account.currency_code}</span></span>
                  <strong className="shrink-0 tabular-nums">{money(account.balance_minor, account.currency_code)}</strong>
                </button>
              ))}
            </div>
          ) : <p className="workspace-muted mt-3 text-sm">No accounts yet.</p>}
        </section>

        <section className="workspace-panel rounded-xl p-5">
          <div className="flex items-center justify-between gap-3">
            <h2 className="font-semibold">Recent transactions</h2>
            <button type="button" onClick={() => navigate("Transactions")} className="workspace-accent text-xs font-semibold hover:underline">
              View transactions
            </button>
          </div>
          {data.recent_transactions.length ? (
            <div className="mt-2">
              {data.recent_transactions.map((transaction) => (
                <button
                  key={`${transaction.id}-${transaction.account_id}`}
                  type="button"
                  onClick={() => navigate("Transactions")}
                  className="workspace-divider flex w-full justify-between gap-3 border-b py-3 text-left text-sm last:border-b-0"
                >
                  <span className="min-w-0">
                    <strong className="block truncate">{transaction.description || transaction.transaction_type}</strong>
                    <span className="workspace-muted text-xs">
                      {transaction.account_name} · {new Date(transaction.occurred_at).toLocaleDateString()} · {transaction.transaction_type}
                    </span>
                  </span>
                  <span className={`shrink-0 tabular-nums ${transaction.entry_amount_minor < 0 ? "text-rose-700" : "text-emerald-700"}`}>
                    {transaction.entry_amount_minor < 0 ? "−" : "+"}{money(Math.abs(transaction.entry_amount_minor), transaction.currency_code)}
                  </span>
                </button>
              ))}
            </div>
          ) : <p className="workspace-muted mt-3 text-sm">No posted transactions yet.</p>}
        </section>

        <section className="workspace-panel rounded-xl p-5">
          <h2 className="font-semibold">Budget progress</h2>
          {data.budgets.length ? data.budgets.map((budget) => {
            const progress = budget.planned_expense_minor > 0
              ? Math.max(0, Math.min(100, budget.actual_expense_minor / budget.planned_expense_minor * 100))
              : 0;
            return (
              <div key={budget.id} className="mt-4">
                <div className="flex flex-wrap justify-between gap-2 text-sm">
                  <span>{budget.name} <span className="workspace-muted">· ends {budget.ends_on}</span></span>
                  <span className="tabular-nums">{money(budget.actual_expense_minor, budget.currency_code)} / {money(budget.planned_expense_minor, budget.currency_code)}</span>
                </div>
                <div
                  className="workspace-accent-track mt-2 h-2 overflow-hidden rounded-full"
                  role="progressbar"
                  aria-label={`${budget.name} spending`}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={Math.round(progress)}
                >
                  <div className="h-full rounded-full bg-violet-700" style={{ width: `${progress}%` }} />
                </div>
              </div>
            );
          }) : <p className="workspace-muted mt-3 text-sm">No current budget periods.</p>}
        </section>

        <section className="workspace-panel rounded-xl p-5">
          <h2 className="font-semibold">Savings and upcoming subscriptions</h2>
          {data.savings_goals.map((goal) => (
            <p key={goal.id} className="workspace-divider mt-3 border-b pb-3 text-sm last:border-b-0">
              {goal.name} <span className="workspace-muted">·</span> {money(goal.contributed_minor, goal.currency_code)} of {money(goal.target_amount_minor, goal.currency_code)}
            </p>
          ))}
          {data.upcoming_subscriptions.map((subscription) => (
            <p key={subscription.id} className="workspace-divider mt-3 border-b pb-3 text-sm last:border-b-0">
              {subscription.name} <span className="workspace-muted">· due {subscription.next_due_date}</span> · {money(subscription.amount_minor, subscription.currency_code)}
            </p>
          ))}
          {!data.savings_goals.length && !data.upcoming_subscriptions.length && (
            <p className="workspace-muted mt-3 text-sm">No active goals or subscriptions due in the next 30 days.</p>
          )}
        </section>
      </div>
    </section>
  );
}

export default DashboardPage;
