import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AnalyticsPage from "./AnalyticsPage";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const sample = {
  currencies: ["KES"],
  accounts: [{ id: "cash", name: "Cash", currency_code: "KES" }],
  categories: [{ id: "food", name: "Food", currency_code: null }],
  total_income_minor: 250000,
  total_expense_minor: 87500,
  total_refund_minor: 0,
  total_net_spending_minor:87500,
  trends: [{ period: "2026-11", income_minor: 250000, expense_minor: 87500, refund_minor:0 }],
  category_breakdown: [{ category_id: "food", name: "Food", transaction_type: "expense", amount_minor: 87500 }],
  account_comparison: [{ account_id: "cash", name: "Cash", income_minor: 250000, expense_minor: 87500, refund_minor:0 }],
  recurring_expenses: [{ category_id: null, name: "Internet", transaction_type: "expense", amount_minor: 60000 }],
  unusual_expenses: [{ transaction_id: "unusual", occurred_at: "2026-11-01T10:00:00Z", description: "Utility bill", category_name: "Utilities", amount_minor: 10000, recent_average_minor: 1000, comparison_count: 3 }],
  budget_performance: [{ budget_id: "budget", name: "October Plan", starts_on: "2026-10-01", ends_on: "2026-10-31", planned_income_minor: 250000, planned_expense_minor: 100000, planned_savings_minor: 50000, actual_income_minor: 250000, actual_expense_minor: 87500, actual_savings_minor: 25000 }],
  savings_progress: [{ goal_id: "goal", name: "Reserve", target_amount_minor: 100000, contributed_minor: 25000, status: "active" }],
};

describe("AnalyticsPage", () => {
  beforeEach(() => { invoke.mockReset(); invoke.mockResolvedValue(sample); });
  it("loads summaries with filters and exports reports", async () => {
    render(<AnalyticsPage />);
    expect(await screen.findByText("Income, expense and refund trends")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Monthly income, gross expenses and refunds chart" })).toBeInTheDocument();
    expect(screen.getByRole("table", { name: /Exact monthly values for the financial trend chart/ })).toHaveTextContent("2026-11");
    expect(screen.getByRole("img", { name: "Expense category mix chart" })).toBeInTheDocument();
    expect(screen.getAllByText("Food").length).toBeGreaterThan(1);
    expect(screen.getByText("Internet")).toBeInTheDocument();
    expect(screen.getByText("Utility bill")).toBeInTheDocument();
    expect(screen.getByText("October Plan")).toBeInTheDocument();
    expect(screen.getByText("Reserve")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Additional simulated income"), { target: { value: "100" } });
    expect(screen.getByText(/Simulated net:/).textContent).toContain("1,725");
    fireEvent.change(screen.getByLabelText("Account"), { target: { value: "cash" } });
    await waitFor(() => expect(invoke).toHaveBeenLastCalledWith("get_analytics_summary", expect.objectContaining({ accountId: "cash" })));
    const create = vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:test");
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    expect(create).toHaveBeenCalled();
    expect(revoke).toHaveBeenCalledWith("blob:test");
  });
});
