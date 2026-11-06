import { useState, type FormEvent } from "react";
import type { Account, UpdateAccountInput } from "./types";

interface AccountEditFormProps {
    account: Account;
    isSubmitting: boolean;
    error?: string | null;
    onSubmit: (request: UpdateAccountInput) => Promise<void>;
    onCancel: () => void;
}

export default function AccountEditForm({ account, isSubmitting, error, onSubmit, onCancel }: AccountEditFormProps) {
    const [name, setName] = useState(account.name);
    const [institutionName, setInstitutionName] = useState(account.institution_name ?? "");
    const [notes, setNotes] = useState(account.notes ?? "");
    const [formError, setFormError] = useState<string | null>(null);

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setFormError(null);

        if (!name.trim()) {
            setFormError("Account name is required.");
            return;
        }

        try {
            await onSubmit({
                name: name.trim(),
                institution_name: institutionName.trim() || null,
                notes: notes.trim() || null,
            });
        } catch {
            // The parent hook retains the operation error.
        }
    }
    const inputClassName = "mt-1.5 w-full rounded-lg border border-slate-300 bg-white px-3 py-2.5 text-sm text-slate-900 outline-none transition focus:border-emerald-600 focus:ring-2 focus:ring-emerald-100";

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/40 p-4">
            <div role="dialog" aria-modal="true" aria-labelledby="edit-account-title"
                className="w-full max-w-xl overflow-y-auto rounded-2xl bg-white shadow-xl"
            >
                <div className="border-b border-slate-200 px-6 py-5">
                    <h3 id="edit-account-title" className="text-lg font-semibold text-slate-900">Edit account</h3>
                    <p className="mt-1 text-sm text-slate-500"> Update the details of {account.name}.</p>
                </div>

                <form onSubmit={handleSubmit}>
                    <div className="space-y-5 px-6 py-6">
                        <div>
                            <label htmlFor="edit-account-name" className="text-sm font-medium text-slate-700">Account name *</label>
                            <input id="edit-account-name" required maxLength={100} value={name} onChange={(event) => setName(event.target.value)} className={inputClassName} />
                        </div>

                        <div>
                            <label htmlFor="edit-institution-name" className="text-sm font-medium text-slate-700">Institution / Provider </label>
                            <input id="edit-institution-name" maxLength={100} value={institutionName}
                                onChange={(event) =>
                                    setInstitutionName(event.target.value)
                                }
                                className={inputClassName}
                            />
                        </div>

                        <div>
                            <label htmlFor="edit-account-notes" className="text-sm font-medium text-slate-700">Notes</label>
                            <textarea id="edit-account-notes" rows={3} maxLength={1000} value={notes} onChange={(event) => setNotes(event.target.value)} className={inputClassName} />
                        </div>

                        {(formError || error) && (
                            <p role="alert" className="rounded-lg bg-red-50 px-4 py-3 text-sm text-red-700" > {formError || error} </p>
                        )}
                    </div>

                    <div className="flex justify-end gap-3 border-t border-slate-200 px-6 py-4">
                        <button type="button" onClick={onCancel} disabled={isSubmitting}
                            className="rounded-lg border border-slate-300 px-4 py-2.5 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                        >
                            Cancel
                        </button>

                        <button type="submit" disabled={isSubmitting}
                            className="rounded-lg bg-emerald-700 px-4 py-2.5 text-sm font-medium text-white hover:bg-emerald-800 disabled:opacity-50"
                        >
                            {isSubmitting ? "Saving..." : "Save changes"}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
