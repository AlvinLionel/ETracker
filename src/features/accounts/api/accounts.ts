import { invoke } from "@tauri-apps/api/core";

import type { Account, CreateAccountInput, UpdateAccountInput } from "../types";

export async function createAccount(request: CreateAccountInput): Promise<Account> {
    return invoke<Account>("create_account", { request });
}

export async function getAccount(id: string): Promise<Account | null> {
    return invoke<Account | null>("get_account", { id });
}

export async function listActiveAccounts(): Promise<Account[]> {
    return invoke<Account[]>("list_active_accounts");
}

export async function updateAccount(id: string, request: UpdateAccountInput): Promise<Account | null> {
    return invoke<Account | null>("update_account", { id, request });
}

export async function archiveAccount(id: string, expectedBalanceMinor: number): Promise<Account | null> {
    return invoke<Account | null>("archive_account", { id, expectedBalanceMinor });
}

export async function reactivateAccount(id: string): Promise<Account | null> {
    return invoke<Account | null>("reactivate_account", { id });
}
export async function listArchivedAccounts(): Promise<Account[]> {
    return invoke<Account[]>("list_archived_accounts");
}
export async function getAccountBalance(id: string): Promise<number> {
    return invoke<number>("get_account_balance", { id });
}