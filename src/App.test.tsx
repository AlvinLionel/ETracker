import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({ theme: "system" }),
}));

vi.mock("./features/dashboard/DashboardPage", () => ({
  default: () => <div>Dashboard test content</div>,
}));

describe("application navigation shell", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn(() => ({
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      })),
    );
  });

  it("provides compact navigation and keeps the active page in sync", async () => {
    render(<App />);

    expect(await screen.findByText("Dashboard test content")).toBeInTheDocument();
    const now = new Date();
    expect(screen.getByText(`Calendar Q${Math.floor(now.getMonth() / 3) + 1} · ${now.getFullYear()}`)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reconcile statement" })).toHaveAttribute("title", "Open statement import reconciliation");
    expect(screen.getByRole("navigation", { name: "Primary navigation" })).toBeInTheDocument();
    const compactNavigation = screen.getByRole("navigation", { name: "Compact navigation" });
    const compactLabels = within(compactNavigation).getAllByRole("button").map((button) => button.textContent);
    expect(compactLabels.indexOf("Imports")).toBeLessThan(compactLabels.indexOf("Transactions"));
    expect(compactLabels.indexOf("Categories")).toBeLessThan(compactLabels.indexOf("Transactions"));
    fireEvent.click(within(compactNavigation).getByRole("button", { name: "Categories" }));
    expect(await screen.findByRole("heading", { name: "Categories", level: 1 })).toBeInTheDocument();
    fireEvent.click(within(compactNavigation).getByRole("button", { name: "Accounts" }));

    expect(screen.getByRole("heading", { name: "Account management" })).toBeInTheDocument();
    expect(within(compactNavigation).getByRole("button", { name: "Accounts" })).toHaveAttribute("aria-current", "page");
  });
});
