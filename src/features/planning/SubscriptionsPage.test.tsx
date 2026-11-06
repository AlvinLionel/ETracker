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
});
