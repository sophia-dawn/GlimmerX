//! Account reference resolution (design doc 25, §10.3).
//!
//! Replaces the old `commands::ai::resolve_source_account` behaviour where a
//! failed match silently fell back to a default account — the single biggest
//! diagnosability defect of the old AI path. Here a failed resolution is an
//! error, and every fuzzy hit is surfaced as a warning.

use crate::db::accounts::AccountRecord;
use crate::db::AppError;

use super::types::AccountRef;

/// Resolve [`AccountRef`] to an account id.
///
/// Name rules, in order:
/// 1. Exact match (case-insensitive) → hit, no warning.
/// 2. Unique contains match (either direction) → hit plus a warning naming
///    the account the hint landed on.
/// 3. Several contains matches → error (the caller must supply an id).
/// 4. No match at all → error.
///
/// Inactive accounts never match; an `Id` reference to an inactive account is
/// an error too (posting to it would fail validation anyway).
///
/// Returns `(account_id, warning)`.
pub fn resolve_account(
    accounts: &[AccountRecord],
    reference: &AccountRef,
) -> Result<(String, Option<String>), AppError> {
    match reference {
        AccountRef::Id(id) => match accounts.iter().find(|a| &a.id == id) {
            Some(acc) if acc.is_active => Ok((acc.id.clone(), None)),
            Some(_) => Err(AppError::ValidationError(
                "errors.account.inactive".to_string(),
            )),
            None => Err(AppError::NotFound("errors.accountNotFound".to_string())),
        },

        AccountRef::Name(raw) => {
            let name = raw.trim();
            if name.is_empty() {
                return Err(AppError::NotFound(
                    "errors.ledger.accountNameNotFound".to_string(),
                ));
            }
            let wanted = name.to_lowercase();

            // Rule 1: exact (case-insensitive).
            if let Some(acc) = accounts
                .iter()
                .filter(|a| a.is_active)
                .find(|a| a.name.to_lowercase() == wanted)
            {
                return Ok((acc.id.clone(), None));
            }

            // Rule 2: contains, either direction, longest name wins when the
            // hint contains several account names.
            let mut contains: Vec<&AccountRecord> = accounts
                .iter()
                .filter(|a| {
                    a.is_active
                        && (a.name.to_lowercase().contains(&wanted)
                            || wanted.contains(&a.name.to_lowercase()))
                })
                .collect();

            match contains.len() {
                1 => {
                    let acc = contains.remove(0);
                    Ok((
                        acc.id.clone(),
                        Some(format!("账户「{name}」按名称匹配到「{}」", acc.name)),
                    ))
                }
                0 => Err(AppError::NotFound(
                    "errors.ledger.accountNameNotFound".to_string(),
                )),
                // Rule 3: ambiguous — the caller must disambiguate with an id.
                _ => Err(AppError::ValidationError(
                    "errors.ledger.accountNameAmbiguous".to_string(),
                )),
            }
        }
    }
}
