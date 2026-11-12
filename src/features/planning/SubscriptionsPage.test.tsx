import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import SubscriptionsPage from "./SubscriptionsPage";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), accounts: vi.fn(), categories: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../accounts/api/accounts", () => ({ listActiveAccounts: mocks.accounts }));
vi.mock("../transactions/api/transactions", () => ({ listActiveCategories: mocks.categories }));

describe("SubscriptionsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks(); mocks.accounts.mockResolvedValue([{ id: "account", name: "Main", currency_code: "KES" }]); mocks.categories.mockResolvedValue([{ id: "utilities", name: "Utilities", category_type: "expense" }]);
    mocks.invoke.mockImplementation(async (command: string) => command === "list_subscriptions" ? [] : "created");
  });
  it("keeps projected subscriptions separate from transactions", async () => {
    render(<SubscriptionsPage />);
    fireEvent.change(await screen.findByLabelText("Name"), { target: { value: "Music" } });
    fireEvent.change(screen.getByLabelText("Amount"), { target: { value: "9.99" } });
    fireEvent.click(screen.getByRole("button", { name: "Add subscription" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("create_subscription", expect.objectContaining({ request: expect.objectContaining({ name: "Music", amount_minor: 999 }) })));
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_transaction", expect.anything());
  });
  it("saves and projects a custom subscription interval", async () => {
    render(<SubscriptionsPage />);
    fireEvent.change(await screen.findByLabelText("Name"), { target: { value: "Quarterly service" } });
    fireEvent.change(screen.getByLabelText("Amount"), { target: { value: "24.00" } });
    fireEvent.change(screen.getByLabelText("Frequency"), { target: { value: "custom" } });
    fireEvent.change(screen.getByLabelText("Custom frequency interval"), { target: { value: "3" } });
    fireEvent.change(screen.getByLabelText("Custom frequency unit"), { target: { value: "months" } });
    fireEvent.click(screen.getByRole("button", { name: "Add subscription" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("create_subscription", expect.objectContaining({ request: expect.objectContaining({ name: "Quarterly service", amount_minor: 2400, frequency: "custom:3:months" }) })));
  });
  it("shows a saved custom frequency and its monthly cost estimate", async () => {
    mocks.invoke.mockImplementation(async (command: string) => command === "list_subscriptions" ? [{ id: "custom", name: "Quarterly service", description: null, amount_minor: 2400, currency_code: "KES", frequency: "custom:3:months", next_due_date: "2026-12-01", account_id: null, category_id: null, status: "active", payment_count: 0, paid_minor: 0 }] : "created");
    render(<SubscriptionsPage />);
    expect(await screen.findByText(/Every 3 months/)).toBeInTheDocument();
    expect(screen.getByText(/8\.00/)).toBeInTheDocument();
  });
  it("links an already posted expense without creating another transaction", async () => {
    mocks.invoke.mockImplementation(async (command: string) => command === "list_subscriptions" ? [{ id: "sub", name: "Music", description: null, amount_minor: 999, currency_code: "KES", frequency: "monthly", next_due_date: "2026-11-20", account_id: null, category_id: null, status: "active", payment_count: 0, paid_minor: 0 }] : command === "list_eligible_subscription_payments" ? [{ transaction_id: "payment", amount_minor: 999, occurred_at: "2026-11-20T08:00:00Z", description: "Music" }] : undefined);
    render(<SubscriptionsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Link posted payment" }));
    fireEvent.change(await screen.findByLabelText("Posted payment for Music"), { target: { value: "payment" } });
    fireEvent.click(screen.getByRole("button", { name: "Link payment" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("link_subscription_payment", { subscriptionId: "sub", transactionId: "payment" }));
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_transaction", expect.anything());
  });
});
