import type { Account } from "./types";
import { formatMoney } from "../../shared/formatMoney";

interface AccountArchiveDialogProps {
    account: Account;
    balanceMinor: number | null;
    isLoading: boolean;
    isSubmitting: boolean;
    error: string | null;
    onConfirm: () => void;
    onCancel: () => void;
}

function formatKES(amountMinor: number): string {
    return formatMoney(amountMinor, "KES");
}

export default function AccountArchiveDialog({ account, balanceMinor, isLoading, isSubmitting, error, onConfirm, onCancel }: AccountArchiveDialogProps) {
    const hasBalance = balanceMinor !== null && balanceMinor !== 0;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/40 p-4">
            <div role="dialog" aria-modal="true" aria-labelledby="archive-dialog-title"
                className="w-full max-w-md rounded-xl bg-white p-6 shadow-xl"
            >
                <h2 id="archive-dialog-title" className="text-lg font-semibold text-slate-900">Archive account</h2>

                <p className="mt-2 text-sm leading-6 text-slate-600">
                    Are you sure you want to archive{" "}
                    <strong>{account.name}</strong>?
                    It will no longer be available for new transactions, but its historical records will be preserved.
                </p>

                <div className="mt-5 rounded-lg bg-slate-50 p-4">
                    <p className="text-xs font-medium uppercase tracking-wide text-slate-500">Current balance</p>

                    {isLoading
                        ? (<p className="mt-2 text-sm text-slate-500"> Retrieving balance... </p>)
                        : balanceMinor !== null
                            ? (<p className="mt-2 text-xl font-semibold text-slate-900"> {formatKES(balanceMinor)} </p>)
                            : (<p className="mt-2 text-sm text-red-600">Balance unavailable.</p>)}
                </div>

                {hasBalance && (
                    <div className="mt-4 rounded-lg border border-amber-200 bg-amber-50 p-4">
                        <p className="text-sm font-semibold text-amber-800">This account still has money in it.</p>
                        <p className="mt-1 text-sm leading-5 text-amber-700">Archiving will not transfer or remove this money. Consider transferring the funds separately if necessary.</p>
                    </div>
                )}

                {error && (
                    <div role="alert" className="mt-4 rounded-lg border border-red-200 bg-red-50 p-3">
                        <p className="text-sm text-red-700">{error}</p>
                    </div>
                )}

                <div className="mt-6 flex justify-end gap-3">
                    <button type="button" onClick={onCancel} disabled={isSubmitting}
                        className="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                    >
                        Cancel
                    </button>

                    <button type="button" onClick={onConfirm}
                        disabled={isLoading || isSubmitting || balanceMinor === null}
                        className="rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-50"
                    >
                        {isSubmitting ? "Archiving..." : "Confirm archive"}
                    </button>
                </div>
            </div>
        </div>
    );
}
