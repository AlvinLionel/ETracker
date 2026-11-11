import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import DashboardPage from "./DashboardPage";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
const dashboardSummary = {
  currency_code: "KES", balance_minor: 700000, income_minor: 100000,
  expense_minor: 50000, refund_minor: 10000, net_spending_minor: 40000,
  accounts: [{ id: "cash", name: "Cash", currency_code: "KES", balance_minor: 700000 }],
  recent_transactions: [{ id: "tx", account_id: "cash", account_name: "Cash", transaction_type: "expense", amount_minor: 5000, entry_amount_minor: -5000, currency_code: "KES", occurred_at: "2026-11-06T10:00:00Z", description: "Lunch" }],
  budgets: [{ id: "budget", name: "Groceries", currency_code: "KES", ends_on: "2026-11-30", planned_expense_minor: 10000, actual_expense_minor: 12500 }],
  savings_goals: [], upcoming_subscriptions: [], import_batches_pending_review: 0, import_batches_with_discrepancy: 0,
};

describe("DashboardPage", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation((command: string) => command === "get_dashboard_summary"
      ? Promise.resolve(dashboardSummary)
      : Promise.resolve({ currencies: ["KES"], trends: [] }));
  });

  it("shows the ledger summary and navigates to transactions", async () => {
    const navigate = vi.fn();
    render(<DashboardPage navigate={navigate} />);
    expect(await screen.findByRole("heading", { name: "Financial dashboard" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Import statement" }));
    expect(navigate).toHaveBeenCalledWith("Imports");
    expect(screen.getByText("Gross expenses this month")).toBeInTheDocument();
    expect(screen.getByText("KES 400.00 net of refunds")).toBeInTheDocument();
    expect(await screen.findByText("No posted activity in this period.")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Groceries spending" })).toHaveAttribute("aria-valuenow", "100");
    fireEvent.click(screen.getByRole("button", { name: "All accounts" }));
    expect(navigate).toHaveBeenCalledWith("Accounts");
    fireEvent.click(screen.getByText("Lunch"));
    expect(navigate).toHaveBeenCalledWith("Transactions");
  });

  it("loads the selected dashboard chart period from analytics", async () => {
    render(<DashboardPage navigate={vi.fn()} />);
    fireEvent.click(await screen.findByRole("button", { name: "3M" }));

    const now = new Date();
    const start = new Date(now.getFullYear(), now.getMonth() - 2, 1);
    const dateString = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
    await screen.findByText("No posted activity in this period.");
    expect(invoke).toHaveBeenLastCalledWith("get_analytics_summary", expect.objectContaining({ from: dateString(start) }));
    expect(screen.getByRole("button", { name: "3M" })).toHaveAttribute("aria-pressed", "true");
  });

  it("shows an accessible error when the summary cannot load", async () => {
    invoke.mockRejectedValueOnce(new Error("offline"));
    render(<DashboardPage navigate={vi.fn()} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Dashboard information could not be loaded.");
  });

  it("surfaces unfinished statement imports and reconciliation differences", async () => {
    invoke.mockImplementation((command: string) => command === "get_dashboard_summary"
      ? Promise.resolve({ ...dashboardSummary, import_batches_pending_review: 2, import_batches_with_discrepancy: 1 })
      : Promise.resolve({ currencies: ["KES"], trends: [] }));
    const navigate = vi.fn();
    render(<DashboardPage navigate={navigate} />);
    expect(await screen.findByRole("region", { name: "Statement imports to review" })).toHaveTextContent("2 unfinished workspaces");
    expect(screen.getByRole("region", { name: "Statement imports to review" })).toHaveTextContent("1 balance difference needs attention");
    fireEvent.click(screen.getByRole("button", { name: "Review imports" }));
    expect(navigate).toHaveBeenCalledWith("Imports");
  });
});
