//! Parsing of hierarchical account paths such as `Assets/Cash/Wallet`.

use crate::db::AppError;

const VALID_ROOT_TYPES: &[&str] = &["asset", "liability", "income", "expense", "equity"];

/// Map common plural forms to their singular type names.
fn normalize_root_type(segment: &str) -> Option<String> {
    let lower = segment.to_lowercase();
    if VALID_ROOT_TYPES.contains(&lower.as_str()) {
        return Some(lower);
    }
    let mapping = [
        ("assets", "asset"),
        ("liabilities", "liability"),
        ("incomes", "income"),
        ("expenses", "expense"),
        ("equities", "equity"),
    ];
    for (from, to) in mapping {
        if lower == from {
            return Some(to.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Path parsing helpers
// ---------------------------------------------------------------------------

/// Split a full path like "Assets/Bank/招商银行" into segments.
pub(in crate::db::accounts) fn parse_path(full_path: &str) -> Vec<&str> {
    full_path
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Determine the account type from the first segment of the path.
pub(in crate::db::accounts) fn type_from_path(segments: &[&str]) -> Result<String, AppError> {
    let first = segments
        .first()
        .ok_or_else(|| AppError::ValidationError("Account path must not be empty".into()))?;
    normalize_root_type(first).ok_or_else(|| {
        AppError::ValidationError(format!(
            "Invalid account root type '{}'. Valid types: {:?}",
            first, VALID_ROOT_TYPES
        ))
    })
}
