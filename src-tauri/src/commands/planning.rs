use crate::{
    infrastructure::database::Database,
    persistence::planning::{
        BudgetLine, BudgetPeriod, BudgetTemplate, EligibleSavingsTransfer, PlannedItem,
        SavingsGoal, SqlitePlanningRepository, Subscription, SubscriptionInput,
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

#[derive(Deserialize)]
pub struct CreateSavingsGoalRequest {
    pub name: String,
    pub target_amount_minor: i64,
    pub currency_code: String,
    pub target_date: Option<String>,
    pub description: Option<String>,
    pub account_id: Option<String>,
}

#[derive(Deserialize)]
pub struct SavingsContributionRequest {
    pub goal_id: String,
    pub amount_minor: i64,
    pub contribution_date: String,
    pub description: Option<String>,
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

#[tauri::command]
pub fn list_savings_goals(state: State<'_, Database>) -> Result<Vec<SavingsGoal>, String> {
    SqlitePlanningRepository::new(state.inner()).list_savings_goals()
}

#[tauri::command]
pub fn create_savings_goal(
    state: State<'_, Database>,
    request: CreateSavingsGoalRequest,
) -> Result<String, String> {
    SqlitePlanningRepository::new(state.inner()).create_savings_goal(
        &request.name,
        request.target_amount_minor,
        &request.currency_code,
        request.target_date.as_deref(),
        request.description.as_deref(),
        request.account_id.as_deref(),
    )
}

#[tauri::command]
pub fn list_eligible_savings_transfers(
    state: State<'_, Database>,
    goal_id: String,
) -> Result<Vec<EligibleSavingsTransfer>, String> {
    SqlitePlanningRepository::new(state.inner()).eligible_savings_transfers(&goal_id)
}

#[tauri::command]
pub fn add_savings_transfer(
    state: State<'_, Database>,
    goal_id: String,
    transaction_id: String,
) -> Result<(), String> {
    SqlitePlanningRepository::new(state.inner()).add_savings_transfer(&goal_id, &transaction_id)
}

#[tauri::command]
pub fn add_savings_contribution(
    state: State<'_, Database>,
    request: SavingsContributionRequest,
) -> Result<(), String> {
    SqlitePlanningRepository::new(state.inner()).add_savings_contribution(
        &request.goal_id,
        request.amount_minor,
        &request.contribution_date,
        request.description.as_deref(),
    )
}

#[tauri::command]
pub fn set_savings_goal_status(
    state: State<'_, Database>,
    goal_id: String,
    status: String,
) -> Result<(), String> {
    SqlitePlanningRepository::new(state.inner()).set_savings_goal_status(&goal_id, &status)
}

#[tauri::command]
pub fn list_subscriptions(state: State<'_, Database>) -> Result<Vec<Subscription>, String> {
    SqlitePlanningRepository::new(state.inner()).list_subscriptions()
}

#[tauri::command]
pub fn create_subscription(
    state: State<'_, Database>,
    request: SubscriptionInput,
) -> Result<String, String> {
    SqlitePlanningRepository::new(state.inner()).create_subscription(&request)
}

#[tauri::command]
pub fn update_subscription(
    state: State<'_, Database>,
    id: String,
    request: SubscriptionInput,
) -> Result<(), String> {
    SqlitePlanningRepository::new(state.inner()).update_subscription(&id, &request)
}

#[tauri::command]
pub fn set_subscription_status(
    state: State<'_, Database>,
    id: String,
    status: String,
) -> Result<(), String> {
    SqlitePlanningRepository::new(state.inner()).set_subscription_status(&id, &status)
}
