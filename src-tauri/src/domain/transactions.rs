use chrono::DateTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionType {
    Income,
    Expense,
}
impl TransactionType {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "income" => Some(Self::Income),
            "expense" => Some(Self::Expense),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
        }
    }
    pub fn entry_amount(self, amount: i64) -> i64 {
        match self {
            Self::Income => amount,
            Self::Expense => -amount,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allocation {
    pub category_id: String,
    pub amount_minor: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewTransaction {
    pub account_id: String,
    pub transaction_type: TransactionType,
    pub amount_minor: i64,
    pub currency_code: String,
    pub occurred_at: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub allocations: Vec<Allocation>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionValidationError {
    EmptyAccount,
    InvalidAmount,
    InvalidCurrency,
    InvalidDate,
    InvalidAllocation,
}
impl NewTransaction {
    pub fn validate(&self) -> Result<(), TransactionValidationError> {
        if self.account_id.trim().is_empty() {
            return Err(TransactionValidationError::EmptyAccount);
        }
        if self.amount_minor <= 0 {
            return Err(TransactionValidationError::InvalidAmount);
        }
        if self.currency_code.len() != 3
            || !self.currency_code.chars().all(|c| c.is_ascii_uppercase())
        {
            return Err(TransactionValidationError::InvalidCurrency);
        }
        if DateTime::parse_from_rfc3339(&self.occurred_at).is_err() {
            return Err(TransactionValidationError::InvalidDate);
        }
        if !self.allocations.is_empty() {
            let mut sum = 0_i64;
            for a in &self.allocations {
                if a.category_id.trim().is_empty() || a.amount_minor <= 0 {
                    return Err(TransactionValidationError::InvalidAllocation);
                }
                sum = sum
                    .checked_add(a.amount_minor)
                    .ok_or(TransactionValidationError::InvalidAllocation)?;
            }
            if sum != self.amount_minor {
                return Err(TransactionValidationError::InvalidAllocation);
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> NewTransaction {
        NewTransaction {
            account_id: "a".into(),
            transaction_type: TransactionType::Expense,
            amount_minor: 100,
            currency_code: "KES".into(),
            occurred_at: "2026-11-06T10:00:00+03:00".into(),
            description: None,
            notes: None,
            allocations: vec![],
        }
    }
    #[test]
    fn accepts_uncategorized() {
        assert!(valid().validate().is_ok());
    }
    #[test]
    fn rejects_non_positive() {
        let mut t = valid();
        t.amount_minor = 0;
        assert_eq!(t.validate(), Err(TransactionValidationError::InvalidAmount));
    }
    #[test]
    fn allocations_reconcile() {
        let mut t = valid();
        t.allocations = vec![Allocation {
            category_id: "food".into(),
            amount_minor: 99,
        }];
        assert_eq!(
            t.validate(),
            Err(TransactionValidationError::InvalidAllocation)
        );
    }
    #[test]
    fn signs_entries() {
        assert_eq!(TransactionType::Expense.entry_amount(50), -50);
        assert_eq!(TransactionType::Income.entry_amount(50), 50);
    }
}
