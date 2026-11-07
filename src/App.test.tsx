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
    expect(screen.getByRole("navigation", { name: "Primary navigation" })).toBeInTheDocument();
    const compactNavigation = screen.getByRole("navigation", { name: "Compact navigation" });
    fireEvent.click(within(compactNavigation).getByRole("button", { name: "Accounts" }));

    expect(screen.getByRole("heading", { name: "Account management" })).toBeInTheDocument();
    expect(within(compactNavigation).getByRole("button", { name: "Accounts" })).toHaveAttribute("aria-current", "page");
  });
});
