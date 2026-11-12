use crate::infrastructure::database::Database;
use chrono::{Datelike, Months, NaiveDate, Utc};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ForecastRequest {
    pub currency_code: String,
    pub period_months: u32,
}

#[derive(Deserialize, Serialize)]
pub struct ForecastSchedule {
    pub enabled: bool,
    pub currency_code: String,
    pub period_months: u32,
}

#[derive(Serialize)]
pub struct ForecastPeriod {
    pub month_start: String,
    pub predicted_income_minor: i64,
    pub predicted_expense_minor: i64,
    pub predicted_subscription_minor: i64,
    pub predicted_savings_minor: i64,
    pub predicted_ending_balance_minor: i64,
    pub planned_income_minor: Option<i64>,
    pub planned_expense_minor: Option<i64>,
    pub planned_savings_minor: Option<i64>,
    pub evaluation: Option<ForecastEvaluation>,
}

#[derive(Serialize)]
pub struct ForecastEvaluation {
    pub actual_income_minor: i64,
    pub actual_expense_minor: i64,
    pub actual_savings_minor: i64,
    pub income_variance_minor: i64,
    pub expense_variance_minor: i64,
    pub savings_variance_minor: i64,
    pub limitations: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ForecastAssumptions {
    pub history_start: String,
    pub history_end: String,
    pub active_months_with_activity: i64,
    pub average_income_minor: i64,
    pub average_non_subscription_expense_minor: i64,
    pub normalized_active_subscription_minor: i64,
    pub average_savings_contribution_minor: i64,
    pub starting_balance_minor: i64,
}

#[derive(Serialize)]
pub struct ForecastSnapshot {
    pub id: String,
    pub generated_at: String,
    pub currency_code: String,
    pub period_months: u32,
    pub confidence: String,
    pub assumptions: ForecastAssumptions,
    pub limitations: Vec<String>,
    pub periods: Vec<ForecastPeriod>,
}

pub struct SqliteForecastRepository<'a> {
    database: &'a Database,
}

impl<'a> SqliteForecastRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn generate(&self, request: &ForecastRequest) -> Result<String, String> {
        let today = chrono::DateTime::<Utc>::from(std::time::SystemTime::now()).date_naive();
        self.generate_as_of(request, today, None)
    }

    pub fn get_schedule(&self) -> Result<ForecastSchedule, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        connection
            .query_row(
                "SELECT enabled,currency_code,period_months FROM forecast_schedule WHERE id=1",
                [],
                |r| {
                    Ok(ForecastSchedule {
                        enabled: r.get::<_, i64>(0)? != 0,
                        currency_code: r.get(1)?,
                        period_months: r.get(2)?,
                    })
                },
            )
            .map_err(|e| e.to_string())
    }

    pub fn set_schedule(&self, schedule: &ForecastSchedule) -> Result<bool, String> {
        if !schedule.enabled {
            let connection = self
                .database
                .connection()
                .lock()
                .map_err(|_| "Database is unavailable")?;
            connection.execute("UPDATE forecast_schedule SET enabled=0,updated_at=CURRENT_TIMESTAMP WHERE id=1", []).map_err(|e|e.to_string())?;
            return Ok(false);
        }
        if schedule.currency_code != "KES"
            || schedule.currency_code.len() != 3
            || !schedule
                .currency_code
                .chars()
                .all(|c| c.is_ascii_uppercase())
            || !(1..=12).contains(&schedule.period_months)
        {
            return Err(
                "Choose a three-letter currency and a forecast between one and twelve months."
                    .into(),
            );
        }
        {
            let connection = self
                .database
                .connection()
                .lock()
                .map_err(|_| "Database is unavailable")?;
            connection.execute("UPDATE forecast_schedule SET enabled=?1,currency_code=?2,period_months=?3,updated_at=CURRENT_TIMESTAMP WHERE id=1",params![schedule.enabled as i64,schedule.currency_code,schedule.period_months]).map_err(|e|e.to_string())?;
        }
        if schedule.enabled {
            self.generate_scheduled_if_due()
        } else {
            Ok(false)
        }
    }

    pub fn generate_scheduled_if_due(&self) -> Result<bool, String> {
        let today = chrono::DateTime::<Utc>::from(std::time::SystemTime::now()).date_naive();
        self.generate_scheduled_as_of(today)
    }

    fn generate_scheduled_as_of(&self, today: NaiveDate) -> Result<bool, String> {
        let schedule = self.get_schedule()?;
        if !schedule.enabled || schedule.currency_code != "KES" {
            return Ok(false);
        }
        let scheduled_month = today
            .with_day(1)
            .ok_or("The current date is invalid.")?
            .to_string();
        {
            let connection = self
                .database
                .connection()
                .lock()
                .map_err(|_| "Database is unavailable")?;
            let already_generated: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM forecast_snapshots WHERE scheduled_month=?1)",
                    [&scheduled_month],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if already_generated {
                return Ok(false);
            }
        }
        self.generate_as_of(
            &ForecastRequest {
                currency_code: schedule.currency_code,
                period_months: schedule.period_months,
            },
            today,
            Some(&scheduled_month),
        )?;
        Ok(true)
    }

    fn generate_as_of(
        &self,
        request: &ForecastRequest,
        today: NaiveDate,
        scheduled_month: Option<&str>,
    ) -> Result<String, String> {
        if request.currency_code != "KES"
            || request.currency_code.len() != 3
            || !request
                .currency_code
                .chars()
                .all(|c| c.is_ascii_uppercase())
            || !(1..=12).contains(&request.period_months)
        {
            return Err(
                "Choose a three-letter currency and a forecast between one and twelve months."
                    .into(),
            );
        }
        let current_month = today.with_day(1).ok_or("The current date is invalid.")?;
        let history_start = current_month
            .checked_sub_months(Months::new(6))
            .ok_or("The forecast history date is out of range.")?;
        let history_end = current_month
            .pred_opt()
            .ok_or("The forecast history date is out of range.")?;
        let first_forecast_month = current_month
            .checked_add_months(Months::new(1))
            .ok_or("The forecast date is out of range.")?;
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let (income_sum, expense_sum, savings_sum, active_months): (i64,i64,i64,i64) = connection.query_row(
            "SELECT COALESCE(SUM(CASE WHEN t.transaction_type='income' AND NOT EXISTS(SELECT 1 FROM refund_transactions rf WHERE rf.transaction_id=t.id) THEN t.amount_minor ELSE 0 END),0),COALESCE(SUM(CASE WHEN t.transaction_type='expense' AND NOT EXISTS(SELECT 1 FROM subscription_payments p WHERE p.transaction_id=t.id) THEN t.amount_minor ELSE 0 END),0),COALESCE((SELECT SUM(c.amount_minor) FROM savings_contributions c JOIN savings_goals g ON g.id=c.goal_id WHERE g.currency_code=?3 AND date(c.contribution_date) BETWEEN ?1 AND ?2 AND (c.transaction_id IS NULL OR EXISTS(SELECT 1 FROM financial_transactions linked WHERE linked.id=c.transaction_id AND linked.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=linked.id AND reversal.status='posted')))),0),COUNT(DISTINCT strftime('%Y-%m',t.occurred_at)) FROM financial_transactions t WHERE t.status='posted' AND t.currency_code=?3 AND date(t.occurred_at) BETWEEN ?1 AND ?2 AND t.transaction_type IN ('income','expense') AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id AND reversal.status='posted')",
            params![history_start.to_string(),history_end.to_string(),request.currency_code],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())?;
        let average_income = income_sum / 6;
        let average_non_subscription_expense = expense_sum / 6;
        let average_savings = savings_sum / 6;
        let starting_balance: i64 = connection.query_row("SELECT COALESCE(SUM(e.amount_minor),0) FROM account_entries e JOIN accounts a ON a.id=e.account_id JOIN financial_transactions t ON t.id=e.transaction_id WHERE a.currency_code=?1 AND t.status='posted'", [&request.currency_code], |r|r.get(0)).map_err(|e|e.to_string())?;
        let mut subscription_statement=connection.prepare("SELECT amount_minor,frequency FROM subscriptions WHERE status='active' AND currency_code=?1").map_err(|e|e.to_string())?;
        let subscriptions = subscription_statement
            .query_map([&request.currency_code], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let monthly_subscription_cost: i64 = subscriptions
            .iter()
            .map(|(amount, frequency)| match frequency.as_str() {
                "weekly" => amount.saturating_mul(52) / 12,
                "monthly" => *amount,
                "quarterly" => amount / 3,
                "yearly" => amount / 12,
                _ => {
                    let mut parts = frequency.split(':');
                    let interval = if parts.next() == Some("custom") {
                        parts.next().and_then(|value| value.parse::<i64>().ok())
                    } else {
                        None
                    };
                    let unit = parts.next();
                    match (
                        interval.filter(|value| (1..=365).contains(value)),
                        unit,
                        parts.next(),
                    ) {
                        (Some(count), Some("days"), None) => amount.saturating_mul(30) / count,
                        (Some(count), Some("weeks"), None) => {
                            amount.saturating_mul(52) / (12 * count)
                        }
                        (Some(count), Some("months"), None) => amount / count,
                        (Some(count), Some("years"), None) => amount / (12 * count),
                        _ => 0,
                    }
                }
            })
            .fold(0_i64, i64::saturating_add);
        let confidence = if active_months < 3 {
            "limited"
        } else if active_months < 6 {
            "moderate"
        } else {
            "higher"
        }
        .to_string();
        let limitations=vec![
            "Uses arithmetic averages across the six completed months before generation; seasonal changes and future events are not modeled.".to_string(),
            "Active subscription costs are normalized to monthly amounts and added separately from historical subscription payments. Link paid expenses to their subscriptions to keep the historical baseline from including those costs.".to_string(),
            "Refunds are recorded separately and are not projected as ordinary income or future cash inflows.".to_string(),
            "Matching budget plans are shown as context and are not added to the projection, to avoid double-counting planned spending.".to_string(),
            "Coverage describes the number of months with posted activity; it is not a probability of forecast accuracy.".to_string(),
        ];
        let inputs = serde_json::json!({"currency_code":request.currency_code,"period_months":request.period_months,"history_months":6,"history_start":history_start.to_string(),"history_end":history_end.to_string(),"active_months_with_activity":active_months,"average_income_minor":average_income,"average_non_subscription_expense_minor":average_non_subscription_expense,"normalized_active_subscription_minor":monthly_subscription_cost,"average_savings_contribution_minor":average_savings,"starting_balance_minor":starting_balance,"method":"Six completed-month arithmetic average with separately normalized active subscriptions."});
        let id = Uuid::new_v4().to_string();
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO forecast_snapshots(id,currency_code,period_months,history_months,method,confidence,inputs_json,limitations_json,scheduled_month) VALUES (?1,?2,?3,6,?4,?5,?6,?7,?8)",params![id,request.currency_code,request.period_months,"Six completed-month arithmetic average with separately normalized active subscriptions.",confidence,inputs.to_string(),serde_json::to_string(&limitations).map_err(|e|e.to_string())?,scheduled_month]).map_err(|e|e.to_string())?;
        let mut balance = starting_balance;
        for index in 0..request.period_months {
            let month = first_forecast_month
                .checked_add_months(Months::new(index))
                .ok_or("The forecast date is out of range.")?;
            let month_start = month.to_string();
            let (planned_income,planned_expense,planned_savings):(Option<i64>,Option<i64>,Option<i64>)=tx.query_row("SELECT SUM(CASE WHEN item_type='income' THEN amount_minor END),SUM(CASE WHEN item_type='expense' THEN amount_minor END),SUM(CASE WHEN item_type='savings' THEN amount_minor END) FROM budget_periods bp LEFT JOIN budget_period_items i ON i.budget_id=bp.id WHERE bp.currency_code=?1 AND bp.starts_on=?2",params![request.currency_code,month_start],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
            let predicted_expense =
                average_non_subscription_expense.saturating_add(monthly_subscription_cost);
            balance = balance
                .saturating_add(average_income)
                .saturating_sub(predicted_expense);
            tx.execute("INSERT INTO forecast_periods(id,snapshot_id,month_start,predicted_income_minor,predicted_expense_minor,predicted_subscription_minor,predicted_savings_minor,predicted_ending_balance_minor,planned_income_minor,planned_expense_minor,planned_savings_minor) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![Uuid::new_v4().to_string(),id,month_start,average_income,predicted_expense,monthly_subscription_cost,average_savings,balance,planned_income,planned_expense,planned_savings]).map_err(|e|e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn list(&self) -> Result<Vec<ForecastSnapshot>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let mut snapshots_statement=connection.prepare("SELECT id,generated_at,currency_code,period_months,confidence,limitations_json,inputs_json FROM forecast_snapshots ORDER BY generated_at DESC,id DESC").map_err(|e|e.to_string())?;
        let snapshots = snapshots_statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, u32>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for (
            id,
            generated_at,
            currency_code,
            period_months,
            confidence,
            limitations_json,
            inputs_json,
        ) in snapshots
        {
            let limitations = serde_json::from_str(&limitations_json).unwrap_or_default();
            let assumptions: ForecastAssumptions =
                serde_json::from_str(&inputs_json).map_err(|e| e.to_string())?;
            let mut periods_statement=connection.prepare("SELECT p.month_start,p.predicted_income_minor,p.predicted_expense_minor,p.predicted_subscription_minor,p.predicted_savings_minor,p.predicted_ending_balance_minor,p.planned_income_minor,p.planned_expense_minor,p.planned_savings_minor,e.actual_income_minor,e.actual_expense_minor,e.actual_savings_minor,e.income_variance_minor,e.expense_variance_minor,e.savings_variance_minor,e.limitations_json FROM forecast_periods p LEFT JOIN forecast_evaluations e ON e.snapshot_id=p.snapshot_id AND e.month_start=p.month_start WHERE p.snapshot_id=?1 ORDER BY p.month_start").map_err(|e|e.to_string())?;
            let periods = periods_statement
                .query_map([&id], |r| {
                    let actual_income: Option<i64> = r.get(9)?;
                    let evaluation = if let Some(actual_income_minor) = actual_income {
                        Some(ForecastEvaluation {
                            actual_income_minor,
                            actual_expense_minor: r.get(10)?,
                            actual_savings_minor: r.get(11)?,
                            income_variance_minor: r.get(12)?,
                            expense_variance_minor: r.get(13)?,
                            savings_variance_minor: r.get(14)?,
                            limitations: serde_json::from_str(&r.get::<_, String>(15)?)
                                .unwrap_or_default(),
                        })
                    } else {
                        None
                    };
                    Ok(ForecastPeriod {
                        month_start: r.get(0)?,
                        predicted_income_minor: r.get(1)?,
                        predicted_expense_minor: r.get(2)?,
                        predicted_subscription_minor: r.get(3)?,
                        predicted_savings_minor: r.get(4)?,
                        predicted_ending_balance_minor: r.get(5)?,
                        planned_income_minor: r.get(6)?,
                        planned_expense_minor: r.get(7)?,
                        planned_savings_minor: r.get(8)?,
                        evaluation,
                    })
                })
                .map_err(|e| e.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| e.to_string())?;
            result.push(ForecastSnapshot {
                id,
                generated_at,
                currency_code,
                period_months,
                confidence,
                assumptions,
                limitations,
                periods,
            });
        }
        Ok(result)
    }

    pub fn evaluate(&self, snapshot_id: &str, month_start: &str) -> Result<(), String> {
        let today = chrono::DateTime::<Utc>::from(std::time::SystemTime::now()).date_naive();
        self.evaluate_as_of(snapshot_id, month_start, today)
    }

    fn evaluate_as_of(
        &self,
        snapshot_id: &str,
        month_start: &str,
        today: NaiveDate,
    ) -> Result<(), String> {
        let month = NaiveDate::parse_from_str(month_start, "%Y-%m-%d")
            .map_err(|_| "Choose a valid forecast month.")?;
        if month.day() != 1 {
            return Err("Choose the first day of a forecast month.".into());
        }
        let next = month
            .checked_add_months(Months::new(1))
            .ok_or("The evaluation month is out of range.")?;
        let last = next
            .pred_opt()
            .ok_or("The evaluation month is out of range.")?;
        if last > today {
            return Err("A forecast can be evaluated after its month has ended.".into());
        }
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let (currency,predicted_income,predicted_expense,predicted_savings):(String,i64,i64,i64)=connection.query_row("SELECT s.currency_code,p.predicted_income_minor,p.predicted_expense_minor,p.predicted_savings_minor FROM forecast_snapshots s JOIN forecast_periods p ON p.snapshot_id=s.id WHERE s.id=?1 AND p.month_start=?2",params![snapshot_id,month_start],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e|e.to_string())?.ok_or("The forecast month no longer exists.")?;
        let (actual_income,actual_expense,actual_savings):(i64,i64,i64)=connection.query_row("SELECT COALESCE(SUM(CASE WHEN t.transaction_type='income' AND NOT EXISTS(SELECT 1 FROM refund_transactions rf WHERE rf.transaction_id=t.id) THEN t.amount_minor ELSE 0 END),0),COALESCE(SUM(CASE WHEN t.transaction_type='expense' THEN t.amount_minor ELSE 0 END),0),COALESCE((SELECT SUM(c.amount_minor) FROM savings_contributions c JOIN savings_goals g ON g.id=c.goal_id AND g.currency_code=?3 WHERE date(c.contribution_date) BETWEEN ?1 AND ?2 AND (c.transaction_id IS NULL OR EXISTS(SELECT 1 FROM financial_transactions linked WHERE linked.id=c.transaction_id AND linked.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=linked.id AND r.status='posted')))),0) FROM financial_transactions t WHERE t.currency_code=?3 AND t.status='posted' AND date(t.occurred_at) BETWEEN ?1 AND ?2 AND t.transaction_type IN ('income','expense') AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted')",params![month_start,last.to_string(),currency],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
        let limitations=serde_json::to_string(&vec!["Evaluation covers posted transactions and recorded savings contributions for the month; unrecorded activity is not included."]).map_err(|e|e.to_string())?;
        connection.execute("INSERT INTO forecast_evaluations(id,snapshot_id,month_start,actual_income_minor,actual_expense_minor,actual_savings_minor,income_variance_minor,expense_variance_minor,savings_variance_minor,limitations_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![Uuid::new_v4().to_string(),snapshot_id,month_start,actual_income,actual_expense,actual_savings,actual_income-predicted_income,actual_expense-predicted_expense,actual_savings-predicted_savings,limitations]).map_err(|e|if e.to_string().contains("UNIQUE") {"This forecast month has already been evaluated.".into()} else {e.to_string()})?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture() -> Database {
        let db = Database::initialize(Path::new(":memory:")).unwrap();
        let c = db.connection().lock().unwrap();
        c.execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('cash','Cash','cash','KES')",[]).unwrap();
        for (id, kind, amount, date) in [
            ("income", "income", 30000, "2026-06-10"),
            ("expense", "expense", 12000, "2026-06-12"),
            ("subscription", "expense", 5000, "2026-06-15"),
            ("income2", "income", 30000, "2026-07-10"),
            ("expense2", "expense", 12000, "2026-07-12"),
        ] {
            c.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES (?1,?2,?3,'KES',?4,'manual')",params![id,kind,amount,format!("{date}T10:00:00Z")]).unwrap();
            let signed = if kind == "income" { amount } else { -amount };
            c.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES (?1,?2,'cash',?3)",params![format!("entry-{id}"),id,signed]).unwrap();
        }
        c.execute("INSERT INTO subscriptions(id,name,amount_minor,currency_code,frequency,next_due_date) VALUES ('monthly','Music',9000,'KES','monthly','2026-11-20'),('yearly','Domain',12000,'KES','yearly','2026-11-10'),('custom','Quarterly',6000,'KES','custom:3:months','2026-11-15')",[]).unwrap();
        c.execute("INSERT INTO budget_periods(id,name,currency_code,starts_on,ends_on) VALUES ('future-budget','December','KES','2026-12-01','2026-12-31')",[]).unwrap();
        c.execute("INSERT INTO budget_period_items(id,budget_id,item_type,amount_minor,label) VALUES ('planned-income','future-budget','income',40000,'Salary'),('planned-expense','future-budget','expense',20000,'Household'),('planned-savings','future-budget','savings',5000,'Reserve')",[]).unwrap();
        drop(c);
        db
    }

    #[test]
    fn saves_independent_offline_forecast_snapshots_with_assumptions_and_budget_context() {
        let db = fixture();
        let repo = SqliteForecastRepository::new(&db);
        let request = ForecastRequest {
            currency_code: "KES".into(),
            period_months: 2,
        };
        assert!(
            repo.generate_as_of(
                &ForecastRequest {
                    currency_code: "kes".into(),
                    period_months: 2
                },
                NaiveDate::from_ymd_opt(2026, 11, 7).unwrap(),
                None
            )
            .is_err()
        );
        assert!(
            repo.generate_as_of(
                &ForecastRequest {
                    currency_code: "USD".into(),
                    period_months: 1
                },
                NaiveDate::from_ymd_opt(2026, 11, 7).unwrap(),
                None
            )
            .is_err()
        );
        assert!(
            repo.generate_as_of(
                &ForecastRequest {
                    currency_code: "KES".into(),
                    period_months: 13
                },
                NaiveDate::from_ymd_opt(2026, 11, 7).unwrap(),
                None
            )
            .is_err()
        );
        let id = repo
            .generate_as_of(
                &request,
                NaiveDate::from_ymd_opt(2026, 11, 7).unwrap(),
                None,
            )
            .unwrap();
        let second = repo
            .generate_as_of(
                &request,
                NaiveDate::from_ymd_opt(2026, 11, 7).unwrap(),
                None,
            )
            .unwrap();
        assert_ne!(id, second);
        let snapshots = repo.list().unwrap();
        assert_eq!(snapshots.len(), 2);
        let snapshot = snapshots.iter().find(|s| s.id == id).unwrap();
        assert_eq!(snapshot.confidence, "limited");
        assert_eq!(snapshot.periods.len(), 2);
        assert_eq!(snapshot.periods[0].predicted_income_minor, 10000);
        assert_eq!(snapshot.periods[0].predicted_subscription_minor, 12000);
        assert_eq!(snapshot.periods[0].planned_income_minor, Some(40000));
        assert!(
            snapshot
                .limitations
                .iter()
                .any(|x| x.contains("arithmetic averages"))
        );
    }

    #[test]
    fn forecast_evaluation_is_preserved_and_rejects_future_months() {
        let db = fixture();
        let repo = SqliteForecastRepository::new(&db);
        let id = repo
            .generate_as_of(
                &ForecastRequest {
                    currency_code: "KES".into(),
                    period_months: 1,
                },
                NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
                None,
            )
            .unwrap();
        {
            let connection = db.connection().lock().unwrap();
            connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('evaluation-income','income',30000,'KES','2026-02-10T10:00:00Z','manual'),('evaluation-expense','expense',5000,'KES','2026-02-12T10:00:00Z','manual')",[]).unwrap();
            connection.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('evaluation-entry-income','evaluation-income','cash',30000),('evaluation-entry-expense','evaluation-expense','cash',-5000)",[]).unwrap();
        }
        let as_of = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        repo.evaluate_as_of(&id, "2026-02-01", as_of).unwrap();
        assert!(repo.evaluate_as_of(&id, "2026-02-01", as_of).is_err());
        assert!(repo.evaluate_as_of(&id, "2026-03-01", as_of).is_err());
        let snapshot = repo.list().unwrap().remove(0);
        let evaluation = snapshot.periods[0].evaluation.as_ref().unwrap();
        assert_eq!(
            (
                evaluation.actual_income_minor,
                evaluation.actual_expense_minor
            ),
            (30000, 5000)
        );
        assert_eq!(evaluation.income_variance_minor, 30000);
        let transactions: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(transactions, 7);
    }

    #[test]
    fn scheduled_generation_is_opt_in_and_idempotent_per_month() {
        let db = fixture();
        let repo = SqliteForecastRepository::new(&db);
        let november = NaiveDate::from_ymd_opt(2026, 11, 7).unwrap();
        assert!(!repo.generate_scheduled_as_of(november).unwrap());
        db.connection().lock().unwrap().execute("UPDATE forecast_schedule SET enabled=1,currency_code='KES',period_months=2 WHERE id=1",[]).unwrap();
        assert!(repo.generate_scheduled_as_of(november).unwrap());
        assert!(!repo.generate_scheduled_as_of(november).unwrap());
        let count: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM forecast_snapshots WHERE scheduled_month='2026-11-01'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert!(
            repo.generate_scheduled_as_of(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap())
                .unwrap()
        );
    }
}
