import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import AccountForm from "./AccountForm";
import AccountEditForm from "./AccountEditForm";
import AccountArchiveDialog from "./AccountArchiveDialog";
import { useAccounts } from "./hooks/useAccounts";
import type { Account, AccountType, CreateAccountInput, UpdateAccountInput } from "./types";

type AccountImportDraft = { row: number; name: string; account_type: AccountType; institution_name: string; opening_balance: string; balance_date: string; selected: boolean; issue: string; created: boolean };
function parseAccountAmount(value: string): number | null {
    const normalized = value.trim().replace(/[,$\s]/g, "").replace(/^[A-Za-z]{3}/, "");
    if (!/^\d+(?:\.\d{1,2})?$/.test(normalized)) return null;
    const [whole, fraction = ""] = normalized.split(".");
    const minor = BigInt(whole) * 100n + BigInt((fraction + "00").slice(0, 2));
    return minor <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(minor) : null;
}
function importedAccountType(value: string): AccountType {
    const normalized = value.trim().toLowerCase().replace(/[_-]+/g, " ");
    if (/mobile|m pesa|mpesa|wallet|e.?money/.test(normalized)) return "mobile_money";
    if (/bank|checking|current|savings/.test(normalized)) return "bank";
    if (/cash/.test(normalized)) return "cash";
    return "other";
}
function normalizeAccountDate(value: string): string {
    const trimmed = value.trim();
    const iso = /^(\d{4})-(\d{2})-(\d{2})$/.exec(trimmed);
    const local = /^(\d{1,2})[/-](\d{1,2})[/-](\d{4})$/.exec(trimmed);
    const year = iso ? Number(iso[1]) : local ? Number(local[3]) : 0;
    const month = iso ? Number(iso[2]) : local ? Number(local[2]) : 0;
    const day = iso ? Number(iso[3]) : local ? Number(local[1]) : 0;
    if (!year || !month || !day) return "";
    const date = new Date(Date.UTC(year, month - 1, day));
    return date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day
        ? `${String(year).padStart(4, "0")}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`
        : "";
}

const accountTypeLabels = {
    mobile_money: "Mobile Money",
    bank: "Bank",
    cash: "Cash",
    other: "Other",
};

export default function AccountsPage() {
    const {
        accounts, archivedAccounts, isLoading, isArchivedLoading, isMutating, error, loadError, archivedLoadError,
        refresh, refreshArchived, create, update, archive, reactivate, getBalance,
    } = useAccounts();

    const [isCreateFormOpen, setIsCreateFormOpen] = useState(false);
    const [editingAccount, setEditingAccount] = useState<Account | null>(null);
    const [showArchived, setShowArchived] = useState(false);
    const [archivingAccount, setArchivingAccount] = useState<Account | null>(null);
    const [archiveBalance, setArchiveBalance] = useState<number | null>(null);
    const [isBalanceLoading, setIsBalanceLoading] = useState(false);
    const [archiveError, setArchiveError] = useState<string | null>(null);
    const [accountImportFile, setAccountImportFile] = useState<File | null>(null);
    const [accountImportBytes, setAccountImportBytes] = useState<number[]>([]);
    const [accountImportSheets, setAccountImportSheets] = useState<string[]>([]);
    const [accountImportSheet, setAccountImportSheet] = useState("");
    const [accountImportRows, setAccountImportRows] = useState<AccountImportDraft[]>([]);
    const [accountImportError, setAccountImportError] = useState("");
    const [accountImportMessage, setAccountImportMessage] = useState("");
    const [isImportingAccounts, setIsImportingAccounts] = useState(false);

    async function handleCreateAccount(request: CreateAccountInput) {
        await create(request);
        setIsCreateFormOpen(false);
    }

    async function handleUpdateAccount(request: UpdateAccountInput) {
        if (!editingAccount) return;

        await update(editingAccount.id, request);
        setEditingAccount(null);
    }

    async function handleOpenArchiveDialog(account: Account) {
        setArchivingAccount(account);
        setArchiveBalance(null);
        setArchiveError(null);
        setIsBalanceLoading(true);

        try {
            const balance = await getBalance(account.id);
            setArchiveBalance(balance);
        } catch {
            setArchiveError("Unable to retrieve the account balance. Please try again.");
        } finally {
            setIsBalanceLoading(false);
        }
    }

    async function handleConfirmArchive() {
        if (!archivingAccount || archiveBalance === null) return;

        setArchiveError(null);

        try {
            await archive(archivingAccount.id, archiveBalance);
            setArchivingAccount(null);
            setArchiveBalance(null);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);

            if (message.toLowerCase().includes("balance changed")) {
                try {
                    const latestBalance = await getBalance(archivingAccount.id);
                    setArchiveBalance(latestBalance);
                    setArchiveError("The balance changed after your review. Please review the updated balance before confirming again.",);
                } catch {
                    setArchiveError("The balance changed, but the latest balance could not be retrieved. Please try again.",);
                }
            } else {
                setArchiveError(message);
            }
        }
    }
    async function handleShowArchived() {
        setShowArchived(true);

        try {
            await refreshArchived();
        } catch {
            // The hook already stores the loading error.
        }
    }
    async function handleReactivate(accountId: string) {
        try {
            await reactivate(accountId);
        } catch {
            // The hook already stores the operation error.
        }
    }

    function applyAccountImportRows(rows: string[][], fileName: string) {
        if (rows.length < 2) throw new Error("The account list needs a header row and at least one data row.");
        const headers = rows[0].map((value) => value.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim());
        const nameIndex = headers.findIndex((value) => /^(account )?name$|account label/.test(value));
        if (nameIndex < 0) throw new Error("Account-list import needs an Account Name column. Account numbers are not stored by ETracker.");
        const typeIndex = headers.findIndex((value) => /account type|product type|^type$/.test(value));
        const institutionIndex = headers.findIndex((value) => /institution|provider|bank name|mobile network/.test(value));
        const balanceIndex = headers.findIndex((value) => /opening balance|current balance|balance/.test(value));
        const dateIndex = headers.findIndex((value) => /as of|opening date|balance date|effective date/.test(value));
        const currencyIndex = headers.findIndex((value) => /^currency|currency code/.test(value));
        const seen = new Set(accounts.map((item) => `${item.name.trim().toLowerCase()}|${(item.institution_name ?? "").trim().toLowerCase()}`));
        const imported = rows.slice(1).flatMap((row, index) => {
            const name = row[nameIndex]?.trim() ?? "";
            const institution_name = institutionIndex < 0 ? "" : row[institutionIndex]?.trim() ?? "";
            const opening_balance = balanceIndex < 0 ? "" : row[balanceIndex]?.trim() ?? "";
            const rawDate = dateIndex < 0 ? "" : row[dateIndex]?.trim() ?? "";
            const balance_date = normalizeAccountDate(rawDate);
            if (!name && !opening_balance) return [];
            let issue = !name ? "Account name is missing." : "";
            const currency = currencyIndex < 0 ? "KES" : row[currencyIndex]?.trim().toUpperCase() ?? "";
            if (!issue && currency && !["KES", "KSH", "KSHS"].includes(currency)) issue = `Currency ${currency} is unsupported; new accounts use KES.`;
            if (!issue && opening_balance) {
                const amount = parseAccountAmount(opening_balance);
                if (amount === null) issue = "Opening balance must be a non-negative amount with at most two decimal places.";
                else if (amount > 0 && !balance_date) issue = "Use a valid balance date (YYYY-MM-DD or DD/MM/YYYY) to import an opening balance.";
            }
            const key = `${name.toLowerCase()}|${institution_name.toLowerCase()}`;
            const duplicate = Boolean(name && seen.has(key));
            if (!issue && duplicate) issue = "An account with this name and institution already exists or appears earlier in this file; review it in Accounts instead.";
            if (name) seen.add(key);
            return [{ row: index + 2, name, account_type: importedAccountType(typeIndex < 0 ? "" : row[typeIndex] ?? ""), institution_name, opening_balance, balance_date, selected: !issue, issue, created: false }];
        });
        if (!imported.length) throw new Error("No account rows were found in this file.");
        setAccountImportRows(imported);
        setAccountImportMessage(`${imported.length} account rows loaded from ${fileName} for review. Duplicates and unsupported values are excluded.`);
        setAccountImportError("");
    }

    async function loadAccountImportFile(file: File | null) {
        setAccountImportFile(file); setAccountImportRows([]); setAccountImportSheets([]); setAccountImportSheet(""); setAccountImportError(""); setAccountImportMessage("");
        if (!file) return;
        if (file.size > 25 * 1024 * 1024) { setAccountImportFile(null); setAccountImportError("Account files must be 25 MiB or smaller."); return; }
        try {
            const extension = file.name.split(".").pop()?.toLowerCase();
            if (extension === "csv") applyAccountImportRows(await invoke<string[][]>("csv_rows", { contents: await file.text() }), file.name);
            else if (["xls", "xlsx"].includes(extension ?? "")) {
                const bytes = Array.from(new Uint8Array(await file.arrayBuffer())); setAccountImportBytes(bytes);
                const sheets = await invoke<string[]>("xlsx_sheet_names", { contents: bytes });
                if (!sheets.length) throw new Error("This workbook does not contain a readable worksheet.");
                setAccountImportSheets(sheets); setAccountImportSheet(sheets[0]);
            } else throw new Error("Choose a CSV or Excel account list (.xls or .xlsx).");
        } catch (reason) { setAccountImportError(reason instanceof Error ? reason.message : String(reason)); }
    }

    async function loadAccountImportSheet() {
        try {
            if (!accountImportFile) throw new Error("Choose an account-list workbook first.");
            const rows = await invoke<string[][]>("xlsx_rows", { contents: accountImportBytes, sheetName: accountImportSheet, headerRow: 1 });
            applyAccountImportRows(rows, accountImportFile.name);
        } catch (reason) { setAccountImportError(reason instanceof Error ? reason.message : String(reason)); }
    }

    async function createImportedAccounts() {
        const selected = accountImportRows.filter((row) => row.selected && !row.issue && !row.created);
        if (!selected.length) { setAccountImportError("Select at least one valid account row to import."); return; }
        setIsImportingAccounts(true); setAccountImportError(""); setAccountImportMessage("");
        let created = 0;
        const failed: string[] = [];
        for (const row of selected) {
            try {
                const amount = row.opening_balance ? parseAccountAmount(row.opening_balance) : 0;
                await create({ name: row.name, account_type: row.account_type, institution_name: row.institution_name || null, notes: null, opening_balance: amount && amount > 0 ? { amount_minor: amount, occurred_at: `${row.balance_date}T00:00:00Z` } : null });
                created += 1;
                setAccountImportRows((current) => current.map((item) => item.row === row.row ? { ...item, selected: false, created: true, issue: "Imported" } : item));
            } catch (reason) {
                const message = typeof reason === "string" ? reason : reason instanceof Error ? reason.message : "Could not create account.";
                failed.push(`Row ${row.row}: ${message}`);
                setAccountImportRows((current) => current.map((item) => item.row === row.row ? { ...item, selected: false, issue: message } : item));
            }
        }
        setIsImportingAccounts(false);
        setAccountImportMessage(`${created} account${created === 1 ? "" : "s"} imported${failed.length ? `; ${failed.length} need attention` : ""}.`);
        if (failed.length) setAccountImportError(failed.join(" "));
    }

    return (
        <div className="mx-auto max-w-6xl space-y-6">
            <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
                <div>
                    <p className="workspace-accent text-xs font-semibold uppercase tracking-[0.14em]">Financial accounts</p>
                    <h1 className="mt-2 text-2xl font-semibold tracking-tight text-slate-900">Your accounts</h1>
                    <p className="workspace-muted mt-2 max-w-2xl text-sm leading-6">Manage the accounts you use to hold and move your money.</p>
                </div>

                <div className="flex flex-wrap gap-3">
                    <button type="button" onClick={showArchived ? () => setShowArchived(false) : handleShowArchived}
                        aria-pressed={showArchived}
                        className="rounded-lg border border-slate-300 px-4 py-2.5 text-sm font-medium text-slate-700 hover:bg-slate-50">
                        {showArchived ? "Active accounts" : "Archived accounts"}
                    </button>

                    <button type="button" onClick={() => setIsCreateFormOpen(true)} disabled={isMutating}
                        className="inline-flex items-center justify-center gap-2 rounded-lg bg-emerald-700 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-emerald-800 disabled:opacity-50">
                        <span className="text-lg leading-none">+</span>
                        Add account
                    </button>
                </div>
            </div>

            {!showArchived && (
                <section className="workspace-panel space-y-4 rounded-xl p-5">
                    <div>
                        <h2 className="font-semibold">Import an account list</h2>
                        <p className="workspace-muted mt-1 text-sm">
                            Review CSV or Excel account details before importing. New accounts use KES.
                            Account numbers are never retained; exact name and institution duplicates are skipped.
                        </p>
                    </div>

                    <label className="block max-w-xl text-sm font-medium">
                        Account list file
                        <input
                            aria-label="Account list file"
                            type="file"
                            accept=".csv,.xls,.xlsx"
                            onChange={(event) => void loadAccountImportFile(event.target.files?.[0] ?? null)}
                            className="mt-1 block w-full text-sm"
                        />
                    </label>

                    {accountImportSheets.length > 0 && (
                        <div className="flex flex-wrap items-end gap-3">
                            <label className="text-sm font-medium">
                                Worksheet
                                <select
                                    aria-label="Account worksheet"
                                    value={accountImportSheet}
                                    onChange={(event) => setAccountImportSheet(event.target.value)}
                                    className="mt-1 block min-w-64 rounded-md border border-slate-300 bg-white px-3 py-2"
                                >
                                    {accountImportSheets.map((sheet) => <option key={sheet}>{sheet}</option>)}
                                </select>
                            </label>
                            <button type="button" onClick={() => void loadAccountImportSheet()} className="rounded-md border border-slate-300 px-3 py-2 text-sm">
                                Load worksheet
                            </button>
                        </div>
                    )}

                    {accountImportError && <p role="alert" className="text-sm text-rose-700">{accountImportError}</p>}
                    {accountImportMessage && <p role="status" className="text-sm text-emerald-800">{accountImportMessage}</p>}

                    {accountImportRows.length > 0 && (
                        <>
                            <div className="overflow-x-auto">
                                <table className="w-full min-w-[760px] text-left text-sm">
                                    <caption className="sr-only">Account rows imported for user review</caption>
                                    <thead className="border-b text-xs uppercase text-slate-500">
                                        <tr>
                                            <th className="p-2">Import</th>
                                            <th className="p-2">Account</th>
                                            <th className="p-2">Type</th>
                                            <th className="p-2">Institution</th>
                                            <th className="p-2 text-right">Opening balance · KES</th>
                                            <th className="p-2">Balance date</th>
                                            <th className="p-2">Review</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {accountImportRows.map((row) => (
                                            <tr key={row.row} className="border-b">
                                                <td className="p-2">
                                                    <input
                                                        aria-label={`Import account row ${row.row}`}
                                                        type="checkbox"
                                                        checked={row.selected}
                                                        disabled={Boolean(row.issue) || row.created || isImportingAccounts}
                                                        onChange={(event) => setAccountImportRows((current) => current.map((item) => item.row === row.row ? { ...item, selected: event.target.checked } : item))}
                                                    />
                                                </td>
                                                <td className="p-2">{row.name || "—"}</td>
                                                <td className="p-2">{accountTypeLabels[row.account_type]}</td>
                                                <td className="p-2">{row.institution_name || "—"}</td>
                                                <td className="p-2 text-right">{row.opening_balance || "—"}</td>
                                                <td className="p-2">{row.balance_date || "—"}</td>
                                                <td className="p-2">{row.issue || "Ready"}</td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                            <button
                                type="button"
                                disabled={isImportingAccounts || isMutating || !accountImportRows.some((row) => row.selected && !row.issue)}
                                onClick={() => void createImportedAccounts()}
                                className="rounded-md bg-emerald-700 px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
                            >
                                {isImportingAccounts ? "Importing accounts…" : "Create selected accounts"}
                            </button>
                        </>
                    )}
                </section>
            )}

            {showArchived
                ? (
                    <div className="workspace-panel overflow-hidden rounded-xl">
                        <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4">
                            <h3 className="font-semibold text-slate-800"> Archived accounts </h3>
                            <span className="text-xs text-slate-500">
                                {archivedAccounts.length}{" "}
                                {archivedAccounts.length === 1 ? "account" : "accounts"}
                            </span>
                        </div>

                        {isArchivedLoading
                            ? (
                                <p className="p-8 text-center text-sm text-slate-500"> Loading archived accounts... </p>)
                            : archivedLoadError
                                ? (
                                    <div className="p-8 text-center">
                                        <p className="text-sm text-red-600"> {archivedLoadError} </p>
                                        <button type="button" onClick={() => void refreshArchived().catch(() => { })}
                                            className="mt-3 text-sm font-medium text-emerald-700 hover:underline">
                                            Try again
                                        </button>
                                    </div>)
                                : archivedAccounts.length === 0
                                    ? (
                                        <p className="p-10 text-center text-sm text-slate-500">No archived accounts. </p>)
                                    : (
                                        <div className="divide-y divide-slate-100">
                                            {archivedAccounts.map((account) => (
                                                <div key={account.id} className="workspace-row flex flex-wrap items-center justify-between gap-4 px-5 py-4">
                                                    <div className="min-w-0">
                                                        <h4 className="truncate text-sm font-semibold text-slate-800"> {account.name} </h4>
                                                        <p className="mt-1 text-xs text-slate-500">
                                                            {accountTypeLabels[account.account_type]}
                                                            {account.institution_name ? ` · ${account.institution_name}` : ""}
                                                        </p>
                                                        <p className="mt-1 text-xs text-slate-400">
                                                            Archived{" "}
                                                            {account.archived_at ? new Date(account.archived_at).toLocaleDateString() : ""}
                                                        </p>
                                                    </div>
                                                    <button type="button" onClick={() => void handleReactivate(account.id)} disabled={isMutating}
                                                        className="rounded-lg border border-slate-300 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50">
                                                        Reactivate
                                                    </button>
                                                </div>
                                            ))}
                                        </div>
                                    )}
                    </div>)
                : isLoading
                    ? (
                        <div className="workspace-panel rounded-xl p-10 text-center">
                            <p className="text-sm text-slate-500"> Loading your accounts... </p>
                        </div>)
                    : loadError
                        ? (
                            <div className="workspace-panel workspace-error-panel rounded-xl p-8">
                                <h3 className="font-semibold text-red-700"> Unable to load accounts </h3>
                                <p className="mt-2 text-sm text-slate-600">{loadError}</p>
                                <button type="button" onClick={() => void refresh().catch(() => { })}
                                    className="mt-4 rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700">
                                    Try again
                                </button>
                            </div>)
                        : accounts.length === 0
                            ? (
                                <div className="workspace-panel flex min-h-72 flex-col items-center justify-center rounded-xl border-dashed border-slate-300 px-6 py-12 text-center">
                                    <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-slate-100 text-xl text-slate-500"> + </div>
                                    <h3 className="text-base font-semibold text-slate-800"> No active accounts </h3>
                                    <p className="mt-2 max-w-md text-sm leading-6 text-slate-500"> Add an account to start managing your finances, or view your archived accounts. </p>
                                    <button type="button" onClick={() => setIsCreateFormOpen(true)}
                                        className="mt-5 rounded-lg bg-emerald-700 px-4 py-2.5 text-sm font-medium text-white hover:bg-emerald-800">
                                        Create an account
                                    </button>
                                </div>)
                            : (
                                <div className="workspace-panel overflow-hidden rounded-xl">
                                    <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4">
                                        <h3 className="font-semibold text-slate-800"> Active accounts </h3>
                                        <span className="text-xs text-slate-500">
                                            {accounts.length}{" "}
                                            {accounts.length === 1 ? "account" : "accounts"}
                                        </span>
                                    </div>
                                    <div className="divide-y divide-slate-100">
                                        {accounts.map((account) => (
                                            <div key={account.id}
                                                className="workspace-row flex flex-wrap items-center justify-between gap-4 px-5 py-4">
                                                <div className="min-w-0">
                                                    <h4 className="truncate text-sm font-semibold text-slate-800"> {account.name} </h4>
                                                    <p className="mt-1 text-xs text-slate-500">
                                                        {accountTypeLabels[account.account_type]}
                                                        {account.institution_name ? ` · ${account.institution_name}` : ""}
                                                    </p>
                                                </div>
                                                <div className="flex items-center gap-2">
                                                    <button type="button" onClick={() => setEditingAccount(account)} disabled={isMutating}
                                                        className="rounded-lg border border-slate-300 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50">
                                                        Edit
                                                    </button>
                                                    <button type="button" onClick={() => void handleOpenArchiveDialog(account)} disabled={isMutating}
                                                        className="rounded-lg border border-red-200 px-3 py-1.5 text-xs font-medium text-red-700 hover:bg-red-50 disabled:opacity-50">
                                                        Archive
                                                    </button>
                                                    <span className="rounded-full bg-emerald-50 px-3 py-1 text-xs font-medium text-emerald-700">
                                                        Active
                                                    </span>
                                                </div>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}

            {isCreateFormOpen && (
                <AccountForm isSubmitting={isMutating} error={error} onSubmit={handleCreateAccount} onCancel={() => setIsCreateFormOpen(false)} />
            )}

            {editingAccount && (
                <AccountEditForm account={editingAccount} isSubmitting={isMutating} error={error} onSubmit={handleUpdateAccount} onCancel={() => setEditingAccount(null)} />
            )}

            {archivingAccount && (
                <AccountArchiveDialog account={archivingAccount} balanceMinor={archiveBalance} isLoading={isBalanceLoading} isSubmitting={isMutating} error={archiveError}
                    onConfirm={handleConfirmArchive}
                    onCancel={() => {
                        if (isMutating) return;
                        setArchivingAccount(null);
                        setArchiveBalance(null);
                        setArchiveError(null);
                    }}
                />
            )}
        </div>
    );
}
