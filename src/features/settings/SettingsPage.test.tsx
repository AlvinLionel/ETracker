import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SettingsPage from "./SettingsPage";

const invoke = vi.fn();
const fileDialog = vi.hoisted(() => ({ save: vi.fn(), writeFile: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: fileDialog.save }));
vi.mock("@tauri-apps/plugin-fs", () => ({ writeFile: fileDialog.writeFile }));
const dateInDays = (days: number) => { const date = new Date(); date.setDate(date.getDate() + days); return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`; };

describe("SettingsPage", () => {
  beforeEach(() => {
    invoke.mockReset();
    fileDialog.save.mockReset().mockResolvedValue("C:\\Users\\Test\\ETracker_backup.etracker-backup");
    fileDialog.writeFile.mockReset().mockResolvedValue(undefined);
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 5, row_density: "standard" })
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
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_user_preferences", { preferences: { subscription_reminders_enabled: false, subscription_reminder_days: 14, theme: "system", grid_opacity: 5, row_density: "standard" } }));
    expect(await screen.findByText("Subscription reminders are turned off.")).toBeInTheDocument();
  });

  it("runs and displays the local database integrity check", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 5, row_density: "standard" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : Promise.resolve({ integrity_messages: ["ok"], foreign_key_violations: [] }));
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Check local data" }));
    expect(await screen.findByText("The local database passed both checks.")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("check_database_integrity");
  });

  it("shows a diagnostic report when the database has broken references", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 5, row_density: "standard" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : Promise.resolve({ integrity_messages: ["ok"], foreign_key_violations: ["account_entries row Some(3) references accounts (constraint 0)"] }));
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Check local data" }));
    expect(await screen.findByText(/Avoid editing affected records/)).toBeInTheDocument();
    expect(screen.getByText(/references accounts/)).toBeInTheDocument();
  });

  it("saves and applies the selected theme", async () => {
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: /Dark Vault/ }));
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_user_preferences", { preferences: { subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "dark", grid_opacity: 5, row_density: "standard" } }));
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("previews and persists grid opacity and row density", async () => {
    const { unmount } = render(<SettingsPage />);
    await screen.findByRole("heading", { name: "Appearance" });

    fireEvent.change(screen.getByRole("slider", { name: "Accounting grid opacity" }), { target: { value: "12" } });
    fireEvent.click(screen.getByRole("button", { name: /Compact/ }));

    expect(document.documentElement.style.getPropertyValue("--workspace-grid")).toContain("12%");
    expect(document.documentElement.dataset.previewDensity).toBe("compact");
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_user_preferences", { preferences: { subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 12, row_density: "compact" } }));
    unmount();
    expect(document.documentElement.dataset.previewDensity).toBe("compact");
    expect(document.documentElement.style.getPropertyValue("--workspace-grid")).toContain("12%");
    document.documentElement.style.removeProperty("--workspace-grid");
    delete document.documentElement.dataset.previewDensity;
  });

  it("marks the active configuration section and navigates to another section", async () => {
    render(<SettingsPage />);
    const appearance = screen.getByRole("button", { name: "Appearance" });
    const reminders = screen.getByRole("button", { name: "Reminders" });
    expect(reminders).toHaveAttribute("aria-current", "location");

    fireEvent.click(appearance);

    expect(appearance).toHaveAttribute("aria-current", "location");
    expect(reminders).not.toHaveAttribute("aria-current");
  });

  it("saves a passphrase-encrypted backup to a user-selected file only after confirmation", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 5, row_density: "standard" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : command === "export_encrypted_backup" ? Promise.resolve("YWJj") : Promise.resolve());
    const { container } = render(<SettingsPage />);
    await screen.findByRole("heading", { name: "Encrypted backup and restore" });
    fireEvent.change(container.querySelector<HTMLInputElement>('input[aria-label="Backup passphrase"]')!, { target: { value: "a sufficiently long passphrase" } });
    const download = screen.getByRole("button", { name: "Save encrypted backup" });
    expect(download).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Confirm backup passphrase"), { target: { value: "a sufficiently long passphrase" } });
    fireEvent.click(download);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("export_encrypted_backup", { passphrase: "a sufficiently long passphrase" }));
    await waitFor(() => expect(fileDialog.save).toHaveBeenCalledWith(expect.objectContaining({ filters: [{ name: "ETracker encrypted backup", extensions: ["etracker-backup"] }] })));
    expect(fileDialog.writeFile).toHaveBeenCalledWith("C:\\Users\\Test\\ETracker_backup.etracker-backup", new Uint8Array([97, 98, 99]));
    expect(await screen.findByText(/passphrase separately/)).toBeInTheDocument();
  });

  it("does not write a backup when the native save dialog is cancelled", async () => {
    fileDialog.save.mockResolvedValue(null);
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 5, row_density: "standard" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : command === "export_encrypted_backup" ? Promise.resolve("YWJj") : Promise.resolve());
    const { container } = render(<SettingsPage />);
    await screen.findByRole("heading", { name: "Encrypted backup and restore" });
    fireEvent.change(container.querySelector<HTMLInputElement>('input[aria-label="Backup passphrase"]')!, { target: { value: "a sufficiently long passphrase" } });
    fireEvent.change(container.querySelector<HTMLInputElement>('input[aria-label="Confirm backup passphrase"]')!, { target: { value: "a sufficiently long passphrase" } });
    fireEvent.click(screen.getByRole("button", { name: "Save encrypted backup" }));
    expect(await screen.findByRole("status")).toHaveTextContent("Backup save cancelled; no file was written.");
    expect(fileDialog.writeFile).not.toHaveBeenCalled();
    expect(container).toBeInTheDocument();
  });

  it("requires explicit restore confirmation and reloads after validation", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 5, row_density: "standard" })
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

  it("downloads a diagnostic report on request", async () => {
    invoke.mockImplementation((command: string) => command === "get_user_preferences"
      ? Promise.resolve({ subscription_reminders_enabled: true, subscription_reminder_days: 7, theme: "system", grid_opacity: 5, row_density: "standard" })
      : command === "list_subscriptions" ? Promise.resolve([])
        : command === "get_diagnostic_report" ? Promise.resolve({ app_version: "0.1.0", operating_system: "windows", architecture: "x86_64", sqlite_version: "3.50.0", schema_version: 14, latest_supported_schema_version: 14, database_integrity: "passed", foreign_key_check: "passed" }) : Promise.resolve());
    const create = vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:diagnostics");
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    render(<SettingsPage />);
    fireEvent.click(await screen.findByRole("button", { name: "Download diagnostic report" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("get_diagnostic_report"));
    expect(create).toHaveBeenCalled();
    expect(await screen.findByRole("status")).toHaveTextContent(/Privacy-safe diagnostic report downloaded/);
  });
});
