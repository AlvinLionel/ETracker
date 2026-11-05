export type AccountType = "mobile_money" | "bank" | "cash" | "other";

export interface Account {
  id: string;
  name: string;
  account_type: AccountType;
  institution_name: string | null;
  currency_code: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

export interface OpeningBalanceInput {
  amount_minor: number;
  occurred_at: string;
}

export interface CreateAccountInput {
  name: string;
  account_type: AccountType;
  institution_name?: string | null;
  notes?: string | null;
  opening_balance?: OpeningBalanceInput | null;
}

export interface UpdateAccountInput {
  name: string;
  institution_name?: string | null;
  notes?: string | null;
}