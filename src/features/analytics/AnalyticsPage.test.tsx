import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AnalyticsPage from "./AnalyticsPage";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
const confirmMock = vi.hoisted(() => vi.fn());
vi.mock("../../shared/confirmAction", () => ({ confirmAction: (...args: unknown[]) => confirmMock(...args) }));

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
  beforeEach(() => { invoke.mockReset(); confirmMock.mockReset().mockResolvedValue(true); invoke.mockImplementation((command: string) => command === "list_saved_analytics_reports" ? Promise.resolve([]) : Promise.resolve(sample)); });
  it("loads summaries with filters and exports reports", async () => {
    render(<AnalyticsPage />);
    expect(await screen.findByText("Monthly income vs expenditure trajectory")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Monthly income, gross expenses and refunds chart" })).toBeInTheDocument();
    expect(screen.getByRole("table", { name: /Exact monthly values for the financial trend chart/ })).toHaveTextContent("2026-11");
    expect(screen.getByRole("img", { name: "Income, gross expenses and refunds by account" })).toBeInTheDocument();
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
    fireEvent.click(screen.getByRole("button", { name: "Export statement CSV" }));
    expect(create).toHaveBeenCalled();
    expect(revoke).toHaveBeenCalledWith("blob:test");
  });

  it("maps the six-month preset to an inclusive calendar range", async () => {
    render(<AnalyticsPage />);
    await screen.findByText("Monthly income vs expenditure trajectory");
    fireEvent.click(screen.getByRole("button", { name: "6 months" }));

    const now = new Date();
    const start = new Date(now.getFullYear(), now.getMonth() - 5, 1);
    const dateString = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
    await waitFor(() => expect(invoke).toHaveBeenLastCalledWith("get_analytics_summary", expect.objectContaining({ from: dateString(start), to: dateString(now) })));
    expect(screen.getByRole("button", { name: "6 months" })).toHaveAttribute("aria-pressed", "true");
  });

  it("compares the selected range with an equal-length previous period", async () => {
    render(<AnalyticsPage />);
    await screen.findByText("Monthly income vs expenditure trajectory");
    fireEvent.click(screen.getByRole("button", { name: "Compare previous period" }));
    expect(await screen.findByRole("region", { name: "Comparison with previous period" })).toBeInTheDocument();
    expect(screen.getByText(/Previous range:/)).toBeInTheDocument();
    const calls = invoke.mock.calls.filter(([command]) => command === "get_analytics_summary");
    expect(calls.length).toBeGreaterThanOrEqual(2);
  });

  it("saves the selected filters as a named report for reuse", async () => {
    const saved = { id: "saved-1", name: "Monthly review", from_date: "2026-10-01", to_date: "2026-10-31", account_id: "cash", category_id: "food", currency_code: "KES", created_at: "2026-11-12", updated_at: "2026-11-12" };
    invoke.mockImplementation((command: string) => command === "list_saved_analytics_reports" ? Promise.resolve([]) : command === "save_analytics_report" ? Promise.resolve(saved) : Promise.resolve(sample));
    render(<AnalyticsPage />);
    await screen.findByText("Monthly income vs expenditure trajectory");
    fireEvent.click(screen.getByRole("button", { name: "Save current report" }));
    fireEvent.change(screen.getByLabelText("Report name"), { target: { value: "Monthly review" } });
    fireEvent.click(screen.getByRole("button", { name: "Save report" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("save_analytics_report", { request: expect.objectContaining({ name: "Monthly review", from_date: expect.any(String), to_date: expect.any(String), currency_code: "KES" }) }));
    expect(await screen.findByRole("option", { name: "Monthly review" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Delete saved report" })).toBeInTheDocument();
  });

  it("restores a saved report's date and ledger filters", async () => {
    const saved = { id: "saved-2", name: "Food in October", from_date: "2026-10-01", to_date: "2026-10-31", account_id: "cash", category_id: "food", currency_code: "KES", created_at: "2026-11-12", updated_at: "2026-11-12" };
    invoke.mockImplementation((command: string) => command === "list_saved_analytics_reports" ? Promise.resolve([saved]) : Promise.resolve(sample));
    render(<AnalyticsPage />);
    await screen.findByText("Monthly income vs expenditure trajectory");
    fireEvent.change(await screen.findByRole("combobox", { name: "Saved report" }), { target: { value: "saved-2" } });
    expect(screen.getByLabelText("From date")).toHaveValue("2026-10-01");
    expect(screen.getByLabelText("To date")).toHaveValue("2026-10-31");
    expect(screen.getByLabelText("Account")).toHaveValue("cash");
    expect(screen.getByLabelText("Category")).toHaveValue("food");
    await waitFor(() => expect(invoke).toHaveBeenLastCalledWith("get_analytics_summary", expect.objectContaining({ from: "2026-10-01", to: "2026-10-31", accountId: "cash", categoryId: "food", currencyCode: "KES" })));
  });
});
