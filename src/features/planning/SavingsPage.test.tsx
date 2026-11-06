import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import SavingsPage from "./SavingsPage";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), accounts: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../accounts/api/accounts", () => ({ listActiveAccounts: mocks.accounts }));

describe("SavingsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.accounts.mockResolvedValue([]);
    mocks.invoke.mockImplementation(async (command: string) => command === "list_savings_goals" ? [{ id: "goal", name: "Emergency fund", target_amount_minor: 100000, contributed_minor: 12500, currency_code: "KES", target_date: null, status: "active", account_id: null }] : command === "list_eligible_savings_transfers" ? [] : "created");
  });
  it("shows saved progress", async () => { render(<SavingsPage />); expect(await screen.findByText(/saved of/)).toBeInTheDocument(); });
  it("records an actual contribution without invoking transaction creation", async () => {
    render(<SavingsPage />);
    fireEvent.change(await screen.findByLabelText("Contribution amount (KES)"), { target: { value: "25.50" } });
    fireEvent.click(screen.getByRole("button", { name: "Record savings" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("add_savings_contribution", expect.objectContaining({ request: expect.objectContaining({ goal_id: "goal", amount_minor: 2550 }) })));
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_transaction", expect.anything());
  });
  it("links an existing incoming transfer as savings", async () => {
    mocks.invoke.mockImplementation(async (command: string) => command === "list_savings_goals" ? [{ id: "goal", name: "Emergency fund", target_amount_minor: 100000, contributed_minor: 0, currency_code: "KES", target_date: null, status: "active", account_id: "savings" }] : command === "list_eligible_savings_transfers" ? [{ transaction_id: "transfer", amount_minor: 5000, occurred_at: "2026-11-06T10:00:00Z", description: "Monthly transfer" }] : "created");
    render(<SavingsPage />);
    fireEvent.change(await screen.findByLabelText("Eligible transfer"), { target: { value: "transfer" } });
    fireEvent.click(screen.getByRole("button", { name: "Link selected transfer" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("add_savings_transfer", { goalId: "goal", transactionId: "transfer" }));
  });
});
