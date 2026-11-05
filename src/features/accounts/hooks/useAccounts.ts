import { useCallback, useEffect, useRef, useState } from "react";

import { archiveAccount, createAccount, listActiveAccounts, reactivateAccount, updateAccount } from "../api/accounts";

import type { Account, CreateAccountInput, UpdateAccountInput } from "../types";

function getErrorMessage(error: unknown): string {
    if (typeof error === "string") return error;

    if (error instanceof Error) return error.message;

    return "An unexpected account operation error occurred.";
}

export function useAccounts() {
    const [accounts, setAccounts] = useState<Account[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [isMutating, setIsMutating] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const mutationLock = useRef(false);

    const refresh = useCallback(async () => {
        setIsLoading(true);
        setError(null);

        try {
            const result = await listActiveAccounts();
            setAccounts(result);
        } catch (error) {
            setError(getErrorMessage(error));
            throw error;
        } finally {
            setIsLoading(false);
        }
    }, []);

    const runMutation = useCallback(
        async <T,>(operation: () => Promise<T>): Promise<T> => {
            if (mutationLock.current) {
                const message = "Another account operation is already in progress.";
                setError(message);
                throw new Error(message);
            }

            mutationLock.current = true;
            setIsMutating(true);
            setError(null);

            try {
                return await operation();
            } catch (error) {
                setError(getErrorMessage(error));
                throw error;
            } finally {
                mutationLock.current = false;
                setIsMutating(false);
            }
        },
        [],
    );

    const create = useCallback(
        (request: CreateAccountInput): Promise<Account> =>
            runMutation(async () => {
                const account = await createAccount(request);
                setAccounts((current) => [account, ...current]);
                return account;
            }),
        [runMutation],
    );

    const update = useCallback(
        (id: string, request: UpdateAccountInput): Promise<Account> =>
            runMutation(async () => {
                const account = await updateAccount(id, request);

                if (!account) throw new Error("Account could not be found or updated.");

                setAccounts((current) =>
                    current.map((item) => (item.id === id ? account : item)),
                );

                return account;
            }),
        [runMutation],
    );

    const archive = useCallback(
        (id: string): Promise<Account> =>
            runMutation(async () => {
                const account = await archiveAccount(id);

                if (!account) throw new Error("Account could not be found or archived.");

                setAccounts((current) =>
                    current.filter((item) => item.id !== id),
                );

                return account;
            }),
        [runMutation],
    );

    const reactivate = useCallback(
        (id: string): Promise<Account> =>
            runMutation(async () => {
                const account = await reactivateAccount(id);

                if (!account) throw new Error("Account could not be found or reactivated.");

                setAccounts((current) => [account, ...current]);
                return account;
            }),
        [runMutation],
    );

    useEffect(() => {
        void refresh().catch(() => {
            // The error is already stored in hook state.
        });
    }, [refresh]);

    return {
        accounts,
        isLoading,
        isMutating,
        error,
        refresh,
        create,
        update,
        archive,
        reactivate,
    };
}