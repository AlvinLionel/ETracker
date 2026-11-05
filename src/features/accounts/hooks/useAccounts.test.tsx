import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { archiveAccount, createAccount, listActiveAccounts, reactivateAccount, updateAccount } from "../api/accounts";
import { useAccounts } from "./useAccounts";
import type { Account } from "../types";

vi.mock("../api/accounts", () => ({
    archiveAccount: vi.fn(),
    createAccount: vi.fn(),
    listActiveAccounts: vi.fn(),
    reactivateAccount: vi.fn(),
    updateAccount: vi.fn(),
}));

const mockAccount: Account = {
    id: "account-001",
    name: "Personal M-Pesa",
    account_type: "mobile_money",
    institution_name: "Safaricom",
    currency_code: "KES",
    notes: null,
    created_at: "2026-11-05T07:00:00.000Z",
    updated_at: "2026-11-05T07:00:00.000Z",
    archived_at: null,
};

describe("useAccounts", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.mocked(listActiveAccounts).mockResolvedValue([]);
    });

    it("loads active accounts on initialization", async () => {
        vi.mocked(listActiveAccounts).mockResolvedValue([mockAccount]);

        const { result } = renderHook(() => useAccounts());

        expect(result.current.isLoading).toBe(true);

        await waitFor(() => {
            expect(result.current.isLoading).toBe(false);
        });

        expect(result.current.accounts).toEqual([mockAccount]);
        expect(result.current.error).toBeNull();
    });

    it("creates an account and updates local state", async () => {
        vi.mocked(createAccount).mockResolvedValue(mockAccount);

        const { result } = renderHook(() => useAccounts());

        await waitFor(() => {
            expect(result.current.isLoading).toBe(false);
        });
        await act(async () => {
            await result.current.create({
                name: "Personal M-Pesa",
                account_type: "mobile_money",
                institution_name: "Safaricom",
            });
        });

        expect(result.current.accounts).toEqual([mockAccount]);
        expect(createAccount).toHaveBeenCalledOnce();
        expect(result.current.error).toBeNull();
    });

    it("updates an existing account", async () => {
        const updatedAccount = {
            ...mockAccount,
            name: "Updated M-Pesa",
        };

        vi.mocked(listActiveAccounts).mockResolvedValue([mockAccount]);
        vi.mocked(updateAccount).mockResolvedValue(updatedAccount);

        const { result } = renderHook(() => useAccounts());

        await waitFor(() => {
            expect(result.current.accounts).toHaveLength(1);
        });
        await act(async () => {
            await result.current.update(mockAccount.id, {
                name: "Updated M-Pesa",
            });
        });

        expect(result.current.accounts[0].name).toBe("Updated M-Pesa");
    });

    it("removes an account from active state after archiving", async () => {
        vi.mocked(listActiveAccounts).mockResolvedValue([mockAccount]);
        vi.mocked(archiveAccount).mockResolvedValue({
            ...mockAccount,
            archived_at: "2026-11-05T08:00:00.000Z",
        });

        const { result } = renderHook(() => useAccounts());

        await waitFor(() => {
            expect(result.current.accounts).toHaveLength(1);
        });
        await act(async () => {
            await result.current.archive(mockAccount.id);
        });

        expect(result.current.accounts).toHaveLength(0);
    });

    it("adds a reactivated account to active state", async () => {
        vi.mocked(reactivateAccount).mockResolvedValue(mockAccount);

        const { result } = renderHook(() => useAccounts());

        await waitFor(() => {
            expect(result.current.isLoading).toBe(false);
        });
        await act(async () => {
            await result.current.reactivate(mockAccount.id);
        });

        expect(result.current.accounts).toEqual([mockAccount]);
    });

    it("stores and rethrows errors when account creation fails", async () => {
        vi.mocked(createAccount).mockRejectedValue(
            new Error("Account creation failed."),
        );

        const { result } = renderHook(() => useAccounts());

        await waitFor(() => {
            expect(result.current.isLoading).toBe(false);
        });
        await act(async () => {
            await expect(
                result.current.create({
                    name: "Personal M-Pesa",
                    account_type: "mobile_money",
                }),
            ).rejects.toThrow("Account creation failed.");
        });

        expect(result.current.error).toBe("Account creation failed.");
        expect(result.current.accounts).toHaveLength(0);
        expect(result.current.isMutating).toBe(false);
    });

    it("rejects overlapping account mutations", async () => {
        let resolveCreation!: (account: Account) => void;
        vi.mocked(createAccount).mockImplementation(() =>
            new Promise<Account>((resolve) => {
                resolveCreation = resolve;
            }),
        );

        const { result } = renderHook(() => useAccounts());

        await waitFor(() => {
            expect(result.current.isLoading).toBe(false);
        });

        let firstOperation!: Promise<Account>;

        act(() => {
            firstOperation = result.current.create({
                name: "Personal M-Pesa",
                account_type: "mobile_money",
            });
        });

        expect(result.current.isMutating).toBe(true);

        await act(async () => {
            await expect(
                result.current.create({
                    name: "Second M-Pesa",
                    account_type: "mobile_money",
                }),
            ).rejects.toThrow(
                "Another account operation is already in progress.",
            );
        });
        await act(async () => {
            resolveCreation(mockAccount);
            await firstOperation;
        });

        expect(result.current.accounts).toEqual([mockAccount]);
        expect(result.current.isMutating).toBe(false);
        expect(createAccount).toHaveBeenCalledOnce();
    });
});