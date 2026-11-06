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
  trends: [{ period: "2026-11", income_minor: 250000, expense_minor: 87500 }],
  category_breakdown: [{ category_id: "food", name: "Food", transaction_type: "expense", amount_minor: 87500 }],
  account_comparison: [{ account_id: "cash", name: "Cash", income_minor: 250000, expense_minor: 87500 }],
  recurring_expenses: [{ category_id: null, name: "Internet", transaction_type: "expense", amount_minor: 60000 }],
};

describe("AnalyticsPage", () => {
  beforeEach(() => { invoke.mockReset(); invoke.mockResolvedValue(sample); });
  it("loads summaries with filters and exports reports", async () => {
    render(<AnalyticsPage />);
    expect(await screen.findByText("Income and expense trends")).toBeInTheDocument();
    expect(screen.getAllByText("Food").length).toBeGreaterThan(1);
    expect(screen.getByText("Internet")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Account"), { target: { value: "cash" } });
    await waitFor(() => expect(invoke).toHaveBeenLastCalledWith("get_analytics_summary", expect.objectContaining({ accountId: "cash" })));
    const create = vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:test");
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    expect(create).toHaveBeenCalled();
    expect(revoke).toHaveBeenCalledWith("blob:test");
  });
});
