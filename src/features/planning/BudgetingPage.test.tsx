import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import BudgetingPage from "./BudgetingPage";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), categories: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../transactions/api/transactions", () => ({ listActiveCategories: mocks.categories }));

describe("BudgetingPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.categories.mockImplementation(async (type: string) => type === "expense" ? [{ id: "food", name: "Food", category_type: "expense" }] : []);
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "list_budget_templates") return [{ id: "template", name: "Household", currency_code: "KES" }];
      if (command === "list_budget_periods") return [{ id: "budget", name: "Household — 2026-11", currency_code: "KES", starts_on: "2026-11-01", ends_on: "2026-11-30" }];
      if (command === "get_budget_period_items") return [{ id: "line", item_type: "expense", category_id: "food", label: "Food", planned_minor: 25000, actual_minor: 8000 }];
      if (command === "get_budget_period_totals") return { actual_income_minor: 30000, actual_expense_minor: 8000, actual_savings_minor: 5000, unallocated_minor: 9999 };
      if (command === "list_budget_template_items") return [{ item_type: "expense", category_id: "food", label: "Food", amount_minor: 25000 }];
      return "created";
    });
  });

  it("shows actual spending against the copied monthly plan", async () => {
    render(<BudgetingPage />);
    expect(await screen.findByRole("heading", { name: "Monthly plan worksheet" })).toBeInTheDocument();
    expect(screen.getByRole("table", { name: /Monthly financial plan with posted actuals/ })).toBeInTheDocument();
    expect(await screen.findByText("Food", { selector: "td" })).toBeInTheDocument();
    expect(screen.getAllByText(/80\.00/)).toHaveLength(2);
    expect(screen.getByText(/170\.00/)).toBeInTheDocument();
    expect(screen.getByText("Unallocated actual")).toBeInTheDocument();
    expect(screen.getByText(/99\.99/)).toBeInTheDocument();
  });

  it("creates an independent monthly budget from a template", async () => {
    render(<BudgetingPage />);
    fireEvent.change(await screen.findByLabelText("Budget period"), { target: { value: "" } });
    fireEvent.change(screen.getByLabelText("Template"), { target: { value: "template" } });
    fireEvent.change(screen.getByLabelText("Month"), { target: { value: "2026-12" } });
    fireEvent.click(screen.getByRole("button", { name: "Create monthly budget" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("create_budget_period", { templateId: "template", month: "2026-12" }));
    expect(await screen.findByRole("status")).toHaveTextContent("independent plan");
  });

  it("edits a monthly snapshot independently", async () => {
    render(<BudgetingPage />);
    expect(await screen.findByText("Food", { selector: "td" })).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Edit this monthly plan" }));
    fireEvent.change(screen.getByLabelText("Plan amount 1"), { target: { value: "300.00" } });
    fireEvent.click(screen.getByRole("button", { name: "Save monthly plan" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("update_budget_period_items", { budgetId: "budget", items: [{ item_type: "expense", category_id: "food", label: "Food", amount_minor: 30000 }] }));
  });

  it("edits a template without changing existing month snapshots", async () => {
    render(<BudgetingPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Edit selected template" }));
    fireEvent.change(await screen.findByLabelText("Template name"), { target: { value: "Updated household" } });
    fireEvent.click(screen.getByRole("button", { name: "Save template changes" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("update_budget_template", { templateId: "template", name: "Updated household", items: [{ item_type: "expense", category_id: "food", label: "Food", amount_minor: 25000 }] }));
  });
});
