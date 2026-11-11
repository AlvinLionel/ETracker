import { invoke } from "@tauri-apps/api/core";

export interface SavedAnalyticsReport {
  id: string;
  name: string;
  from_date: string | null;
  to_date: string | null;
  account_id: string | null;
  category_id: string | null;
  currency_code: string | null;
  created_at: string;
  updated_at: string;
}

export type SavedAnalyticsReportInput = Omit<SavedAnalyticsReport, "id" | "created_at" | "updated_at">;

export const listSavedAnalyticsReports = () => invoke<SavedAnalyticsReport[]>("list_saved_analytics_reports");
export const saveAnalyticsReport = (request: SavedAnalyticsReportInput) => invoke<SavedAnalyticsReport>("save_analytics_report", { request });
export const deleteSavedAnalyticsReport = (reportId: string) => invoke<boolean>("delete_saved_analytics_report", { reportId });
