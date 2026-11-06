import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import ImportsPage from "./ImportsPage";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), accounts: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../accounts/api/accounts", () => ({ listActiveAccounts: mocks.accounts }));

describe("ImportsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.accounts.mockResolvedValue([{ id: "a", name: "Main", currency_code: "KES" }]);
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Date", "Amount", "Memo"]);
      if (command === "create_import_batch") return Promise.resolve("batch-1");
      if (command === "stage_csv_import") return Promise.resolve(2);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      if (command === "list_active_categories") return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
  });

  it("maps a CSV file into a persistent import workspace", async () => {
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    const file = new File(["Date,Amount,Memo\n2026-11-01,10.00,Shop"], "statement.csv", { type: "text/csv" });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    await screen.findByLabelText("Date column");
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("stage_csv_import", expect.objectContaining({
      request: expect.objectContaining({ batch_id: "batch-1", date_column: "Date", amount_column: "Amount" }),
    })));
    expect(await screen.findByRole("status")).toHaveTextContent("2 staged rows");
  });

  it("selects an XLSX worksheet and stages mapped rows", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "xlsx_sheet_names") return Promise.resolve(["Transactions", "Summary"]);
      if (command === "xlsx_headers") return Promise.resolve(["Date", "Amount", "Memo"]);
      if (command === "create_import_batch") return Promise.resolve("batch-xlsx");
      if (command === "stage_xlsx_import") return Promise.resolve(1);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    const file = new File([new Uint8Array([1, 2, 3])], "statement.xlsx", { type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    await screen.findByLabelText("Worksheet");
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("stage_xlsx_import", expect.objectContaining({
      request: expect.objectContaining({ batch_id: "batch-xlsx", sheet_name: "Transactions", header_row: 1, date_column: "Date", amount_column: "Amount" }),
    })));
    expect(await screen.findByRole("status")).toHaveTextContent("1 staged rows");
  });

  it("shows the saved processing logs for a workspace", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([{ id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" }]);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([{ id: "log-1", timestamp: "2026-11-06", stage: "parse", severity: "INFO", message: "Staged 2 CSV rows", record_id: null, corrective_action: null }]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Records" }));
    fireEvent.click(screen.getByRole("button", { name: "Logs" }));
    expect(await screen.findByText("Staged 2 CSV rows")).toBeInTheDocument();
  });

  it("submits statement balances for reconciliation", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([{ id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" }]);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      if (command === "reconcile_import_batch") return Promise.resolve({ opening_balance_minor: 10000, closing_balance_minor: 8750, expected_closing_balance_minor: 8750, difference_minor: 0, outcome: "reconciled", explanation: "Balances match." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Records" }));
    fireEvent.change(screen.getByLabelText("Opening balance"), { target: { value: "100.00" } });
    fireEvent.change(screen.getByLabelText("Closing balance"), { target: { value: "87.50" } });
    fireEvent.click(screen.getByRole("button", { name: "Check balances" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("reconcile_import_batch", { request: { batch_id: "batch-1", opening_balance_minor: 10000, closing_balance_minor: 8750 } }));
  });

  it("saves confirmed split category allocations for a staged expense", async () => {
    const batch = { id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" };
    const record = { id: "row-1", source_row_reference: "2", original_date: "2026-11-05", normalized_date: "2026-11-05T00:00:00Z", original_description: "Shop", original_amount: "12.50", normalized_amount_minor: 1250, transaction_type: "expense", validation_status: "valid", duplicate_status: "unmatched", review_status: "pending", allocations: [] };
    mocks.invoke.mockImplementation((command: string, args?: { categoryType?: string }) => {
      if (command === "list_import_batches") return Promise.resolve([batch]);
      if (command === "list_staged_import_records") return Promise.resolve([record]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      if (command === "list_active_categories") return Promise.resolve(args?.categoryType === "expense" ? [{ id: "food", name: "Food", category_type: "expense" }, { id: "home", name: "Home", category_type: "expense" }] : []);
      return Promise.resolve(undefined);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Records" }));
    fireEvent.click(await screen.findByRole("button", { name: "Categories" }));
    fireEvent.click(screen.getByRole("button", { name: "Add split" }));
    fireEvent.click(screen.getByRole("button", { name: "Add split" }));
    const selectors = screen.getAllByLabelText("Category");
    fireEvent.change(selectors[0], { target: { value: "food" } });
    fireEvent.change(selectors[1], { target: { value: "home" } });
    const amounts = screen.getAllByLabelText("Amount");
    fireEvent.change(amounts[amounts.length - 2], { target: { value: "7.00" } });
    fireEvent.change(amounts[amounts.length - 1], { target: { value: "5.50" } });
    fireEvent.click(screen.getByRole("button", { name: "Save categories" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("set_import_record_allocations", { request: { record_id: "row-1", allocations: [{ category_id: "food", amount_minor: 700 }, { category_id: "home", amount_minor: 550 }] } }));
  });
});
