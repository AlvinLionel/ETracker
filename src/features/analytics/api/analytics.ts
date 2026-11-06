import { invoke } from "@tauri-apps/api/core";

export interface AnalyticsFilterOption { id: string; name: string; currency_code: string | null }
export interface AnalyticsPoint { period: string; income_minor: number; expense_minor: number }
export interface AnalyticsCategory { category_id: string | null; name: string; transaction_type: string; amount_minor: number }
export interface AnalyticsAccount { account_id: string; name: string; income_minor: number; expense_minor: number }
export interface AnalyticsSummary {
  currencies: string[];
  accounts: AnalyticsFilterOption[];
  categories: AnalyticsFilterOption[];
  total_income_minor: number;
  total_expense_minor: number;
  trends: AnalyticsPoint[];
  category_breakdown: AnalyticsCategory[];
  account_comparison: AnalyticsAccount[];
  recurring_expenses: AnalyticsCategory[];
}

export function getAnalyticsSummary(filters: { from: string; to: string; accountId: string; categoryId: string; currencyCode: string }) {
  return invoke<AnalyticsSummary>("get_analytics_summary", {
    from: filters.from || null,
    to: filters.to || null,
    accountId: filters.accountId || null,
    categoryId: filters.categoryId || null,
    currencyCode: filters.currencyCode || null,
  });
}
