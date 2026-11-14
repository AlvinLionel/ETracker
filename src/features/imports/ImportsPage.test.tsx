import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import ImportsPage from "./ImportsPage";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), accounts: vi.fn(), pdfText: vi.fn(), confirmAction: vi.fn(), saveDialog: vi.fn(), writeFile: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: mocks.saveDialog }));
vi.mock("@tauri-apps/plugin-fs", () => ({ writeFile: mocks.writeFile }));
vi.mock("../accounts/api/accounts", () => ({ listActiveAccounts: mocks.accounts }));
vi.mock("./pdfText", () => ({ extractPdfTextPages: mocks.pdfText }));
vi.mock("../../shared/confirmAction", () => ({ confirmAction: mocks.confirmAction }));

describe("ImportsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.pdfText.mockResolvedValue({ pages: ["M-PESA statement\n2026-11-01 Grocery shop 12.50"], ocrPageNumbers: [] });
    mocks.confirmAction.mockResolvedValue(true);
    mocks.saveDialog.mockResolvedValue("C:\\Users\\Test\\statement.csv");
    mocks.writeFile.mockResolvedValue(undefined);
    mocks.accounts.mockResolvedValue([{ id: "a", name: "Main", currency_code: "KES" }]);
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "list_import_category_rules") return Promise.resolve([]);
      if (command === "list_import_category_suggestions") return Promise.resolve([]);
      if (command === "list_active_categories") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Date", "Amount", "Memo"]);
      if (command === "csv_rows") return Promise.resolve([["Date", "Amount", "Memo"], ["2026-11-01", "10.00", "Shop"]]);
      if (command === "create_import_batch") return Promise.resolve("batch-1");
      if (command === "stage_csv_import") return Promise.resolve(2);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      if (command === "list_active_categories") return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
  });

  it("uses a single custom file-picker action and keeps the selected filename visible", async () => {
    render(<ImportsPage />);
    expect(screen.getByRole("button", { name: "Choose file" })).toBeInTheDocument();
    expect(screen.queryByText("Local processing · KES")).not.toBeInTheDocument();
    expect(screen.queryByText("Choose statement file")).not.toBeInTheDocument();
    const file = new File(["Date,Amount,Memo\n2026-11-01,10.00,Shop"], "November statement.csv", { type: "text/csv" });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    expect(await screen.findByText("Selected file:", { exact: false })).toHaveTextContent("November statement.csv");
  });

  it("maps a CSV file into a persistent import workspace", async () => {
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    const file = new File(["Date,Amount,Memo\n2026-11-01,10.00,Shop"], "statement.csv", { type: "text/csv" });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    await screen.findByLabelText("Date column");
    expect(screen.getByLabelText("Date column")).toHaveValue("Date");
    expect(screen.getByLabelText("Amount column")).toHaveValue("Amount");
    fireEvent.change(screen.getByLabelText("Date format"), { target: { value: "day_month_year_slash" } });
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("stage_csv_import", expect.objectContaining({
      request: expect.objectContaining({ batch_id: "batch-1", date_column: "Date", amount_column: "Amount", positive_amount_type: "unknown", date_format: "day_month_year_slash" }),
    })));
    expect(mocks.invoke).toHaveBeenCalledWith("create_import_batch", { request: expect.objectContaining({ source_file_contents: expect.any(Array) }) });
    expect(await screen.findByRole("status")).toHaveTextContent("2 staged rows");
  });

  it("opens a saved import's audit log directly", async () => {
    const batch = { id: "batch-audit", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "2026-11-06", revision_of_batch_id: null };
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([batch]);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([{ id: "log-1", timestamp: "2026-11-06T10:00:00Z", stage: "parse", severity: "info", message: "Rows staged", record_id: null, corrective_action: null }]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);

    fireEvent.click(await screen.findByRole("button", { name: "Audit log" }));

    const workspaceViews = screen.getByRole("navigation", { name: "Import workspace views" });
    await waitFor(() => expect(within(workspaceViews).getByRole("button", { name: "Logs" })).toHaveAttribute("aria-current", "page"));
    expect(await screen.findByText("Rows staged")).toBeInTheDocument();
  });

  it("applies an accepted repeated-description category suggestion", async () => {
    const suggestion = { category_name: "Green Valley Hotel", category_type: "expense", match_text: "green valley hotel", occurrence_count: 4 };
    const applied = { ...suggestion, categorized_transaction_count: 4 };
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "list_import_category_rules") return Promise.resolve([]);
      if (command === "list_import_category_suggestions") return Promise.resolve([suggestion]);
      if (command === "apply_import_category_suggestion") return Promise.resolve(applied);
      if (command === "list_active_categories") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);

    expect(await screen.findByText(/appears in 4 uncategorized imported transactions/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Add category & categorize matches" }));

    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("apply_import_category_suggestion", {
      categoryType: "expense",
      matchText: "green valley hotel",
    }));
    expect(await screen.findByRole("status")).toHaveTextContent("categorized 4 existing transactions");
    expect(screen.getByRole("status")).toHaveTextContent("Future matching imports will use it automatically");
  });

  it("rejects an oversized statement before reading or sending it", async () => {
    render(<ImportsPage />);
    const file = new File(["small fixture"], "large-statement.csv", { type: "text/csv" });
    Object.defineProperty(file, "size", { value: 25 * 1024 * 1024 + 1 });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/25 MiB or smaller/);
    expect(mocks.invoke).not.toHaveBeenCalledWith("csv_headers", expect.anything());
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("rejects unsupported file extensions before creating an import workspace", async () => {
    render(<ImportsPage />);
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["notes"], "notes.docx")] } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/file type is not supported/i);
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("rejects a renamed non-PDF document before extraction", async () => {
    render(<ImportsPage />);
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["meeting notes"], "notes.pdf", { type: "application/pdf" })] } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/contents do not match its/i);
    expect(mocks.pdfText).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("rejects a non-financial CSV without creating a retained import workspace", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Title", "Owner", "Status"]);
      if (command === "csv_rows") return Promise.resolve([["Title", "Owner", "Status"], ["Planning notes", "Alex", "Complete"]]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["Title,Owner,Status\nPlanning notes,Alex,Complete"], "notes.csv", { type: "text/csv" })] } });
    await screen.findByLabelText("Date column");
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/does not contain transaction rows/i);
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("rejects an unrelated PDF before retaining it as a statement batch", async () => {
    mocks.pdfText.mockResolvedValue({ pages: ["Meeting agenda", "Welcome and discussion points"], ocrPageNumbers: [] });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["%PDF-1.5 fixture"], "agenda.pdf", { type: "application/pdf" })] } });
    await screen.findByText("agenda.pdf");
    expect(screen.queryByText(/Selectable text is extracted directly/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/does not appear to contain bank or mobile-money statement activity/i);
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("detects a budget-plan spreadsheet and prevents it entering transaction history", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Category", "Planned Amount", "Budget Month"]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["Category,Planned Amount"], "monthly-budget.csv", { type: "text/csv" })] } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/look like a budget plan/i);
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("classifies a dated budget plan before statement fields", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Date", "Category", "Planned Amount"]);
      return Promise.resolve([]);
    });
    render(<ImportsPage navigate={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["budget"], "dated-plan.csv", { type: "text/csv" })] } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/look like a budget plan/i);
    expect(screen.getByRole("button", { name: "Open Budgeting" })).toBeInTheDocument();
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("detects an account list and prevents it entering transaction history", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Account Name", "Account Number", "Current Balance"]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["Account Name,Account Number"], "accounts.csv", { type: "text/csv" })] } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/look like an account list/i);
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("recognizes account lists that include a dated balance without misclassifying statements", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Account Name", "Account Type", "Institution", "Opening Balance", "Opening Date", "Currency"]);
      return Promise.resolve([]);
    });
    render(<ImportsPage navigate={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["account list"], "accounts.csv", { type: "text/csv" })] } });
    expect(await screen.findByRole("alert")).toHaveTextContent(/select the file there to review and import/i);
    expect(screen.getByRole("button", { name: "Open Accounts" })).toBeInTheDocument();
    expect(mocks.invoke).not.toHaveBeenCalledWith("create_import_batch", expect.anything());
  });

  it("keeps dated transaction statements with an account name in statement flow", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Account Name", "Date", "Amount", "Memo"]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["statement"], "statement.csv", { type: "text/csv" })] } });
    expect(await screen.findByLabelText("Date column")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("selects an XLSX worksheet and stages mapped rows", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "xlsx_sheet_names") return Promise.resolve(["Transactions", "Summary"]);
      if (command === "xlsx_headers") return Promise.resolve(["Date", "Amount", "Memo"]);
      if (command === "xlsx_rows") return Promise.resolve([["Date", "Amount", "Memo"], ["2026-11-01", "10.00", "Shop"]]);
      if (command === "create_import_batch") return Promise.resolve("batch-xlsx");
      if (command === "stage_xlsx_import") return Promise.resolve(1);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    const file = new File([new Uint8Array([0x50, 0x4b, 0x03, 0x04])], "statement.xlsx", { type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    await screen.findByLabelText("Worksheet");
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("stage_xlsx_import", expect.objectContaining({
      request: expect.objectContaining({ batch_id: "batch-xlsx", sheet_name: "Transactions", header_row: 1, date_column: "Date", amount_column: "Amount" }),
    })));
    expect(await screen.findByRole("status")).toHaveTextContent("1 staged rows");
  });

  it("stages extracted PDF text lines for manual correction", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "create_import_batch") return Promise.resolve("pdf-batch");
      if (command === "stage_pdf_extracted_text") return Promise.resolve(1);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    const file = new File(["%PDF-1.5 fixture"], "statement.pdf", { type: "application/pdf" });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    await screen.findByText("statement.pdf");
    expect(screen.queryByText(/Selectable text is extracted directly/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("stage_pdf_extracted_text", { batchId: "pdf-batch", pages: ["M-PESA statement\n2026-11-01 Grocery shop 12.50"] }));
    expect(await screen.findByRole("status")).toHaveTextContent("1 PDF text lines for manual review");
  });

  it("stages only detected PDF table rows and reports the mapped statement headers", async () => {
    mocks.pdfText.mockResolvedValue({
      pages: ["M-PESA statement\nCompletion Time Details Paid In Withdrawn Balance\n2026-11-01 10:00:00 Shop 25.00"],
      ocrPageNumbers: [],
      table: {
        headers: ["Completion Time", "Details", "Paid In", "Withdrawn", "Balance"],
        rows: [{ pageNumber: 1, date: "2026-11-01 10:00:00", description: "Shop", amount: "-25.00", transactionType: "expense" }],
      },
    });
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "create_import_batch") return Promise.resolve("pdf-table-batch");
      if (command === "stage_pdf_table_rows") return Promise.resolve(1);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File(["%PDF-1.5 fixture"], "statement.pdf", { type: "application/pdf" })] } });
    await screen.findByText("statement.pdf");
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));

    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("stage_pdf_table_rows", {
      batchId: "pdf-table-batch",
      pages: ["2026-11-01 10:00:00 Shop -25.00"],
      rowTransactionTypes: ["expense"],
    }));
    expect(await screen.findByRole("status")).toHaveTextContent("Detected statement columns: Completion Time, Details, Paid In, Withdrawn, Balance");
  });

  it("keeps an encrypted PDF import open until its password unlocks extraction", async () => {
    mocks.pdfText.mockRejectedValueOnce("password-required").mockResolvedValueOnce({ pages: ["M-PESA statement\n2026-11-01 Grocery shop 12.50"], ocrPageNumbers: [] });
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "create_import_batch") return Promise.resolve("protected-batch");
      if (command === "stage_pdf_extracted_text") return Promise.resolve(1);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    const file = new File(["%PDF-1.5 protected fixture"], "protected.pdf", { type: "application/pdf" });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    await waitFor(() => expect(screen.getByRole("button", { name: "Create import workspace" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));

    expect(await screen.findByLabelText("PDF password")).toBeInTheDocument();
    expect(screen.queryByText(/The password is used only to unlock this file/)).not.toBeInTheDocument();
    expect(await screen.findByRole("status")).toHaveTextContent(/Enter its password below to continue extraction/);
    fireEvent.change(screen.getByLabelText("PDF password"), { target: { value: "statement-secret" } });
    fireEvent.click(screen.getByRole("button", { name: "Unlock PDF and continue" }));

    await waitFor(() => expect(mocks.pdfText.mock.calls[mocks.pdfText.mock.calls.length - 1]?.slice(0, 2)).toEqual([expect.any(Uint8Array), "statement-secret"]));
    expect(mocks.invoke).toHaveBeenCalledWith("stage_pdf_extracted_text", { batchId: "protected-batch", pages: ["M-PESA statement\n2026-11-01 Grocery shop 12.50"] });
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "create_import_batch")).toHaveLength(1);
    expect(await screen.findByRole("status")).toHaveTextContent("1 PDF text lines for manual review");
  });

  it("compares a revised import with a selected finalized batch", async () => {
    const previous = { id: "old-batch", account_id: "a", source_file_name: "original.csv", source_format: "csv", status: "finalized", reconciliation_status: "reconciled", created_at: "2026-10-01", revision_of_batch_id: null };
    const revised = { id: "new-batch", account_id: "a", source_file_name: "revised.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "2026-11-06", revision_of_batch_id: "old-batch" };
    const change = { id: "change-1", change_type: "modified", resolution: null, original_record_id: "old-row", original_date: "2026-11-01", original_source_row_reference: "2", original_amount_minor: 1250, original_type: "expense", original_description: "Groceries", revised_record_id: "new-row", revised_date: "2026-11-01", revised_source_row_reference: "2", revised_amount_minor: 1500, revised_type: "expense", revised_description: "Groceries", manual_conflict: false };
    let batchReads = 0;
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve(++batchReads === 1 ? [previous] : [revised, previous]);
      if (command === "csv_headers") return Promise.resolve(["Date", "Amount", "Memo"]);
      if (command === "csv_rows") return Promise.resolve([["Date", "Amount", "Memo"], ["2026-11-01", "15.00", "Groceries"]]);
      if (command === "create_import_batch") return Promise.resolve("new-batch");
      if (command === "stage_csv_import") return Promise.resolve(1);
      if (command === "compare_import_revision" || command === "list_import_revision_changes") return Promise.resolve([change]);
      if (command === "set_import_revision_resolution") return Promise.resolve(undefined);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    fireEvent.change(await screen.findByLabelText("Compare with a previous statement", { exact: false }), { target: { value: "old-batch" } });
    const file = new File(["Date,Amount,Memo\n2026-11-01,15.00,Groceries"], "revised.csv", { type: "text/csv" });
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [file] } });
    await screen.findByLabelText("Date column");
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("compare_import_revision", { batchId: "new-batch" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("create_import_batch", { request: expect.objectContaining({ revision_of_batch_id: "old-batch" }) }));
    expect(await screen.findByRole("heading", { name: "Revised statement comparison" })).toBeInTheDocument();
    fireEvent.click((await screen.findAllByRole("button", { name: "Records" }))[0]);
    expect(await screen.findByRole("heading", { name: "Revised statement comparison" })).toBeInTheDocument();
    expect(screen.getByText("modified")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Review change change-1"), { target: { value: "use_revised" } });
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("set_import_revision_resolution", { request: { change_id: "change-1", resolution: "use_revised" } }));
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
    expect(await screen.findByText(/Create category rules, import a statement/)).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Records" }));
    expect(screen.getByRole("navigation", { name: "Import workspace views" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Logs" }));
    expect(await screen.findByText("Staged 2 CSV rows")).toBeInTheDocument();
  });

  it("shows and pre-fills the corrected normalized description", async () => {
    const batch = { id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" };
    const record = { id: "row-1", source_row_reference: "2", original_date: "2026-11-05", normalized_date: "2026-11-05T00:00:00Z", original_description: "Raw statement memo", normalized_description: "Corrected merchant", original_amount: "12.50", normalized_amount_minor: 1250, transaction_type: "expense", validation_status: "valid", duplicate_status: "unmatched", review_status: "corrected", allocations: [] };
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([batch]);
      if (command === "list_staged_import_records") return Promise.resolve([record]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Records" }));
    expect(await screen.findByText("Corrected merchant")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Correct" }));
    expect(screen.getByLabelText("Description")).toHaveValue("Corrected merchant");
    fireEvent.click(screen.getByRole("button", { name: "Save correction" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("correct_import_record", {
      request: { record_id: "row-1", normalized_date: "2026-11-05T00:00:00Z", amount_minor: 1250, transaction_type: "expense", description: "Corrected merchant" },
    }));
  });

  it("downloads the retained original statement file", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([{ id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" }]);
      if (command === "get_import_source_file") return Promise.resolve([65, 66, 67]);
      if (command === "list_staged_import_records") return Promise.resolve([]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Download source" }));
    await waitFor(() => expect(mocks.writeFile).toHaveBeenCalledWith("C:\\Users\\Test\\statement.csv", new Uint8Array([65, 66, 67])));
    expect(mocks.invoke).toHaveBeenCalledWith("get_import_source_file", { batchId: "batch-1" });
    expect(mocks.saveDialog).toHaveBeenCalledWith({
      defaultPath: "statement.csv",
      filters: [{ name: "Statement file", extensions: ["csv"] }],
    });
    expect(await screen.findByRole("status")).toHaveTextContent("Original statement saved to the selected location.");
  });

  it("does not write a source file when the save dialog is cancelled", async () => {
    mocks.saveDialog.mockResolvedValue(null);
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([{ id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" }]);
      if (command === "get_import_source_file") return Promise.resolve([65, 66, 67]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Download source" }));
    expect(await screen.findByRole("status")).toHaveTextContent("Source file save cancelled");
    expect(mocks.writeFile).not.toHaveBeenCalled();
  });

  it("requires confirmation before discarding an unfinished workspace", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([{ id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" }]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Discard" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("discard_import_batch", { batchId: "batch-1" }));
    expect(mocks.confirmAction).toHaveBeenCalledWith(expect.stringContaining("90-day retention period"));
  });

  it("imports eligible rows together after one explicit confirmation", async () => {
    const batch = { id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now", revision_of_batch_id: null };
    const record = { id: "row-1", source_row_reference: "2", original_date: "2026-11-05", normalized_date: "2026-11-05T00:00:00Z", original_description: "Shop", normalized_description: "Shop", original_amount: "12.50", normalized_amount_minor: 1250, transaction_type: "expense", validation_status: "valid", duplicate_status: "unmatched", review_status: "pending", allocations: [] };
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([batch]);
      if (command === "list_staged_import_records") return Promise.resolve([record]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      if (command === "finalize_import_batch") return Promise.resolve(1);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Records" }));
    const reviewToggle = await screen.findByText(/Review rows \(optional\)/);
    expect(reviewToggle.closest("details")).not.toHaveAttribute("open");
    fireEvent.click(screen.getByRole("button", { name: "Import now" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("finalize_import_batch", { batchId: "batch-1" }));
    expect(mocks.confirmAction).toHaveBeenCalledWith(expect.stringContaining("Import 1 eligible transaction"));
  });

  it("explains how ready, duplicate, and review rows affect the import count", async () => {
    const batch = { id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now", revision_of_batch_id: null };
    const records = [
      { id: "ready", source_row_reference: "2", original_date: "2026-11-05", normalized_date: "2026-11-05T00:00:00Z", original_description: "Shop", normalized_description: "Shop", original_amount: "12.50", normalized_amount_minor: 1250, transaction_type: "expense", validation_status: "valid", duplicate_status: "unmatched", review_status: "pending", allocations: [] },
      { id: "duplicate", source_row_reference: "3", original_date: "2026-11-05", normalized_date: "2026-11-05T00:00:00Z", original_description: "Shop", normalized_description: "Shop", original_amount: "12.50", normalized_amount_minor: 1250, transaction_type: "expense", validation_status: "valid", duplicate_status: "probable", review_status: "pending", allocations: [] },
      { id: "unclear", source_row_reference: "4", original_date: "bad date", normalized_date: null, original_description: "Unknown", normalized_description: "Unknown", original_amount: "12.50", normalized_amount_minor: 1250, transaction_type: null, validation_status: "uncertain", duplicate_status: "unmatched", review_status: "pending", allocations: [] },
    ];
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([batch]);
      if (command === "list_staged_import_records") return Promise.resolve(records);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "get_import_reconciliation") return Promise.resolve({ opening_balance_minor: null, closing_balance_minor: null, expected_closing_balance_minor: null, difference_minor: null, outcome: "unavailable", explanation: "No statement balances provided." });
      return Promise.resolve([]);
    });
    render(<ImportsPage />);

    fireEvent.click(await screen.findByRole("button", { name: "Records" }));

    const summary = await screen.findByLabelText("Import row summary");
    expect(summary).toHaveTextContent("1 ready to import");
    expect(summary).toHaveTextContent("1 of those will be uncategorized");
    expect(summary).toHaveTextContent("1 possible duplicate is withheld");
    expect(summary).toHaveTextContent("1 row needs date, amount, or direction review");
  });

  it("shows a clear error if the discard confirmation dialog fails", async () => {
    mocks.confirmAction.mockRejectedValueOnce(new Error("dialog permission denied"));
    mocks.invoke.mockImplementation((command: string) => command === "list_import_batches"
      ? Promise.resolve([{ id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" }])
      : Promise.resolve([]));
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Discard" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("The confirmation dialog could not be opened");
    expect(mocks.invoke).not.toHaveBeenCalledWith("discard_import_batch", expect.anything());
  });

  it("keeps an unfinished workspace when discard is declined", async () => {
    mocks.confirmAction.mockResolvedValue(false);
    mocks.invoke.mockImplementation((command: string) => command === "list_import_batches"
      ? Promise.resolve([{ id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" }])
      : Promise.resolve([]));
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Discard" }));
    await waitFor(() => expect(mocks.confirmAction).toHaveBeenCalled());
    expect(mocks.invoke).not.toHaveBeenCalledWith("discard_import_batch", expect.anything());
  });

  it("imports without requiring a statement balance check", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([{ id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" }]);
      if (command === "list_staged_import_records") return Promise.resolve([{ id: "row-1", source_row_reference: "2", original_date: "2026-11-05", normalized_date: "2026-11-05T00:00:00Z", original_description: "Shop", normalized_description: "Shop", original_amount: "12.50", normalized_amount_minor: 1250, transaction_type: "expense", validation_status: "valid", duplicate_status: "unmatched", review_status: "pending", allocations: [] }]);
      if (command === "list_import_logs") return Promise.resolve([]);
      if (command === "finalize_import_batch") return Promise.resolve(1);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Records" }));
    expect(screen.queryByText("Statement balance check")).not.toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Import 1 transaction" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("finalize_import_batch", { batchId: "batch-1" }));
  });

  it("creates a reusable category rule from phrases", async () => {
    let rules: { category_id: string; category_name: string; category_type: "expense"; match_text: string }[] = [];
    mocks.invoke.mockImplementation((command: string, args?: { categoryName?: string; categoryType?: "income" | "expense"; matchTexts?: string[] }) => {
      if (command === "list_import_category_rules") return Promise.resolve(rules);
      if (command === "save_import_category_rule") {
        rules = (args?.matchTexts ?? []).map(match_text => ({ category_id: "food", category_name: args?.categoryName ?? "Food", category_type: "expense", match_text }));
        return Promise.resolve(rules);
      }
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "list_active_categories") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(screen.getByLabelText("Rule category name"), { target: { value: "Food" } });
    fireEvent.change(screen.getByLabelText("Description phrases"), { target: { value: "inland hotel\nhope traders" } });
    fireEvent.click(screen.getByRole("button", { name: "Add rule" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("save_import_category_rule", {
      categoryId: null,
      categoryName: "Food",
      categoryType: "expense",
      matchTexts: ["inland hotel", "hope traders"],
    }));
    expect(await screen.findByText("inland hotel, hope traders")).toBeInTheDocument();
  });

  it("previews automatic categorization and omits per-row category controls", async () => {
    const batch = { id: "batch-1", account_id: "a", source_file_name: "statement.csv", source_format: "csv", status: "review", reconciliation_status: "unavailable", created_at: "now" };
    const record = { id: "row-1", source_row_reference: "2", original_date: "2026-11-05", normalized_date: "2026-11-05T00:00:00Z", original_description: "Inland Hotel", normalized_description: "Inland Hotel", original_amount: "12.50", normalized_amount_minor: 1250, transaction_type: "expense", validation_status: "valid", duplicate_status: "unmatched", review_status: "pending", allocations: [] };
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([batch]);
      if (command === "list_import_category_rules") return Promise.resolve([{ category_id: "food", category_name: "Food", category_type: "expense", match_text: "hotel" }]);
      if (command === "list_staged_import_records") return Promise.resolve([record]);
      if (command === "list_import_logs") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Records" }));
    expect(await screen.findByText("Food")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Categories" })).not.toBeInTheDocument();
    expect(screen.queryByText("Statement balance check")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Import 1 transaction" })).toBeInTheDocument();
  });

  it("maps distinct money-in and money-out columns when staging", async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_import_batches") return Promise.resolve([]);
      if (command === "list_import_category_rules") return Promise.resolve([]);
      if (command === "list_active_categories") return Promise.resolve([]);
      if (command === "csv_headers") return Promise.resolve(["Date", "Paid In", "Withdrawn", "Details"]);
      if (command === "csv_rows") return Promise.resolve([["Date", "Paid In", "Withdrawn", "Details"], ["2026-11-01", "10.00", "", "Salary"], ["2026-11-02", "", "3.00", "Shop"]]);
      if (command === "create_import_batch") return Promise.resolve("batch-1");
      if (command === "stage_csv_import") return Promise.resolve(2);
      if (command === "list_staged_import_records" || command === "list_import_logs") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(<ImportsPage />);
    fireEvent.change(await screen.findByLabelText("Account"), { target: { value: "a" } });
    const contents = "Date,Paid In,Withdrawn,Details\n2026-11-01,10.00,,Salary\n2026-11-02,,3.00,Shop";
    fireEvent.change(screen.getByLabelText("Statement file"), { target: { files: [new File([contents], "statement.csv", { type: "text/csv" })] } });
    await screen.findByLabelText("Money in column");
    fireEvent.click(screen.getByRole("button", { name: "Create import workspace" }));
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("stage_csv_import", expect.objectContaining({
      request: expect.objectContaining({ incoming_amount_column: "Paid In", outgoing_amount_column: "Withdrawn", positive_amount_type: "income" }),
    })));
  });
});
