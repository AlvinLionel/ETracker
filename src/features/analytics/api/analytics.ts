import { invoke } from "@tauri-apps/api/core";

export interface AnalyticsFilterOption { id: string; name: string; currency_code: string | null }
export interface AnalyticsPoint { period: string; income_minor: number; expense_minor: number; refund_minor:number }
export interface AnalyticsCategory { category_id: string | null; name: string; transaction_type: string; amount_minor: number }
export interface AnalyticsAccount { account_id: string; name: string; income_minor: number; expense_minor: number; refund_minor:number }
export interface UnusualExpense { transaction_id: string; occurred_at: string; description: string | null; category_name: string; amount_minor: number; recent_average_minor: number; comparison_count: number }
export interface BudgetPerformance { budget_id: string; name: string; starts_on: string; ends_on: string; planned_income_minor: number; planned_expense_minor: number; planned_savings_minor: number; actual_income_minor: number; actual_expense_minor: number; actual_savings_minor: number }
export interface SavingsProgress { goal_id: string; name: string; target_amount_minor: number; contributed_minor: number; status: string }
export interface AnalyticsSummary {
  currencies: string[];
  accounts: AnalyticsFilterOption[];
  categories: AnalyticsFilterOption[];
  total_income_minor: number;
  total_expense_minor: number;
  total_refund_minor:number;
  total_net_spending_minor:number;
  trends: AnalyticsPoint[];
  category_breakdown: AnalyticsCategory[];
  account_comparison: AnalyticsAccount[];
  recurring_expenses: AnalyticsCategory[];
  unusual_expenses: UnusualExpense[];
  budget_performance: BudgetPerformance[];
  savings_progress: SavingsProgress[];
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
