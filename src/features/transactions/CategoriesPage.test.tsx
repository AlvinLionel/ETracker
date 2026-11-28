import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import CategoriesPage from "./CategoriesPage";

const invoke = vi.fn();
const confirmMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock("../../shared/confirmAction", () => ({ confirmAction: (...args: unknown[]) => confirmMock(...args) }));

describe("CategoriesPage", () => {
  beforeEach(() => {
    invoke.mockReset();
    confirmMock.mockReset().mockResolvedValue(true);
    let activeCategories = [{ id: "food", name: "Food", category_type: "expense", is_system: false }];
    invoke.mockImplementation((command: string, args?: { categoryType?: string; name?: string }) => {
      if (command === "list_import_category_rules") return Promise.resolve([]);
      if (command === "list_active_categories") return Promise.resolve(activeCategories.filter((category) => category.category_type === args?.categoryType || (args?.categoryType === "expense" && category.category_type === "emergency")));
      if (command === "create_category") {
        const category = { id: "travel", name: args?.name ?? "Travel", category_type: args?.categoryType ?? "expense", is_system: false };
        activeCategories = [...activeCategories, category];
        return Promise.resolve(category);
      }
      if (command === "save_import_category_rule") return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
  });

  it("creates a category that can be used to review spending", async () => {
    render(<CategoriesPage />);
    expect(await screen.findByText("Food")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Category name"), { target: { value: "Travel" } });
    fireEvent.change(screen.getByLabelText("Identifying phrases"), { target: { value: "bus, taxi" } });
    fireEvent.click(screen.getByRole("button", { name: "Add category" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("create_category", { name: "Travel", categoryType: "expense" }));
    expect(invoke).toHaveBeenCalledWith("save_import_category_rule", { categoryId: "travel", categoryName: "Travel", categoryType: "expense", matchTexts: ["bus", "taxi"] });
    expect(await screen.findByRole("status")).toHaveTextContent("Expense category created.");
    expect(await screen.findByText("Travel")).toBeInTheDocument();
  });

  it("creates an emergency category that is included with expense categories", async () => {
    render(<CategoriesPage />);
    fireEvent.change(screen.getByLabelText("Category name"), { target: { value: "Medical reserve" } });
    fireEvent.change(screen.getByLabelText("Identifying phrases"), { target: { value: "clinic, hospital" } });
    fireEvent.change(screen.getByLabelText("Category type"), { target: { value: "emergency" } });
    fireEvent.click(screen.getByRole("button", { name: "Add category" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("create_category", { name: "Medical reserve", categoryType: "emergency" }));
    expect(invoke).toHaveBeenCalledWith("save_import_category_rule", { categoryId: "travel", categoryName: "Medical reserve", categoryType: "expense", matchTexts: ["clinic", "hospital"] });
    expect(await screen.findByText("Emergency category created.")).toBeInTheDocument();
    expect(await screen.findByText("Medical reserve")).toBeInTheDocument();
  });

  it("offers archive when history prevents category deletion", async () => {
    let categories = [{ id: "food", name: "Food", category_type: "expense", is_system: false }];
    invoke.mockImplementation((command: string, args?: { categoryType?: string; id?: string }) => {
      if (command === "list_active_categories") return Promise.resolve(categories.filter((category) => category.category_type === args?.categoryType || (args?.categoryType === "expense" && category.category_type === "emergency")));
      if (command === "list_import_category_rules") return Promise.resolve([]);
      if (command === "delete_category") return Promise.reject("This category is used by existing records or plans. Archive it to preserve that history.");
      if (command === "archive_category") { categories = []; return Promise.resolve(); }
      return Promise.resolve(undefined);
    });
    render(<CategoriesPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Delete" }));
    fireEvent.click(await screen.findByRole("button", { name: "Archive instead" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("archive_category", { id: "food" }));
    expect(await screen.findByRole("status")).toHaveTextContent("Existing history and plans are preserved.");
  });
});
