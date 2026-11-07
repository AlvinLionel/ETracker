import { invoke } from "@tauri-apps/api/core";

export interface ForecastEvaluation { actual_income_minor:number; actual_expense_minor:number; actual_savings_minor:number; income_variance_minor:number; expense_variance_minor:number; savings_variance_minor:number; limitations:string[] }
export interface ForecastAssumptions { history_start:string; history_end:string; active_months_with_activity:number; average_income_minor:number; average_non_subscription_expense_minor:number; normalized_active_subscription_minor:number; average_savings_contribution_minor:number; starting_balance_minor:number }
export interface ForecastPeriod { month_start:string; predicted_income_minor:number; predicted_expense_minor:number; predicted_subscription_minor:number; predicted_savings_minor:number; predicted_ending_balance_minor:number; planned_income_minor:number|null; planned_expense_minor:number|null; planned_savings_minor:number|null; evaluation:ForecastEvaluation|null }
export interface ForecastSnapshot { id:string; generated_at:string; currency_code:string; period_months:number; confidence:string; assumptions:ForecastAssumptions; limitations:string[]; periods:ForecastPeriod[] }
export interface ForecastSchedule { enabled:boolean; currency_code:string; period_months:number }

export const listForecasts=()=>invoke<ForecastSnapshot[]>("list_forecasts");
export const generateForecast=(request:{currency_code:string;period_months:number})=>invoke<string>("generate_forecast",{request});
export const evaluateForecast=(snapshotId:string,monthStart:string)=>invoke<void>("evaluate_forecast",{snapshotId,monthStart});
export const getForecastSchedule=()=>invoke<ForecastSchedule>("get_forecast_schedule");
export const setForecastSchedule=(schedule:ForecastSchedule)=>invoke<boolean>("set_forecast_schedule",{schedule});
