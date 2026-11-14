//! Write capabilities (design doc 25, §3.2–§3.4).
//!
//! Two functions, both returning rich outcomes. The transaction boundary
//! belongs to the quick-add layer — this module never opens its own
//! transaction around posting writes, only around account creation +
//! opening balance (which the db layer leaves to its caller).

use rusqlite::Connection;

use crate::db::transactions::{QuickAddInput, TransactionRecord, TransactionWithPostings};
use crate::db::{accounts, categories, transactions, AppError};
use crate::utils::money::MAX_AMOUNT_CENTS;

use super::read::validate_date;
use super::resolve::resolve_account;
use super::types::{
    AccountOutcome, AccountRef, CategoryRef, CreateAccountRequest, LedgerOutcome, PostingView,
    RecordKind, RecordRequest,
};

/// Integer cents to the plain decimal string the quick-add layer expects.
/// Pure integer arithmetic — no float, same discipline as
/// `utils::money::parse_amount_to_cents`.
fn cents_to_decimal(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{}{}.{:02}", sign, abs / 100, abs % 100)
}

/// Enrich raw postings with account identity for preview surfaces.
///
/// `all_accounts` is a caller-provided snapshot (usually `list_accounts`);
/// ids missing from it — the equity account of an opening balance, system
/// rows — fall back to `find_account`, which has no `is_system` filter.
fn posting_views(
    conn: &Connection,
    all_accounts: &[accounts::AccountRecord],
    postings: &[(String, i64)],
) -> Result<Vec<PostingView>, AppError> {
    let mut views = Vec::with_capacity(postings.len());
    for (account_id, amount) in postings {
        let acc = match all_accounts.iter().find(|a| &a.id == account_id) {
            Some(acc) => acc.clone(),
            None => accounts::find_account(conn, account_id)?
                .ok_or_else(|| AppError::NotFound("errors.accountNotFound".to_string()))?,
        };
        views.push(PostingView {
            account_id: account_id.clone(),
            account_name: acc.name,
            account_type: acc.account_type,
            amount: *amount,
        });
    }
    Ok(views)
}

/// Record an expense, income or transfer.
///
/// Account names are resolved by the four rules in [`super::resolve`] (never
/// a silent default fallback); category names are resolved-or-created inside
/// the quick-add transaction (P0-3). Every guess lands in `warnings`.
pub fn record_transaction(
    conn: &Connection,
    req: &RecordRequest,
) -> Result<LedgerOutcome, AppError> {
    if req.amount_cents <= 0 {
        return Err(AppError::ValidationError(
            "errors.transaction.amountMustBePositive".to_string(),
        ));
    }
    if req.amount_cents > MAX_AMOUNT_CENTS {
        return Err(AppError::ValidationError(
            "errors.transaction.invalidAmount".to_string(),
        ));
    }
    if let Some(ref date) = req.date {
        validate_date(date)?;
    }

    // Resolve account references against a single snapshot of the accounts.
    let all_accounts = accounts::list_accounts(conn)?;
    let mut warnings: Vec<String> = Vec::new();

    let mut resolve = |r: &Option<AccountRef>| -> Result<Option<String>, AppError> {
        match r {
            None => Ok(None),
            Some(reference) => {
                let (id, warning) = resolve_account(&all_accounts, reference)?;
                if let Some(w) = warning {
                    warnings.push(w);
                }
                Ok(Some(id))
            }
        }
    };

    let source_account_id = resolve(&req.source_account)?;
    let destination_account_id = resolve(&req.destination_account)?;

    // Kind-specific required fields, checked before any write.
    match req.kind {
        RecordKind::Expense if source_account_id.is_none() => {
            return Err(AppError::ValidationError(
                "errors.transaction.expenseRequiresSource".to_string(),
            ));
        }
        RecordKind::Income if destination_account_id.is_none() => {
            return Err(AppError::ValidationError(
                "errors.transaction.incomeRequiresDestination".to_string(),
            ));
        }
        RecordKind::Transfer if source_account_id.is_none() || destination_account_id.is_none() => {
            return Err(AppError::ValidationError(
                "errors.transaction.transferRequiresBothAccounts".to_string(),
            ));
        }
        _ => {}
    }

    // Category: by id (verified, type checked) or by name (resolved inside
    // the quick-add transaction; warn when it will be newly created).
    let mut category_id = None;
    let mut category_name = None;
    if let Some(ref category) = req.category {
        let expected_type = req.kind.category_type().ok_or_else(|| {
            AppError::ValidationError("errors.transaction.transferHasNoCategory".to_string())
        })?;
        match category {
            CategoryRef::Id(id) => {
                let record = categories::find_category(conn, id)?
                    .ok_or_else(|| AppError::NotFound("errors.categoryNotFound".to_string()))?;
                if record.category_type != expected_type {
                    return Err(AppError::ValidationError(
                        "errors.ledger.categoryTypeMismatch".to_string(),
                    ));
                }
                category_id = Some(id.clone());
            }
            CategoryRef::Name(name) => {
                if categories::find_by_type_and_name(conn, &expected_type, name)?.is_none() {
                    warnings.push(format!("分类「{name}」是新建的"));
                }
                category_name = Some(name.clone());
            }
        }
    }

    let input = QuickAddInput {
        mode: req.kind.as_str().to_string(),
        amount: cents_to_decimal(req.amount_cents),
        source_account_id,
        destination_account_id,
        category_id,
        category_name,
        description: Some(req.description.clone()),
        date: req.date.clone(),
    };

    let written = transactions::quick_add_transaction(conn, &input)?;

    to_outcome(conn, &all_accounts, &written, warnings)
}

/// Create an account, optionally with an opening balance.
///
/// Account creation and the opening-balance entry share one transaction, so
/// a failed balance write cannot leave an empty account behind.
pub fn create_account(
    conn: &Connection,
    req: &CreateAccountRequest,
) -> Result<AccountOutcome, AppError> {
    if let Some(amount) = req.opening_balance_cents {
        if amount.abs() > MAX_AMOUNT_CENTS {
            return Err(AppError::ValidationError(
                "errors.transaction.invalidAmount".to_string(),
            ));
        }
    }
    if let Some(ref date) = req.opening_balance_date {
        validate_date(date)?;
    }

    let tx = conn.unchecked_transaction()?;

    let full_input = accounts::CreateAccountFullInput {
        description: req.description.clone(),
        account_number: None,
        iban: None,
        is_active: true,
        include_net_worth: true,
    };
    let account_id = accounts::create_account_with_path(&tx, &req.path, Some(&full_input))?;

    // The opening balance writes through the equity entry (the only path
    // allowed to post on equity), same defaults as the account command.
    if let Some(amount) = req.opening_balance_cents {
        if amount != 0 {
            accounts::create_opening_balance(
                &tx,
                &account_id,
                amount,
                "Equity",
                "Opening Balances",
                req.opening_balance_date.as_deref(),
            )?;
        }
    }

    tx.commit()?;

    // Surface the opening entry as preview postings. The opening balance is
    // identified by its shape (a posting on an equity account), not by
    // description matching — same rule as the rest of the codebase.
    let all_accounts = accounts::list_accounts(conn)?;
    let opening = transactions::list_transactions(conn, Some(&account_id), None, None)?
        .into_iter()
        .find(|t| {
            t.postings.iter().any(|p| {
                // The equity side is not in `all_accounts` (system row), so
                // resolve the posting's account type individually.
                accounts::find_account(conn, &p.account_id)
                    .map(|acc| acc.is_some_and(|a| a.account_type == "equity"))
                    .unwrap_or(false)
            })
        });
    let opening_transaction_id = opening.as_ref().map(|t| t.transaction.id.clone());
    let opening_postings = match opening {
        Some(t) => posting_views(
            conn,
            &all_accounts,
            &t.postings
                .iter()
                .map(|p| (p.account_id.clone(), p.amount))
                .collect::<Vec<_>>(),
        )?,
        None => Vec::new(),
    };

    // Canonical path: root type + name as stored.
    let created = accounts::find_account(conn, &account_id)?
        .ok_or_else(|| AppError::NotFound("errors.accountNotFound".to_string()))?;
    let root = match created.account_type.as_str() {
        "asset" => "Assets",
        "liability" => "Liabilities",
        other => other,
    };
    let account_path = format!("{}/{}", root, created.name);

    Ok(AccountOutcome {
        account_id,
        account_path,
        account_type: created.account_type,
        opening_transaction_id,
        opening_postings,
        warnings: Vec::new(),
    })
}

fn to_outcome(
    conn: &Connection,
    all_accounts: &[accounts::AccountRecord],
    written: &TransactionWithPostings,
    warnings: Vec<String>,
) -> Result<LedgerOutcome, AppError> {
    let TransactionRecord {
        id,
        date,
        description,
        ..
    } = &written.transaction;
    let postings = posting_views(
        conn,
        all_accounts,
        &written
            .postings
            .iter()
            .map(|p| (p.account_id.clone(), p.amount))
            .collect::<Vec<_>>(),
    )?;
    Ok(LedgerOutcome {
        transaction_id: id.clone(),
        date: date.clone(),
        description: description.clone(),
        postings,
        warnings,
    })
}
