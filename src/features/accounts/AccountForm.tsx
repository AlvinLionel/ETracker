
import { useState, type FormEvent } from "react";
import type { AccountType, CreateAccountInput } from "./types";

interface AccountFormProps {
    isSubmitting: boolean;
    error?: string | null;
    onSubmit: (request: CreateAccountInput) => Promise<void>;
    onCancel: () => void;
}

const accountTypes: { value: AccountType; label: string }[] = [
    { value: "mobile_money", label: "Mobile Money" },
    { value: "bank", label: "Bank" },
    { value: "cash", label: "Cash" },
    { value: "other", label: "Other" },
];

function parseAmountToMinorUnits(value: string): number | null {
    const normalized = value.trim();

    if (!/^\d+(\.\d{1,2})?$/.test(normalized)) return null;

    const [whole, fraction = ""] = normalized.split(".");
    const amount = Number(whole) * 100 + Number(fraction.padEnd(2, "0"));

    if (!Number.isSafeInteger(amount) || amount <= 0) return null;

    return amount;
}

function AccountForm({ isSubmitting, error, onSubmit, onCancel }: AccountFormProps) {
    const [name, setName] = useState("");
    const [accountType, setAccountType] = useState<AccountType>("mobile_money");
    const [institutionName, setInstitutionName] = useState("");
    const [notes, setNotes] = useState("");
    const [openingBalance, setOpeningBalance] = useState("");
    const [openingBalanceDate, setOpeningBalanceDate] = useState("");
    const [formError, setFormError] = useState<string | null>(null);

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setFormError(null);

        const trimmedName = name.trim();

        if (!trimmedName) {
            setFormError("Account name is required.");
            return;
        }

        const hasOpeningBalance = openingBalance.trim() !== "";
        let openingBalanceInput: CreateAccountInput["opening_balance"] = null;

        if (hasOpeningBalance) {
            const amountMinor = parseAmountToMinorUnits(openingBalance);

            if (amountMinor === null) {
                setFormError("Enter a valid opening balance greater than zero, with no more than two decimal places.");
                return;
            }

            if (!openingBalanceDate) {
                setFormError("Select the effective date and time of the opening balance.");
                return;
            }

            const parsedDate = new Date(openingBalanceDate);

            if (Number.isNaN(parsedDate.getTime())) {
                setFormError("Enter a valid opening balance date and time.");
                return;
            }

            openingBalanceInput = {
                amount_minor: amountMinor,
                occurred_at: parsedDate.toISOString(),
            };
        }

        try {
            await onSubmit({
                name: trimmedName,
                account_type: accountType,
                institution_name: institutionName.trim() || null,
                notes: notes.trim() || null,
                opening_balance: openingBalanceInput,
            });
        } catch {
            // The parent hook retains the operation error.
            // Keep the form open so the user can correct or retry.
        }
    }

    const inputClassName = "mt-1.5 w-full rounded-lg border border-slate-300 bg-white px-3 py-2.5 text-sm text-slate-900 outline-none transition focus:border-emerald-600 focus:ring-2 focus:ring-emerald-100";
    const labelClassName = "text-sm font-medium text-slate-700";

    return (
        <div
            className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/40 p-4"
            onMouseDown={(event) => {
                if (event.target === event.currentTarget && !isSubmitting) onCancel();
            }}
        >
            <div role="dialog" aria-modal="true" aria-labelledby="account-form-title"
                className="max-h-[90vh] w-full max-w-xl overflow-y-auto rounded-2xl bg-white shadow-xl"
            >
                <div className="border-b border-slate-200 px-6 py-5">
                    <h3 id="account-form-title" className="text-lg font-semibold text-slate-900">
                        Add an account
                    </h3>
                    <p className="mt-1 text-sm text-slate-500">
                        Enter the details of the financial account you want to track.
                    </p>
                </div>

                <form onSubmit={handleSubmit}>
                    <div className="space-y-5 px-6 py-6">
                        <div>
                            <label htmlFor="account-name" className={labelClassName}>
                                Account name <span className="text-red-600">*</span>
                            </label>
                            <input id="account-name" autoFocus required maxLength={100} value={name} onChange={(event) => setName(event.target.value)} placeholder="e.g. Personal M-Pesa" className={inputClassName} />
                        </div>

                        <div>
                            <label htmlFor="account-type" className={labelClassName}>
                                Account type
                            </label>
                            <select id="account-type" value={accountType}
                                onChange={(event) =>
                                    setAccountType(event.target.value as AccountType)
                                }
                                className={inputClassName}
                            >
                                {accountTypes.map((type) => (
                                    <option key={type.value} value={type.value}>
                                        {type.label}
                                    </option>
                                ))}
                            </select>
                        </div>

                        <div>
                            <label htmlFor="institution-name" className={labelClassName}>
                                Institution / Provider
                            </label>
                            <input id="institution-name" maxLength={100} value={institutionName} onChange={(event) => setInstitutionName(event.target.value)} placeholder="e.g. Safaricom, Equity Bank" className={inputClassName} />
                        </div>

                        <div className="border-t border-slate-200 pt-5">
                            <h4 className="text-sm font-semibold text-slate-800">
                                Opening balance
                            </h4>
                            <p className="mt-1 text-xs leading-5 text-slate-500">
                                Optional. If provided, this creates the account's initial balance entry in the ledger.
                            </p>

                            <div className="mt-4 grid gap-4 sm:grid-cols-2">
                                <div>
                                    <label htmlFor="opening-balance" className={labelClassName}>
                                        Amount (KES)
                                    </label>
                                    <input id="opening-balance" type="number" min="0.01" step="0.01" inputMode="decimal" value={openingBalance}
                                        onChange={(event) =>
                                            setOpeningBalance(event.target.value)
                                        }
                                        placeholder="0.00" className={inputClassName}
                                    />
                                </div>

                                <div>
                                    <label htmlFor="opening-balance-date" className={labelClassName}>
                                        Effective date and time
                                    </label>
                                    <input id="opening-balance-date" type="datetime-local" value={openingBalanceDate}
                                        onChange={(event) =>
                                            setOpeningBalanceDate(event.target.value)
                                        }
                                        disabled={!openingBalance.trim()} className={inputClassName}
                                    />
                                </div>
                            </div>
                        </div>

                        <div>
                            <label htmlFor="account-notes" className={labelClassName}>
                                Notes
                            </label>
                            <textarea id="account-notes" rows={3} maxLength={1000} value={notes} placeholder="Optional information about this account" className={inputClassName}
                                onChange={(event) => setNotes(event.target.value)}
                            />
                        </div>

                        {(formError || error) && (
                            <p role="alert" className="rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700">
                                {formError || error}
                            </p>
                        )}
                    </div>

                    <div className="flex justify-end gap-3 border-t border-slate-200 px-6 py-4">
                        <button type="button" onClick={onCancel} disabled={isSubmitting}
                            className="rounded-lg border border-slate-300 px-4 py-2.5 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                        >
                            Cancel
                        </button>

                        <button type="submit" disabled={isSubmitting}
                            className="rounded-lg bg-emerald-700 px-4 py-2.5 text-sm font-medium text-white hover:bg-emerald-800 disabled:cursor-not-allowed disabled:opacity-60"
                        >
                            {isSubmitting ? "Creating..." : "Create account"}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}

export default AccountForm;
