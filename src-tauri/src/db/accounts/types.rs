//! Row types and command-facing input structs for accounts.

/// Internal DB row representation for an account.
///
/// No `currency` field: the ledger is CNY-only, and the DB column is frozen
/// legacy that `insert_account` writes as a literal.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountRecord {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub description: String,
    pub account_number: Option<String>,
    pub is_system: bool,
    pub iban: Option<String>,
    pub is_active: bool,
    pub include_net_worth: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Transaction record for account transaction history.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionRecord {
    pub id: String,
    pub date: String,
    pub description: String,
    pub amount: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Metadata key-value record for accounts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountMetaRecord {
    pub id: String,
    pub account_id: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
}

/// Parameters for updating an account.
pub struct UpdateAccountParams<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub account_number: Option<Option<&'a str>>,
    pub iban: Option<Option<&'a str>>,
    pub is_active: Option<bool>,
    pub include_net_worth: Option<bool>,
}

/// Full input for creating an account with all fields.
#[derive(Default, Clone)]
pub struct CreateAccountFullInput {
    pub description: Option<String>,
    pub account_number: Option<String>,
    pub iban: Option<String>,
    pub is_active: bool,
    pub include_net_worth: bool,
}

/// Name for a single account in a batch creation request.
pub struct BatchAccountInput {
    pub name: String,
    pub initial_balance: Option<i64>,
    pub description: Option<String>,
    pub account_number: Option<String>,
}
