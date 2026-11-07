import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import CategoriesPage from "./CategoriesPage";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

describe("CategoriesPage", () => {
  beforeEach(() => {
    invoke.mockReset();
    let activeCategories = [{ id: "food", name: "Food", category_type: "expense", is_system: false }];
    invoke.mockImplementation((command: string, args?: { categoryType?: string }) => {
      if (command === "list_active_categories") return Promise.resolve(activeCategories.filter((category) => category.category_type === args?.categoryType));
      if (command === "create_category") {
        const category = { id: "travel", name: "Travel", category_type: "expense", is_system: false };
        activeCategories = [...activeCategories, category];
        return Promise.resolve(category);
      }
      return Promise.resolve(undefined);
    });
  });

  it("creates a category that can be used to review spending", async () => {
    render(<CategoriesPage />);
    expect(await screen.findByText("Food")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Category name"), { target: { value: "Travel" } });
    fireEvent.click(screen.getByRole("button", { name: "Add category" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("create_category", { name: "Travel", categoryType: "expense" }));
    expect(await screen.findByRole("status")).toHaveTextContent("Expense category created.");
    expect(await screen.findByText("Travel")).toBeInTheDocument();
  });
});
