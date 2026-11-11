import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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

  it("does not add foreign-currency balances to the KES portfolio total", async () => {
    const foreignAccount = { ...activeAccount, id: "foreign-1", name: "Dollar account", currency_code: "USD" };
    invoke.mockImplementation((command: string, args?: { id?: string }) => {
      if (command === "list_active_accounts") return Promise.resolve([activeAccount, foreignAccount]);
      if (command === "list_archived_accounts") return Promise.resolve([]);
      if (command === "get_account_balance") return Promise.resolve(args?.id === "active-1" ? 12500 : 9000000);
      return Promise.resolve(0);
    });

    render(<AccountsPage />);

    const portfolioCard = screen.getByText("Total portfolio assets").closest("article");
    await waitFor(() => expect(portfolioCard).toHaveTextContent(/KES\s*125\.00/));
    expect(portfolioCard).not.toHaveTextContent("90,125.00");
    expect(portfolioCard).toHaveTextContent("KES accounts only; other currencies shown per account");
  });

  it("reviews account-list rows, skips duplicates and creates only safe account fields", async () => {
    const createdAccount = { ...activeAccount, id: "new-1", name: "Travel wallet", account_type: "mobile_money", institution_name: "M-Pesa" };
    invoke.mockImplementation((command: string, args?: { request?: Record<string, unknown> }) => {
      if (command === "list_active_accounts") return Promise.resolve([activeAccount]);
      if (command === "list_archived_accounts") return Promise.resolve([archivedAccount]);
      if (command === "csv_rows") return Promise.resolve([
        ["Account Name", "Account Number", "Type", "Institution", "Opening Balance", "Opening Date", "Currency"],
        ["Everyday account", "123456789012", "bank", "Example Bank", "", "", "KES"],
        ["Travel wallet", "987654321098", "mobile money", "M-Pesa", "12.50", "01/11/2026", "KES"],
        ["Foreign account", "555", "bank", "Example Bank", "2", "2026-11-01", "USD"],
      ]);
      if (command === "create_account") {
        expect(args?.request).toEqual({
          name: "Travel wallet", account_type: "mobile_money", institution_name: "M-Pesa", notes: null,
          opening_balance: { amount_minor: 1250, occurred_at: "2026-11-01T00:00:00Z" },
        });
        expect(JSON.stringify(args?.request)).not.toContain("987654321098");
        return Promise.resolve(createdAccount);
      }
      return Promise.resolve(undefined);
    });

    render(<AccountsPage />);
    expect(await screen.findByText("Everyday account")).toBeInTheDocument();
    const file = new File(["account list"], "accounts.csv", { type: "text/csv" });
    fireEvent.change(screen.getByLabelText("Account list file"), { target: { files: [file] } });

    expect(await screen.findByText(/already exists or appears earlier/)).toBeInTheDocument();
    expect(screen.getByText(/Currency USD is unsupported/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Create selected accounts" }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith("create_account", expect.anything()));
    expect(await screen.findByRole("status")).toHaveTextContent("1 account imported");
    expect(invoke.mock.calls.filter(([command]) => command === "create_account")).toHaveLength(1);
  });

  it("reports a per-row creation failure without marking the account imported", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_active_accounts") return Promise.resolve([]);
      if (command === "list_archived_accounts") return Promise.resolve([]);
      if (command === "csv_rows") return Promise.resolve([["Account Name", "Type"], ["Offline account", "bank"]]);
      if (command === "create_account") return Promise.reject(new Error("Database unavailable"));
      return Promise.resolve(undefined);
    });
    render(<AccountsPage />);
    fireEvent.change(await screen.findByLabelText("Account list file"), { target: { files: [new File(["account list"], "accounts.csv", { type: "text/csv" })] } });
    fireEvent.click(await screen.findByRole("button", { name: "Create selected accounts" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Row 2: Database unavailable");
    expect(screen.getByRole("status")).toHaveTextContent("0 accounts imported; 1 need attention");
    expect(screen.getByText("Database unavailable")).toBeInTheDocument();
  });

  it("rejects oversized account files before parsing them", async () => {
    render(<AccountsPage />);
    const file = new File(["small fixture"], "large-accounts.csv", { type: "text/csv" });
    Object.defineProperty(file, "size", { value: 25 * 1024 * 1024 + 1 });
    fireEvent.change(await screen.findByLabelText("Account list file"), { target: { files: [file] } });

    expect(await screen.findByRole("alert")).toHaveTextContent(/25 MiB or smaller/);
    expect(invoke).not.toHaveBeenCalledWith("csv_rows", expect.anything());
  });

  it("loads a selected Excel worksheet into the account review table", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_active_accounts") return Promise.resolve([]);
      if (command === "list_archived_accounts") return Promise.resolve([]);
      if (command === "xlsx_sheet_names") return Promise.resolve(["Accounts", "Notes"]);
      if (command === "xlsx_rows") return Promise.resolve([["Account Name", "Type"], ["Mobile wallet", "mobile money"]]);
      return Promise.resolve(undefined);
    });
    render(<AccountsPage />);
    const file = new File([new Uint8Array([1, 2, 3])], "accounts.xlsx", { type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" });
    fireEvent.change(await screen.findByLabelText("Account list file"), { target: { files: [file] } });
    expect(await screen.findByLabelText("Account worksheet")).toHaveValue("Accounts");
    fireEvent.click(screen.getByRole("button", { name: "Load worksheet" }));

    expect(await screen.findByText("Mobile wallet")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("xlsx_rows", { contents: [1, 2, 3], sheetName: "Accounts", headerRow: 1 });
  });
});
