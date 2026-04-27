//! Time utilities for consistent local timezone handling.
//!
//! # Time Handling Convention
//!
//! This application uses **local timezone** for all date/time operations.
//!
//! ## Why Local Timezone?
//!
//! GlimmerX is a desktop application where users typically run on a single
//! machine. Unlike web applications that serve users across timezones,
//! a desktop app's timezone is consistent for the user's session.
//!
//! ## Database Storage Format
//!
//! | Field Type | Format | Example |
//! |------------|--------|---------|
//! | Timestamps (created_at, updated_at) | RFC3339 with offset | `2024-04-21T14:30:00+08:00` |
//! | Business dates (transactions.date) | YYYY-MM-DD | `2024-04-21` |
//!
//! ## Migration Notes
//!
//! Existing databases store timestamps in UTC format (e.g., `2024-04-21T06:30:00+00:00`).
//! After this change, new timestamps will use local timezone. Existing data is
//! unaffected - SQLite stores TEXT and comparison works correctly for both formats.
//!
//! ## Important
//!
//! Do NOT use `chrono::Utc::now()` anywhere in this codebase.
//! Always use functions from this module for consistency.

use chrono::{Datelike, Local, NaiveDate};

pub fn now_rfc3339() -> String {
    Local::now().to_rfc3339()
}

pub fn today_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn current_year() -> i32 {
    Local::now().year()
}

pub fn current_month() -> u32 {
    Local::now().month()
}

pub fn month_start(year: i32, month: u32) -> String {
    format!("{}-{:02}-01", year, month)
}

pub fn month_end(year: i32, month: u32) -> String {
    let end_date = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .and_then(|d| d.pred_opt())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, 12, 31).unwrap())
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .and_then(|d| d.pred_opt())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, month, 28).unwrap())
    };
    end_date.format("%Y-%m-%d").to_string()
}

pub fn current_month_bounds() -> (String, String) {
    let year = current_year();
    let month = current_month();
    (month_start(year, month), month_end(year, month))
}

pub fn year_bounds(year: i32) -> (String, String) {
    (format!("{}-01-01", year), format!("{}-12-31", year))
}

pub fn last_month_bounds() -> (String, String) {
    let year = current_year();
    let month = current_month();
    if month == 1 {
        (month_start(year - 1, 12), month_end(year - 1, 12))
    } else {
        (month_start(year, month - 1), month_end(year, month - 1))
    }
}

pub fn last_n_months_bounds(n: u32) -> (String, String) {
    let year = current_year();
    let month = current_month();
    let months_back = n as i32;
    let target_month = month as i32 - months_back;
    if target_month <= 0 {
        let adjusted_year = year - ((-target_month) / 12 + 1);
        let adjusted_month = 12 + (target_month % 12);
        if adjusted_month <= 0 {
            (
                month_start(adjusted_year - 1, (12 + adjusted_month) as u32),
                today_date(),
            )
        } else {
            (
                month_start(adjusted_year, adjusted_month as u32),
                today_date(),
            )
        }
    } else {
        (month_start(year, target_month as u32), today_date())
    }
}

pub fn last_year_bounds() -> (String, String) {
    year_bounds(current_year() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_end_february() {
        // Non-leap year
        let end = month_end(2023, 2);
        assert_eq!(end, "2023-02-28");

        // Leap year
        let end = month_end(2024, 2);
        assert_eq!(end, "2024-02-29");
    }

    #[test]
    fn test_month_end_december() {
        let end = month_end(2024, 12);
        assert_eq!(end, "2024-12-31");
    }

    #[test]
    fn test_month_start() {
        let start = month_start(2024, 3);
        assert_eq!(start, "2024-03-01");
    }

    #[test]
    fn test_year_bounds() {
        let (start, end) = year_bounds(2024);
        assert_eq!(start, "2024-01-01");
        assert_eq!(end, "2024-12-31");
    }

    #[test]
    fn test_now_rfc3339_format() {
        let ts = now_rfc3339();
        // Should contain timezone offset like +08:00 or Z
        assert!(ts.contains('+') || ts.ends_with('Z'));
    }

    #[test]
    fn test_today_date_format() {
        let date = today_date();
        // Should be YYYY-MM-DD format
        assert!(date.len() == 10);
        assert!(date.chars().nth(4) == Some('-'));
        assert!(date.chars().nth(7) == Some('-'));
    }

    #[test]
    fn test_last_month_bounds() {
        let (start, end) = last_month_bounds();
        assert!(start.len() == 10);
        assert!(end.len() == 10);
    }

    #[test]
    fn test_last_n_months_bounds() {
        let (start, end) = last_n_months_bounds(3);
        assert!(start.len() == 10);
        assert!(end.len() == 10);
    }

    #[test]
    fn test_last_year_bounds() {
        let (start, end) = last_year_bounds();
        assert!(start.ends_with("-01-01"));
        assert!(end.ends_with("-12-31"));
    }
}
