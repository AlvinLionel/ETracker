import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import CategoriesPage from "./CategoriesPage";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

describe("CategoriesPage", () => {
  beforeEach(() => {
    invoke.mockReset();
    let activeCategories = [{ id: "food", name: "Food", category_type: "expense", is_system: false }];
    invoke.mockImplementation((command: string, args?: { categoryType?: string; name?: string }) => {
      if (command === "list_active_categories") return Promise.resolve(activeCategories.filter((category) => category.category_type === args?.categoryType || (args?.categoryType === "expense" && category.category_type === "emergency")));
      if (command === "create_category") {
        const category = { id: "travel", name: args?.name ?? "Travel", category_type: args?.categoryType ?? "expense", is_system: false };
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

  it("creates an emergency category that is included with expense categories", async () => {
    render(<CategoriesPage />);
    fireEvent.change(screen.getByLabelText("Category name"), { target: { value: "Medical reserve" } });
    fireEvent.change(screen.getByLabelText("Category type"), { target: { value: "emergency" } });
    fireEvent.click(screen.getByRole("button", { name: "Add category" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("create_category", { name: "Medical reserve", categoryType: "emergency" }));
    expect(await screen.findByText("Emergency category created.")).toBeInTheDocument();
    expect(await screen.findByText("Medical reserve")).toBeInTheDocument();
  });
});
