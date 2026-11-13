//! Account creation, including hierarchical paths and batch creation.

use rusqlite::Connection;

use crate::constants::account_type;
use crate::db::AppError;

use super::crud::{find_account, find_account_by_type_and_name, insert_account};
use super::opening::create_opening_balance;
use super::path::{parse_path, type_from_path};
use super::types::{AccountRecord, BatchAccountInput, CreateAccountFullInput};

/// Create an account from a path like "Assets/招商银行".
/// First segment determines type. Remaining segments joined by "/" become the name.
/// Returns the created account's ID.
///
/// No currency parameter: every account is CNY (see `crud::insert_account`).
pub fn create_account_with_path(
    conn: &Connection,
    full_path: &str,
    input: Option<&CreateAccountFullInput>,
) -> Result<String, AppError> {
    let segments = parse_path(full_path);
    if segments.is_empty() {
        return Err(AppError::ValidationError(
            "Account path must contain at least one segment".into(),
        ));
    }

    let account_type = type_from_path(&segments)?;
    // Name is all segments after the first, joined by "/"
    let name = if segments.len() > 1 {
        segments[1..].join("/")
    } else {
        segments[0].to_string()
    };

    // Check if account with same type and name already exists
    if let Some(_existing) = find_account_by_type_and_name(conn, &account_type, &name)? {
        return Err(AppError::ValidationError(
            "errors.accountNameAlreadyExists".to_string(),
        ));
    }

    let full_input = input.cloned().unwrap_or(CreateAccountFullInput {
        is_active: true,
        // Same default as `account_create` (`unwrap_or(true)`): an asset or
        // liability account counts toward net worth unless the user says
        // otherwise. Defaulting to false here made the checkbox silently
        // disagree between the command path and internal callers.
        include_net_worth: true,
        description: None,
        account_number: None,
        iban: None,
    });
    // Income/Expense accounts are P&L accounts - never include in net worth
    // Asset/Liability can be controlled by user (e.g., some liability accounts may be excluded)
    let include_net_worth = if account_type::should_include_in_net_worth(&account_type) {
        full_input.include_net_worth
    } else {
        false
    };
    let acct = AccountRecord {
        id: String::new(),
        name,
        account_type,
        description: full_input.description.unwrap_or_default(),
        account_number: full_input.account_number,
        is_system: false,
        iban: full_input.iban,
        is_active: full_input.is_active,
        include_net_worth,
        created_at: String::new(),
        updated_at: String::new(),
    };
    insert_account(conn, &acct)
}

// ---------------------------------------------------------------------------
// Batch account creation
// ---------------------------------------------------------------------------

/// Create multiple accounts in a single call.
/// Returns the created account records.
pub fn batch_create_accounts(
    conn: &Connection,
    inputs: Vec<BatchAccountInput>,
) -> Result<Vec<AccountRecord>, AppError> {
    if inputs.is_empty() {
        return Ok(vec![]);
    }

    let mut created_accounts = Vec::new();
    for input in &inputs {
        let segments = parse_path(&input.name);
        let account_type = if segments.is_empty() {
            "asset".to_string()
        } else {
            type_from_path(&segments)?
        };
        let name = if segments.len() > 1 {
            segments[1..].join("/")
        } else {
            input.name.clone()
        };

        if let Some(_existing) = find_account_by_type_and_name(conn, &account_type, &name)? {
            return Err(AppError::ValidationError(
                "errors.accountNameAlreadyExists".to_string(),
            ));
        }

        let include_net_worth = if account_type::should_include_in_net_worth(&account_type) {
            true // Batch creation doesn't have user input, use default true for balance sheet accounts
        } else {
            false // P&L accounts never include in net worth
        };
        let acct = AccountRecord {
            id: String::new(),
            name,
            account_type,
            description: input.description.clone().unwrap_or_default(),
            account_number: input.account_number.clone(),
            is_system: false,
            iban: None,
            is_active: true,
            include_net_worth,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let new_id = insert_account(conn, &acct)?;

        if let Some(balance) = input.initial_balance {
            if balance != 0 {
                create_opening_balance(conn, &new_id, balance, "Equity", "Opening Balances", None)?;
            }
        }

        if let Some(record) = find_account(conn, &new_id)? {
            created_accounts.push(record);
        }
    }

    Ok(created_accounts)
}
