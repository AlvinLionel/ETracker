import type { AnalyticsCategory, AnalyticsPoint } from "./api/analytics";

const palette = ["var(--workspace-negative)", "var(--workspace-info)", "var(--workspace-accent)", "var(--workspace-positive)", "#be6b35", "#35867f", "var(--workspace-warning)", "#596a9b"];
const formatMoney = (minor: number, currency: string) => new Intl.NumberFormat(undefined, { style: "currency", currency, maximumFractionDigits: 0 }).format(minor / 100);
const compactMoney = (minor: number, currency: string) => `${currency} ${new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 }).format(minor / 100)}`;

export function MonthlyTrendChart({ points, currency }: { points: AnalyticsPoint[]; currency: string }) {
  const width = Math.max(680, points.length * 62);
  const height = 248;
  const left = 76;
  const right = 14;
  const top = 18;
  const bottom = 38;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const netValues = points.map((point) => point.income_minor - point.expense_minor + point.refund_minor);
  const rawMin = Math.min(0, ...netValues);
  const rawMax = Math.max(1, ...points.flatMap((point) => [point.income_minor, point.expense_minor, point.refund_minor, point.income_minor - point.expense_minor + point.refund_minor]));
  const axisStep = Math.pow(10, Math.floor(Math.log10(rawMax)));
  const axisMin = Math.floor(rawMin / axisStep) * axisStep;
  const axisMax = Math.max(axisStep, Math.ceil(rawMax / axisStep) * axisStep);
  const scaleY = (value: number) => top + (axisMax - value) / (axisMax - axisMin) * plotHeight;
  const zeroY = scaleY(0);
  const barWidth = Math.min(12, plotWidth / Math.max(points.length, 1) / 5);
  const groupWidth = plotWidth / Math.max(points.length, 1);
  const series = [
    { key: "income", label: "Income", field: "income_minor" as const, color: "var(--workspace-positive)" },
    { key: "expenses", label: "Gross expenses", field: "expense_minor" as const, color: "var(--workspace-negative)" },
    { key: "refunds", label: "Refunds", field: "refund_minor" as const, color: "var(--workspace-info)" },
  ];

  return <>
    <div className="overflow-x-auto" role="region" aria-label="Monthly financial trend chart" tabIndex={0}>
      <svg className="block w-full min-w-[680px]" viewBox={`0 0 ${width} ${height}`} role="img" aria-label="Monthly income, gross expenses and refunds chart" aria-describedby="monthly-trend-description">
        <title id="monthly-trend-title">Monthly income, gross expenses and refunds</title>
        <desc id="monthly-trend-description">Grouped monthly bars for {points[0]?.period} through {points[points.length - 1]?.period}, in {currency}. Exact values are listed below the chart.</desc>
        {[0, 1, 2, 3, 4].map((step) => {
          const value = axisMin + (axisMax - axisMin) * step / 4;
          const y = scaleY(value);
          return <g key={step}>
            <line x1={left} x2={width - right} y1={y} y2={y} stroke="var(--workspace-line)" strokeDasharray={step === 0 ? undefined : "3 4"} />
            <text x={left - 8} y={y + 4} textAnchor="end" fill="var(--workspace-muted)" fontSize="10">{compactMoney(value, currency)}</text>
          </g>;
        })}
        {points.map((point, index) => {
          const center = left + groupWidth * (index + 0.5);
          return <g key={point.period}>
            {series.map((item, seriesIndex) => {
              const value = point[item.field];
              const valueY = scaleY(value);
              const barHeight = Math.max(0, zeroY - valueY);
              const x = center + (seriesIndex - 1) * (barWidth + 3) - barWidth / 2;
              const y = Math.min(valueY, zeroY);
              return <g key={item.key}>
                <rect x={x} y={y} width={barWidth} height={Math.max(value > 0 ? 1 : 0, barHeight)} rx="2" fill={item.color}>
                  <title>{point.period} {item.label}: {formatMoney(value, currency)}</title>
                </rect>
              </g>;
            })}
            <text x={center} y={height - 19} textAnchor="middle" fill="var(--workspace-muted)" fontSize="10">{point.period.slice(2)}</text>
            <title>{point.period}: net cash flow {formatMoney(point.income_minor - point.expense_minor + point.refund_minor, currency)}</title>
          </g>;
        })}
        {points.length > 1 && <polyline fill="none" stroke="var(--workspace-accent)" strokeWidth="2.5" strokeLinejoin="round" strokeLinecap="round" points={points.map((point, index) => `${left + groupWidth * (index + 0.5)},${scaleY(point.income_minor - point.expense_minor + point.refund_minor)}`).join(" ")} />}
        {points.map((point, index) => <circle key={`net-${point.period}`} cx={left + groupWidth * (index + 0.5)} cy={scaleY(point.income_minor - point.expense_minor + point.refund_minor)} r="3.5" fill="var(--workspace-surface)" stroke="var(--workspace-accent)" strokeWidth="2"><title>{point.period} net cash flow: {formatMoney(netValues[index], currency)}</title></circle>)}
      </svg>
    </div>
    <div className="mt-4 flex flex-wrap gap-x-5 gap-y-2 text-xs text-slate-600" aria-hidden="true">
      {series.map((item) => <span key={item.key} className="inline-flex items-center gap-2"><span className="h-2.5 w-2.5 rounded-sm" style={{ backgroundColor: item.color }} />{item.label}</span>)}
      <span className="inline-flex items-center gap-2"><span className="h-0.5 w-4" style={{ backgroundColor: "var(--workspace-accent)" }} />Net cash flow</span>
    </div>
    <table className="sr-only">
      <caption>Exact monthly values for the financial trend chart, in {currency}</caption>
      <thead><tr><th scope="col">Month</th><th scope="col">Income</th><th scope="col">Gross expenses</th><th scope="col">Refunds</th><th scope="col">Net cash flow</th></tr></thead>
      <tbody>{points.map((point) => <tr key={point.period}><th scope="row">{point.period}</th><td>{formatMoney(point.income_minor, currency)}</td><td>{formatMoney(point.expense_minor, currency)}</td><td>{formatMoney(point.refund_minor, currency)}</td><td>{formatMoney(point.income_minor - point.expense_minor + point.refund_minor, currency)}</td></tr>)}</tbody>
    </table>
  </>;
}

export function ExpenseCategoryDonut({ categories, currency }: { categories: AnalyticsCategory[]; currency: string }) {
  const expenses = categories.filter((item) => item.transaction_type === "expense" && item.amount_minor > 0);
  const total = expenses.reduce((sum, item) => sum + item.amount_minor, 0);
  const circumference = 2 * Math.PI * 66;
  let offset = 0;

  if (total === 0) return <p className="rounded-lg bg-slate-50 p-4 text-sm text-slate-500">No categorized expenses to chart for these filters.</p>;

  return <div className="grid items-center gap-5 sm:grid-cols-[180px_1fr]">
    <figure className="mx-auto w-full max-w-[180px]">
      <svg viewBox="0 0 180 180" role="img" aria-label="Expense category mix chart" aria-describedby="expense-mix-description">
        <title id="expense-mix-title">Expense category mix</title>
        <desc id="expense-mix-description">Distribution of {formatMoney(total, currency)} gross categorized expenses across {expenses.length} categories. Category values and shares are listed beside the chart.</desc>
        <circle cx="90" cy="90" r="66" fill="none" stroke="var(--workspace-subtle-surface)" strokeWidth="30" />
        <g transform="rotate(-90 90 90)">
          {expenses.map((item, index) => {
            const segment = circumference * item.amount_minor / total;
            const circle = <circle key={`${item.category_id}-${item.name}-${index}`} cx="90" cy="90" r="66" fill="none" stroke={palette[index % palette.length]} strokeWidth="30" strokeDasharray={`${segment} ${circumference - segment}`} strokeDashoffset={-offset}>
              <title>{item.name}: {formatMoney(item.amount_minor, currency)} ({Math.round(item.amount_minor / total * 100)}%)</title>
            </circle>;
            offset += segment;
            return circle;
          })}
        </g>
        <text x="90" y="84" textAnchor="middle" fill="var(--workspace-muted)" fontSize="10">Gross expenses</text>
        <text x="90" y="103" textAnchor="middle" fill="var(--workspace-text)" fontSize="14" fontWeight="700">{formatMoney(total, currency)}</text>
      </svg>
      <figcaption className="sr-only">Gross expenses by category</figcaption>
    </figure>
    <ul className="space-y-2" aria-label="Expense category values">
      {expenses.map((item, index) => <li key={`${item.category_id}-${item.name}-${index}`} className="flex items-center justify-between gap-3 text-sm">
        <span className="flex min-w-0 items-center gap-2"><span aria-hidden="true" className="h-3 w-3 shrink-0 rounded-sm" style={{ backgroundColor: palette[index % palette.length] }} /><span className="truncate">{item.name}</span></span>
        <span className="shrink-0 text-right"><strong>{formatMoney(item.amount_minor, currency)}</strong><span className="ml-2 text-xs text-slate-500">{Math.round(item.amount_minor / total * 100)}%</span></span>
      </li>)}
    </ul>
  </div>;
}
