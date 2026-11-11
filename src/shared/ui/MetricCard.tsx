type MetricCardProps = {
  label: string;
  value: string;
  detail?: string;
  tone?: "neutral" | "positive" | "negative" | "accent";
};

export function MetricCard({ label, value, detail, tone = "neutral" }: MetricCardProps) {
  return (
    <article className={`workspace-panel dashboard-metric dashboard-metric-${tone} rounded-lg p-4`}>
      <p className="workspace-muted text-xs font-semibold uppercase tracking-[0.08em]">{label}</p>
      <p className="mt-3 font-mono text-2xl font-semibold tabular-nums">{value}</p>
      {detail && <p className="workspace-muted mt-2 text-xs">{detail}</p>}
    </article>
  );
}
