import { useState } from "react";
import AccountForm from "./AccountForm";
import AccountEditForm from "./AccountEditForm";
import AccountArchiveDialog from "./AccountArchiveDialog";
import { useAccounts } from "./hooks/useAccounts";
import type { Account, CreateAccountInput, UpdateAccountInput } from "./types";

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

    return (
        <div className="mx-auto max-w-6xl">
            <div className="mb-8 flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
                <div>
                    <p className="text-sm font-medium text-emerald-700"> FINANCIAL ACCOUNTS </p>
                    <h2 className="mt-2 text-2xl font-semibold tracking-tight text-slate-900"> Your accounts </h2>
                    <p className="mt-2 max-w-2xl text-sm leading-6 text-slate-500"> Manage the accounts you use to hold and move your money.</p>
                </div>

                <div className="flex flex-wrap gap-3">
                    <button type="button" onClick={showArchived ? () => setShowArchived(false) : handleShowArchived}
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

            {showArchived
                ? (
                    <div className="overflow-hidden rounded-xl border border-slate-200 bg-white">
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
                                                <div key={account.id} className="flex flex-wrap items-center justify-between gap-4 px-5 py-4">
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
                        <div className="rounded-xl border border-slate-200 bg-white p-10 text-center">
                            <p className="text-sm text-slate-500"> Loading your accounts... </p>
                        </div>)
                    : loadError
                        ? (
                            <div className="rounded-xl border border-red-200 bg-white p-8">
                                <h3 className="font-semibold text-red-700"> Unable to load accounts </h3>
                                <p className="mt-2 text-sm text-slate-600">{loadError}</p>
                                <button type="button" onClick={() => void refresh().catch(() => { })}
                                    className="mt-4 rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700">
                                    Try again
                                </button>
                            </div>)
                        : accounts.length === 0
                            ? (
                                <div className="flex min-h-72 flex-col items-center justify-center rounded-xl border border-dashed border-slate-300 bg-white px-6 py-12 text-center">
                                    <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-slate-100 text-xl text-slate-500"> + </div>
                                    <h3 className="text-base font-semibold text-slate-800"> No active accounts </h3>
                                    <p className="mt-2 max-w-md text-sm leading-6 text-slate-500"> Add an account to start managing your finances, or view your archived accounts. </p>
                                    <button type="button" onClick={() => setIsCreateFormOpen(true)}
                                        className="mt-5 rounded-lg bg-emerald-700 px-4 py-2.5 text-sm font-medium text-white hover:bg-emerald-800">
                                        Create an account
                                    </button>
                                </div>)
                            : (
                                <div className="overflow-hidden rounded-xl border border-slate-200 bg-white">
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
                                                className="flex flex-wrap items-center justify-between gap-4 px-5 py-4">
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
