import { invoke } from "@tauri-apps/api/core";
import type { CreateTransactionInput, TransactionRecord } from "../types";
export const createTransaction=(request:CreateTransactionInput)=>invoke<string>("create_transaction",{request});
export const listTransactions=(accountId:string,from:string,to:string)=>invoke<TransactionRecord[]>("list_transactions",{accountId:accountId||null,from:from||null,to:to||null});
