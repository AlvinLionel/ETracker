import type { ForecastPeriod } from "./api/forecasting";
import { formatMoney } from "../../shared/formatMoney";

type ForecastTrajectoryChartProps = {
  periods: ForecastPeriod[];
  startingBalance: number;
  historyEnd: string;
  currency: string;
};

export function ForecastTrajectoryChart({ periods, startingBalance, historyEnd, currency }: ForecastTrajectoryChartProps) {
  const points = [
    { month: historyEnd, balance: startingBalance, netFlow: 0, baseline: true },
    ...periods.map((period) => ({
      month: period.month_start,
      balance: period.predicted_ending_balance_minor,
      netFlow: period.predicted_income_minor - period.predicted_expense_minor - period.predicted_subscription_minor - period.predicted_savings_minor,
      baseline: false,
    })),
  ];
  const width = Math.max(760, points.length * 64 + 110);
  const height = 360;
  const left = 96;
  const right = 18;
  const plotWidth = width - left - right;
  const balanceTop = 28;
  const balanceBottom = 190;
  const balanceMin = Math.min(0, ...points.map((point) => point.balance));
  const balanceMax = Math.max(1, ...points.map((point) => point.balance));
  const balanceRange = balanceMax - balanceMin || 1;
  const yBalance = (value: number) => balanceBottom - (value - balanceMin) / balanceRange * (balanceBottom - balanceTop);
  const xPoint = (index: number) => left + index * plotWidth / Math.max(1, points.length - 1);
  const flows = points.slice(1).map((point) => point.netFlow);
  const flowMax = Math.max(1, ...flows.map(Math.abs));
  const flowZero = 278;
  const flowScale = 42 / flowMax;
  const flowGroup = plotWidth / Math.max(1, flows.length);
  const flowBarWidth = Math.min(26, flowGroup * 0.48);
  const monthLabel = (value: string) => new Intl.DateTimeFormat(undefined, { month: "short", year: "2-digit" }).format(new Date(`${value.slice(0, 7)}-01T00:00:00`));
  const balanceLine = points.map((point, index) => `${xPoint(index)},${yBalance(point.balance)}`).join(" ");
  const baselineX = xPoint(0);

  return <>
    <div className="overflow-x-auto" role="region" aria-label="Forecast trajectory chart" tabIndex={0}>
      <svg className="block min-w-[760px] w-full" viewBox={`0 0 ${width} ${height}`} role="img" aria-label="Deterministic ending balance and monthly cash flow forecast" aria-describedby="forecast-trajectory-description">
        <title>Forecast balance and monthly cash flow</title>
        <desc id="forecast-trajectory-description">Saved deterministic projections from {historyEnd.slice(0, 7)} through {periods[periods.length - 1]?.month_start.slice(0, 7)}. The upper chart shows ending balance. Lower bars show each month's net cash flow on an independent scale. No probability range is calculated.</desc>
        {[0, 1, 2, 3].map((step) => {
          const value = balanceMin + balanceRange * step / 3;
          const y = yBalance(value);
          return <g key={step}>
            <line x1={left} x2={width - right} y1={y} y2={y} stroke="var(--workspace-line)" strokeDasharray={step === 0 ? undefined : "3 4"} />
            <text x={left - 8} y={y + 3} textAnchor="end" fill="var(--workspace-muted)" fontSize="10">{new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 }).format(value)}</text>
          </g>;
        })}
        <text x="4" y={balanceTop + 8} fill="var(--workspace-muted)" fontSize="10">{currency} balance</text>
        <polyline points={balanceLine} fill="none" stroke="var(--workspace-accent)" strokeWidth="3" strokeLinejoin="round" strokeLinecap="round" />
        <line x1={baselineX} x2={baselineX} y1={balanceTop} y2="320" stroke="var(--workspace-muted)" strokeDasharray="4 5" />
        <text x={baselineX + 6} y={balanceTop + 12} fill="var(--workspace-muted)" fontSize="9">TODAY</text>
        {points.map((point, index) => <g key={`${point.month}-${index}`}>
          <circle cx={xPoint(index)} cy={yBalance(point.balance)} r={index === 0 ? 4 : 3.5} fill="var(--workspace-surface)" stroke="var(--workspace-accent)" strokeWidth="2">
            <title>{point.baseline ? "Starting balance" : "Projected ending balance"}: {formatMoney(point.balance, currency)}</title>
          </circle>
          <text x={xPoint(index)} y="211" textAnchor="middle" fill="var(--workspace-muted)" fontSize="9">{monthLabel(point.month)}</text>
        </g>)}
        <text x="4" y="245" fill="var(--workspace-muted)" fontSize="10">Monthly net flow</text>
        <line x1={left} x2={width - right} y1={flowZero} y2={flowZero} stroke="var(--workspace-line)" />
        {points.slice(1).map((point, index) => {
          const x = (xPoint(index) + xPoint(index + 1)) / 2 - flowBarWidth / 2;
          const barHeight = Math.abs(point.netFlow) * flowScale;
          return <rect key={`flow-${point.month}-${index}`} x={x} y={point.netFlow >= 0 ? flowZero - barHeight : flowZero} width={flowBarWidth} height={Math.max(1, barHeight)} rx="2" fill={point.netFlow >= 0 ? "var(--workspace-positive)" : "var(--workspace-negative)"}>
            <title>{monthLabel(point.month)} net cash flow: {formatMoney(point.netFlow, currency)}</title>
          </rect>;
        })}
        {points.map((point, index) => <text key={`month-${point.month}-${index}`} x={xPoint(index)} y="338" textAnchor="middle" fill="var(--workspace-muted)" fontSize="9">{monthLabel(point.month)}</text>)}
      </svg>
    </div>
    <div className="mt-3 flex flex-wrap gap-x-5 gap-y-2 text-xs text-slate-600" aria-hidden="true">
      <span className="inline-flex items-center gap-2"><span className="h-0.5 w-4" style={{ backgroundColor: "var(--workspace-accent)" }} />Projected ending balance</span>
      <span className="inline-flex items-center gap-2"><span className="h-2.5 w-2.5 rounded-sm" style={{ backgroundColor: "var(--workspace-positive)" }} />Positive net cash flow</span>
      <span className="inline-flex items-center gap-2"><span className="h-2.5 w-2.5 rounded-sm" style={{ backgroundColor: "var(--workspace-negative)" }} />Negative net cash flow</span>
    </div>
    <table className="sr-only">
      <caption>Exact deterministic forecast values in {currency}</caption>
      <thead><tr><th scope="col">Month</th><th scope="col">Ending balance</th><th scope="col">Net cash flow</th></tr></thead>
      <tbody>{points.map((point, index) => <tr key={`${point.month}-table-${index}`}><th scope="row">{monthLabel(point.month)}</th><td>{formatMoney(point.balance, currency)}</td><td>{point.baseline ? "Baseline" : formatMoney(point.netFlow, currency)}</td></tr>)}</tbody>
    </table>
  </>;
}