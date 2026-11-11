import type { ReactNode } from "react";

type PageHeaderProps = {
  eyebrow: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
};

/** Shared page title block used by feature workspaces. */
export function PageHeader({ eyebrow, title, description, actions }: PageHeaderProps) {
  return (
    <header className="page-heading flex flex-wrap items-end justify-between gap-4">
      <div className="min-w-0">
        <p className="workspace-accent font-mono text-xs font-semibold uppercase tracking-[0.1em]">{eyebrow}</p>
        <h1 className="mt-1 text-3xl font-semibold tracking-tight">{title}</h1>
        {description && <p className="workspace-muted mt-2 max-w-3xl text-sm">{description}</p>}
      </div>
      {actions && <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div>}
    </header>
  );
}
