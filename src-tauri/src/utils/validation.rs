//! Input validation utilities for Tauri commands.
//!
//! Provides helper functions for validating common input types:
//! - Non-empty names (accounts, categories, etc.)
//! - Date format (YYYY-MM-DD)
//! - Pagination bounds (page and page_size)
//! - Amount values (positive, within limits)

use crate::db::AppError;

#[allow(dead_code)]
pub fn validate_non_empty_name(name: &str, field_name: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError(format!(
            "{} cannot be empty",
            field_name
        )));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn validate_date_format(date: &str) -> Result<(), AppError> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err(AppError::ValidationError(
            "errors.invalidDateFormat".to_string(),
        ));
    }

    if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(AppError::ValidationError(
            "errors.invalidDateFormat".to_string(),
        ));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| AppError::ValidationError("errors.invalidDateFormat".to_string()))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| AppError::ValidationError("errors.invalidDateFormat".to_string()))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| AppError::ValidationError("errors.invalidDateFormat".to_string()))?;

    if !(1900..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(AppError::ValidationError(
            "errors.invalidDateFormat".to_string(),
        ));
    }

    Ok(())
}

#[allow(dead_code)]
pub fn validate_pagination(page: Option<u32>, page_size: Option<u32>) -> (u32, u32) {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    (page, page_size)
}

#[allow(dead_code)]
pub fn validate_amount_positive(amount: i64, field_name: &str) -> Result<(), AppError> {
    if amount <= 0 {
        return Err(AppError::ValidationError(format!(
            "{} must be positive",
            field_name
        )));
    }
    if amount > 10_000_000_000 {
        return Err(AppError::ValidationError(format!(
            "{} exceeds maximum",
            field_name
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_non_empty_name_valid() {
        assert!(validate_non_empty_name("Bank Account", "Account name").is_ok());
        assert!(validate_non_empty_name("  Trimmed Name  ", "Account name").is_ok());
    }

    #[test]
    fn test_validate_non_empty_name_empty() {
        let result = validate_non_empty_name("", "Account name");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ValidationError(msg) => assert!(msg.contains("cannot be empty")),
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_validate_non_empty_name_whitespace_only() {
        let result = validate_non_empty_name("   ", "Account name");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_date_format_valid() {
        assert!(validate_date_format("2024-01-15").is_ok());
        assert!(validate_date_format("2024-12-31").is_ok());
        assert!(validate_date_format("1900-01-01").is_ok());
        assert!(validate_date_format("2100-12-31").is_ok());
    }

    #[test]
    fn test_validate_date_format_invalid_format() {
        assert!(validate_date_format("2024/01/15").is_err()); // Wrong separator
        assert!(validate_date_format("2024-1-15").is_err()); // Month not two digits
        assert!(validate_date_format("24-01-15").is_err()); // Year not four digits
        assert!(validate_date_format("2024-01").is_err()); // Missing day
        assert!(validate_date_format("invalid").is_err()); // Not a date
    }

    #[test]
    fn test_validate_date_format_out_of_range() {
        assert!(validate_date_format("1899-01-01").is_err()); // Year < 1900
        assert!(validate_date_format("2101-01-01").is_err()); // Year > 2100
        assert!(validate_date_format("2024-00-01").is_err()); // Month 0
        assert!(validate_date_format("2024-13-01").is_err()); // Month 13
        assert!(validate_date_format("2024-01-00").is_err()); // Day 0
        assert!(validate_date_format("2024-01-32").is_err()); // Day 32
    }

    #[test]
    fn test_validate_pagination_defaults() {
        let (page, page_size) = validate_pagination(None, None);
        assert_eq!(page, 1);
        assert_eq!(page_size, 20);
    }

    #[test]
    fn test_validate_pagination_page_zero_or_negative() {
        // Note: u32 can't be negative, but 0 is handled
        let (page, _) = validate_pagination(Some(0), None);
        assert_eq!(page, 1);
    }

    #[test]
    fn test_validate_pagination_page_size_bounds() {
        let (_, page_size) = validate_pagination(None, Some(0));
        assert_eq!(page_size, 1); // Minimum 1

        let (_, page_size) = validate_pagination(None, Some(150));
        assert_eq!(page_size, 100); // Maximum 100

        let (_, page_size) = validate_pagination(None, Some(50));
        assert_eq!(page_size, 50); // Within bounds
    }

    #[test]
    fn test_validate_amount_positive_valid() {
        assert!(validate_amount_positive(1, "Amount").is_ok()); // 1 cent
        assert!(validate_amount_positive(10_000, "Amount").is_ok()); // 100 yuan
        assert!(validate_amount_positive(10_000_000_000 - 1, "Amount").is_ok());
        // Just under max
    }

    #[test]
    fn test_validate_amount_positive_zero() {
        let result = validate_amount_positive(0, "Amount");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ValidationError(msg) => assert!(msg.contains("must be positive")),
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_validate_amount_positive_negative() {
        let result = validate_amount_positive(-100, "Amount");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_amount_positive_exceeds_max() {
        let result = validate_amount_positive(10_000_000_000 + 1, "Amount");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::ValidationError(msg) => assert!(msg.contains("exceeds maximum")),
            _ => panic!("Expected ValidationError"),
        }
    }
}
