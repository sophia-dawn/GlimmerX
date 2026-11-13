//! Decimal-safe money parsing.
//!
//! Amounts are stored as integer cents everywhere in the database. This module
//! deliberately never converts through `f64`: `f64 as i64` is a saturating cast
//! in Rust, so an input like `"1e309"` would silently become `i64::MAX` and pass
//! the double-entry balance check.

use crate::db::AppError;

/// Business upper bound for a single amount, in cents (100,000,000.00 CNY).
/// Keeps negation, sums and future arithmetic safely inside `i64`.
pub const MAX_AMOUNT_CENTS: i64 = 10_000_000_000;

fn invalid() -> AppError {
    AppError::ValidationError("errors.transaction.invalidAmount".to_string())
}

/// Parse a decimal amount string into integer cents without floating point.
///
/// Accepts an optional sign, digits and at most one decimal point. Fractional
/// digits beyond two are rounded half away from zero (matching the previous
/// `(v * 100.0).round()` behaviour, e.g. `"0.005"` -> 1). Scientific notation,
/// `inf`, `NaN`, separators and values above [`MAX_AMOUNT_CENTS`] are rejected.
pub fn parse_amount_to_cents(amount_str: &str) -> Result<i64, AppError> {
    let s = amount_str.trim();
    if s.is_empty() || s.len() > 24 {
        return Err(invalid());
    }

    let (negative, body) = match s.as_bytes()[0] {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    if body.is_empty() {
        return Err(invalid());
    }

    let mut parts = body.split('.');
    let integer = parts.next().unwrap_or("");
    let fraction = parts.next();
    if parts.next().is_some() {
        return Err(invalid());
    }
    // Reject "." / "-." but keep ".50" and "1." working like the old parser.
    if integer.is_empty() && fraction.is_none_or(str::is_empty) {
        return Err(invalid());
    }
    if !integer.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let fraction = fraction.unwrap_or("");
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }

    // i128 keeps both the parse and the *100 step away from i64 overflow.
    let yuan: i128 = if integer.is_empty() {
        0
    } else {
        integer.parse().map_err(|_| invalid())?
    };
    let mut cents = yuan.checked_mul(100).ok_or_else(invalid)?;

    let digit =
        |index: usize| -> u8 { fraction.as_bytes().get(index).copied().unwrap_or(b'0') - b'0' };
    cents = cents
        .checked_add(i128::from(digit(0)) * 10 + i128::from(digit(1)))
        .ok_or_else(invalid)?;
    if digit(2) >= 5 {
        cents = cents.checked_add(1).ok_or_else(invalid)?;
    }
    if negative {
        cents = -cents;
    }
    if cents.unsigned_abs() > MAX_AMOUNT_CENTS as u128 {
        return Err(invalid());
    }
    i64::try_from(cents).map_err(|_| invalid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_existing_behaviour_preserved() {
        assert_eq!(parse_amount_to_cents("12.50").unwrap(), 1250);
        assert_eq!(parse_amount_to_cents("100").unwrap(), 10000);
        assert_eq!(parse_amount_to_cents("-3.99").unwrap(), -399);
        assert_eq!(parse_amount_to_cents("0.005").unwrap(), 1);
        assert_eq!(parse_amount_to_cents(".50").unwrap(), 50);
        assert_eq!(parse_amount_to_cents("+1.25").unwrap(), 125);
        assert_eq!(parse_amount_to_cents(" 12.50 ").unwrap(), 1250);
        assert_eq!(parse_amount_to_cents("12.345").unwrap(), 1235);
    }

    #[test]
    fn test_non_finite_and_scientific_notation_rejected() {
        let too_long = "9".repeat(30);
        for value in [
            "inf",
            "-inf",
            "NaN",
            "1e309",
            "-1e309",
            "9e18",
            "1e3",
            "abc",
            "",
            "   ",
            "1,000",
            "1.2.3",
            "--1",
            "+-1",
            "1.2a",
            ".",
            "-",
            too_long.as_str(),
        ] {
            assert!(
                parse_amount_to_cents(value).is_err(),
                "expected {value:?} to be rejected"
            );
        }
    }

    #[test]
    fn test_upper_bound_enforced() {
        let max_yuan = MAX_AMOUNT_CENTS / 100;
        assert_eq!(
            parse_amount_to_cents(&format!("{max_yuan}.00")).unwrap(),
            MAX_AMOUNT_CENTS
        );
        assert!(parse_amount_to_cents(&format!("{}.00", max_yuan + 1)).is_err());
        assert!(parse_amount_to_cents(&format!("-{}.00", max_yuan + 1)).is_err());
        assert!(parse_amount_to_cents("9223372036854775807").is_err());
    }
}
