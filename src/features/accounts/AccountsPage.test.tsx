import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AccountsPage from "./AccountsPage";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const activeAccount = {
  id: "active-1",
  name: "Everyday account",
  account_type: "bank",
  institution_name: "Example Bank",
  currency_code: "KES",
  notes: null,
  created_at: "2026-11-01T00:00:00Z",
  updated_at: "2026-11-01T00:00:00Z",
  archived_at: null,
};

const archivedAccount = {
  ...activeAccount,
  id: "archived-1",
  name: "Old account",
  archived_at: "2026-10-01T00:00:00Z",
};

describe("AccountsPage", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation((command: string) => {
      if (command === "list_active_accounts") return Promise.resolve([activeAccount]);
      if (command === "list_archived_accounts") return Promise.resolve([archivedAccount]);
      return Promise.resolve(0);
    });
  });

  it("switches between active and archived accounts with an accessible toggle state", async () => {
    render(<AccountsPage />);

    expect(await screen.findByText("Everyday account")).toBeInTheDocument();
    const archiveToggle = screen.getByRole("button", { name: "Archived accounts" });
    fireEvent.click(archiveToggle);

    expect(await screen.findByText("Old account")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Archived accounts" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Active accounts" })).toHaveAttribute("aria-pressed", "true");
  });
});
