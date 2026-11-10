import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import DashboardPage from "./DashboardPage";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

describe("DashboardPage", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue({
      currency_code: "KES", balance_minor: 700000, income_minor: 100000,
      expense_minor: 50000, refund_minor: 10000, net_spending_minor: 40000,
      accounts: [{ id: "cash", name: "Cash", currency_code: "KES", balance_minor: 700000 }],
      recent_transactions: [{ id: "tx", account_id: "cash", account_name: "Cash", transaction_type: "expense", amount_minor: 5000, entry_amount_minor: -5000, currency_code: "KES", occurred_at: "2026-11-06T10:00:00Z", description: "Lunch" }],
      budgets: [{ id: "budget", name: "Groceries", currency_code: "KES", ends_on: "2026-11-30", planned_expense_minor: 10000, actual_expense_minor: 12500 }],
      savings_goals: [], upcoming_subscriptions: [],
    });
  });

  it("shows the ledger summary and navigates to transactions", async () => {
    const navigate = vi.fn();
    render(<DashboardPage navigate={navigate} />);
    expect(await screen.findByRole("heading", { name: "A clear view of your money" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Import a statement" }));
    expect(navigate).toHaveBeenCalledWith("Imports");
    expect(screen.getByText("Gross expenses")).toBeInTheDocument();
    expect(screen.getByText(/Refunds this month:/)).toHaveTextContent("Ksh");
    expect(screen.getByRole("progressbar", { name: "Groceries spending" })).toHaveAttribute("aria-valuenow", "100");
    fireEvent.click(screen.getByRole("button", { name: "View accounts" }));
    expect(navigate).toHaveBeenCalledWith("Accounts");
    fireEvent.click(screen.getByText("Lunch"));
    expect(navigate).toHaveBeenCalledWith("Transactions");
  });

  it("shows an accessible error when the summary cannot load", async () => {
    invoke.mockRejectedValueOnce(new Error("offline"));
    render(<DashboardPage navigate={vi.fn()} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Dashboard information could not be loaded.");
  });
});
