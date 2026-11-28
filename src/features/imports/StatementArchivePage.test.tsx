import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import StatementArchivePage from "./StatementArchivePage";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => mocks.invoke(...args) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ writeFile: vi.fn() }));

describe("StatementArchivePage", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.invoke.mockResolvedValue([{
      batch_id: "batch-1", account_id: "account-1", account_name: "Daily account",
      source_file_name: "november.csv", source_format: "csv", batch_status: "finalized",
      imported_at: "2026-11-01 10:00:00", retained_until: "2027-01-30 10:00:00", size_bytes: 1024,
    }]);
  });

  it("shows the retained-file expiry and opens the exact import batch", async () => {
    const onReviewBatch = vi.fn();
    render(<StatementArchivePage onReviewBatch={onReviewBatch} />);
    expect(await screen.findByText("november.csv")).toBeInTheDocument();
    expect(screen.getByText("Daily account")).toBeInTheDocument();
    expect(screen.getByText("CSV · 1 KB")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Review workspace" }));
    expect(onReviewBatch).toHaveBeenCalledWith("batch-1");
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("list_retained_import_assets"));
  });
});
