use crate::{
    infrastructure::database::Database,
    persistence::planning::{
        BudgetLine, BudgetPeriod, BudgetTemplate, PlannedItem, SqlitePlanningRepository,
    },
};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub currency_code: String,
    pub items: Vec<PlannedItem>,
}

#[tauri::command]
pub fn list_budget_templates(state: State<'_, Database>) -> Result<Vec<BudgetTemplate>, String> {
    SqlitePlanningRepository::new(state.inner()).list_templates()
}

#[tauri::command]
pub fn create_budget_template(
    state: State<'_, Database>,
    request: CreateTemplateRequest,
) -> Result<String, String> {
    SqlitePlanningRepository::new(state.inner()).create_template(
        &request.name,
        &request.currency_code,
        &request.items,
    )
}

#[tauri::command]
pub fn create_budget_period(
    state: State<'_, Database>,
    template_id: String,
    month: String,
) -> Result<String, String> {
    SqlitePlanningRepository::new(state.inner()).create_period(&template_id, &month)
}

#[tauri::command]
pub fn list_budget_periods(state: State<'_, Database>) -> Result<Vec<BudgetPeriod>, String> {
    SqlitePlanningRepository::new(state.inner()).list_periods()
}

#[tauri::command]
pub fn get_budget_period_items(
    state: State<'_, Database>,
    budget_id: String,
) -> Result<Vec<BudgetLine>, String> {
    SqlitePlanningRepository::new(state.inner()).period_items(&budget_id)
}
