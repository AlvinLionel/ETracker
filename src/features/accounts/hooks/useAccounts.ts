import { useCallback, useEffect, useRef, useState } from "react";
import { archiveAccount, createAccount, getAccountBalance, listActiveAccounts, listArchivedAccounts, reactivateAccount, updateAccount } from "../api/accounts"; import type { Account, CreateAccountInput, UpdateAccountInput } from "../types";

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
    const [loadError, setLoadError] = useState<string | null>(null);
    const [archivedAccounts, setArchivedAccounts] = useState<Account[]>([]);
    const [isArchivedLoading, setIsArchivedLoading] = useState(false);
    const [archivedLoadError, setArchivedLoadError] = useState<string | null>(null);

    const mutationLock = useRef(false);

    const refresh = useCallback(async () => {
        setIsLoading(true);
        setLoadError(null);

        try {
            const result = await listActiveAccounts();
            setAccounts(result);
        } catch (error) {
            setLoadError(getErrorMessage(error));
            throw error;
        } finally {
            setIsLoading(false);
        }
    }, []);

    const refreshArchived = useCallback(async () => {
        setIsArchivedLoading(true);
        setArchivedLoadError(null);

        try {
            const result = await listArchivedAccounts();
            setArchivedAccounts(result);
        } catch (error) {
            setArchivedLoadError(getErrorMessage(error));
            throw error;
        } finally {
            setIsArchivedLoading(false);
        }
    }, []);

    const getBalance = useCallback(async (id: string): Promise<number> => {
        try {
            return await getAccountBalance(id);
        } catch (error) {
            setError(getErrorMessage(error));
            throw error;
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
        (id: string, expectedBalanceMinor: number): Promise<Account> =>
            runMutation(async () => {
                const archivedAccount = await archiveAccount(id, expectedBalanceMinor);

                if (!archivedAccount) throw new Error("Account could not be found or archived.");

                setAccounts((current) =>
                    current.filter((item) => item.id !== id),
                );
                setArchivedAccounts((current) => [
                    archivedAccount,
                    ...current.filter((item) => item.id !== id),
                ]);

                return archivedAccount;
            }),
        [runMutation],
    );

    const reactivate = useCallback(
        (id: string): Promise<Account> =>
            runMutation(async () => {
                const account = await reactivateAccount(id);

                if (!account) throw new Error("Account could not be found or reactivated.");

                setArchivedAccounts((current) =>
                    current.filter((item) => item.id !== id),
                );
                setAccounts((current) => [
                    account,
                    ...current.filter((item) => item.id !== id),
                ]);

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
        accounts, isLoading, isMutating, error, loadError, archivedAccounts, isArchivedLoading, archivedLoadError,
        refresh, create, update, archive, reactivate, refreshArchived, getBalance,
    };
}