use chrono::{DateTime, SecondsFormat, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountType {
    MobileMoney,
    Bank,
    Cash,
    Other,
}

impl AccountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MobileMoney => "mobile_money",
            Self::Bank => "bank",
            Self::Cash => "cash",
            Self::Other => "other",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "mobile_money" => Some(Self::MobileMoney),
            "bank" => Some(Self::Bank),
            "cash" => Some(Self::Cash),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: AccountType,
    pub institution_name: Option<String>,
    pub currency_code: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}
#[derive(Debug, Clone)]
pub struct OpeningBalance {
    pub amount_minor: i64,
    pub occurred_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountValidationError {
    EmptyName,
    UnsupportedCurrency,
    InvalidOpeningBalanceAmount,
    MissingOpeningBalanceDate,
    InvalidOpeningBalanceDate,
}
#[derive(Debug, Clone)]
pub struct NewAccount {
    pub name: String,
    pub account_type: AccountType,
    pub institution_name: Option<String>,
    pub currency_code: String,
    pub notes: Option<String>,
    pub opening_balance: Option<OpeningBalance>,
}
#[derive(Debug, Clone)]
pub struct UpdateAccount {
    pub name: String,
    pub institution_name: Option<String>,
    pub notes: Option<String>,
}

impl NewAccount {
    pub fn new(
        name: String,
        account_type: AccountType,
        institution_name: Option<String>,
        notes: Option<String>,
    ) -> Result<Self, AccountValidationError> {
        let name = name.trim().to_string();

        if name.is_empty() {
            return Err(AccountValidationError::EmptyName);
        }

        Ok(Self {
            name,
            account_type,
            institution_name: Self::normalize_optional(institution_name),
            currency_code: "KES".to_string(),
            notes: Self::normalize_optional(notes),
            opening_balance: None,
        })
    }

    pub(crate) fn normalize_optional(value: Option<String>) -> Option<String> {
        value.and_then(|value| {
            let trimmed = value.trim().to_string();

            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    pub fn validate_currency(currency_code: &str) -> Result<(), AccountValidationError> {
        if currency_code != "KES" {
            return Err(AccountValidationError::UnsupportedCurrency);
        }

        Ok(())
    }
    pub fn with_opening_balance(mut self, opening_balance: OpeningBalance) -> Self {
        self.opening_balance = Some(opening_balance);
        self
    }
}

impl UpdateAccount {
    pub fn new(
        name: String,
        institution_name: Option<String>,
        notes: Option<String>,
    ) -> Result<Self, AccountValidationError> {
        let name = name.trim().to_string();

        if name.is_empty() {
            return Err(AccountValidationError::EmptyName);
        }

        Ok(Self {
            name,
            institution_name: NewAccount::normalize_optional(institution_name),
            notes: NewAccount::normalize_optional(notes),
        })
    }
}

impl Account {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}
impl OpeningBalance {
    pub fn new(amount_minor: i64, occurred_at: String) -> Result<Self, AccountValidationError> {
        if amount_minor <= 0 {
            return Err(AccountValidationError::InvalidOpeningBalanceAmount);
        }

        let occurred_at = occurred_at.trim();

        if occurred_at.is_empty() {
            return Err(AccountValidationError::MissingOpeningBalanceDate);
        }

        let parsed = DateTime::parse_from_rfc3339(occurred_at)
            .map_err(|_| AccountValidationError::InvalidOpeningBalanceDate)?;
        let occurred_at = parsed
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Millis, true);

        Ok(Self {
            amount_minor,
            occurred_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_opening_balance_timestamp_to_utc() {
        let balance =
            OpeningBalance::new(150_000, "2026-11-05T10:00:00+03:00".to_string()).unwrap();

        assert_eq!(balance.amount_minor, 150_000);
        assert_eq!(balance.occurred_at, "2026-11-05T07:00:00.000Z");
    }

    #[test]
    fn rejects_empty_opening_balance_date() {
        let result = OpeningBalance::new(150_000, " ".to_string());

        assert_eq!(
            result.unwrap_err(),
            AccountValidationError::MissingOpeningBalanceDate
        );
    }

    #[test]
    fn rejects_invalid_opening_balance_date() {
        let result = OpeningBalance::new(150_000, "not-a-date".to_string());

        assert_eq!(
            result.unwrap_err(),
            AccountValidationError::InvalidOpeningBalanceDate
        );
    }

    #[test]
    fn rejects_date_without_timezone() {
        let result = OpeningBalance::new(150_000, "2026-11-05T10:00:00".to_string());

        assert_eq!(
            result.unwrap_err(),
            AccountValidationError::InvalidOpeningBalanceDate
        );
    }
}
