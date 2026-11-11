import type { ReactNode } from "react";

type StatusChipProps = {
  children: ReactNode;
  tone?: "neutral" | "positive" | "negative" | "warning" | "accent";
};

export function StatusChip({ children, tone = "neutral" }: StatusChipProps) {
  return <span className={`status-chip status-chip-${tone}`}>{children}</span>;
}
