import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SettingsPage from "./SettingsPage";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
const dateInDays = (days: number) => { const date = new Date(); date.setDate(date.getDate() + days); return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`; };

describe("SettingsPage", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system" })
      : command === "list_subscriptions" ? Promise.resolve([
        { id: "soon", name: "Internet", amount_minor: 2000, currency_code: "KES", frequency: "monthly", next_due_date: dateInDays(3), status: "active" },
        { id: "later", name: "Insurance", amount_minor: 5000, currency_code: "KES", frequency: "yearly", next_due_date: dateInDays(20), status: "active" },
        { id: "paused", name: "Paused", amount_minor: 1000, currency_code: "KES", frequency: "monthly", next_due_date: dateInDays(2), status: "paused" },
      ]) : Promise.resolve());
  });

  it("lists active upcoming payments and persists reminder preferences", async () => {
    render(<SettingsPage />);
    expect(await screen.findByText("Internet")).toBeInTheDocument();
    expect(screen.queryByText("Insurance")).not.toBeInTheDocument();
    expect(screen.queryByText("Paused")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("checkbox", { name: /Show upcoming payment reminders/ }));
    fireEvent.change(screen.getByRole("spinbutton", { name: "Reminder window in days" }), { target: { value: "14" } });
    fireEvent.click(screen.getByRole("button", { name: "Save preferences" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_user_preferences", { preferences: { subscription_reminders_enabled: false, subscription_reminder_days: 14, theme: "system" } }));
    expect(await screen.findByText("Subscription reminders are turned off.")).toBeInTheDocument();
  });

  it("runs and displays the local database integrity check", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : Promise.resolve({ integrity_messages: ["ok"], foreign_key_violations: [] }));
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Check local data" }));
    expect(await screen.findByText("The local database passed both checks.")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("check_database_integrity");
  });

  it("shows a diagnostic report when the database has broken references", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : Promise.resolve({ integrity_messages: ["ok"], foreign_key_violations: ["account_entries row Some(3) references accounts (constraint 0)"] }));
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Check local data" }));
    expect(await screen.findByText(/Avoid editing affected records/)).toBeInTheDocument();
    expect(screen.getByText(/references accounts/)).toBeInTheDocument();
  });

  it("saves and applies the selected theme", async () => {
    render(<SettingsPage />);
    await screen.findByRole("combobox", { name: "Color theme" });
    fireEvent.change(screen.getByRole("combobox", { name: "Color theme" }), { target: { value: "dark" } });
    fireEvent.click(screen.getByRole("button", { name: "Save preferences" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_user_preferences", { preferences: { subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "dark" } }));
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("creates a passphrase-encrypted backup only after confirmation", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : command === "export_encrypted_backup" ? Promise.resolve("YWJj") : Promise.resolve());
    const create = vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:backup");
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    const { container } = render(<SettingsPage />);
    await screen.findByRole("heading", { name: "Encrypted backup and restore" });
    fireEvent.change(container.querySelector<HTMLInputElement>('input[aria-label="Backup passphrase"]')!, { target: { value: "a sufficiently long passphrase" } });
    const download = screen.getByRole("button", { name: "Download encrypted backup" });
    expect(download).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Confirm backup passphrase"), { target: { value: "a sufficiently long passphrase" } });
    fireEvent.click(download);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("export_encrypted_backup", { passphrase: "a sufficiently long passphrase" }));
    expect(create).toHaveBeenCalled(); expect(revoke).toHaveBeenCalledWith("blob:backup");
    expect(await screen.findByText(/passphrase separately/)).toBeInTheDocument();
  });

  it("requires explicit restore confirmation and reloads after validation", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : command === "restore_encrypted_backup" ? Promise.resolve() : Promise.resolve());
    const file = new File(["encrypted"], "backup.etracker-backup");
    Object.defineProperty(file, "arrayBuffer", { value: async () => new TextEncoder().encode("encrypted").buffer });
    render(<SettingsPage />);
    fireEvent.change(await screen.findByLabelText("Encrypted backup file"), { target: { files: [file] } });
    fireEvent.change(screen.getByLabelText("Restore passphrase"), { target: { value: "a sufficiently long passphrase" } });
    const restore = screen.getByRole("button", { name: "Replace current data" });
    expect(restore).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Restore confirmation"), { target: { value: "RESTORE" } });
    fireEvent.click(restore);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("restore_encrypted_backup", { backupBase64: "ZW5jcnlwdGVk", passphrase: "a sufficiently long passphrase" }));
    expect(await screen.findByRole("button", { name: "Reload ETracker" })).toBeInTheDocument();
  });
});
