//! Savings goals.
//!
//! A goal is not a separate pot of money: it is a dedicated asset account with
//! a target attached. "Saving" is then an ordinary asset-to-asset transfer, so
//! progress is just the account balance and the double-entry ledger stays the
//! only source of truth. The alternative — a standalone allocation table — would
//! let the two disagree, which is the failure mode this design removes.

use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::AppError;

/// Account-name prefix for goal accounts, e.g. `储蓄目标/日本旅行`.
pub const ACCOUNT_PREFIX: &str = "储蓄目标";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavingsGoalRecord {
    pub id: String,
    pub name: String,
    /// The asset account holding the money.
    pub account_id: String,
    pub target_amount: i64,
    pub target_date: Option<String>,
    /// Optional manual plan in cents per month.
    pub monthly_plan: Option<i64>,
    /// `active` | `achieved` | `archived`
    pub status: String,
    /// Balance of the goal account (the progress).
    pub saved_amount: i64,
    /// `target_amount - saved_amount`, floored at zero.
    pub remaining: i64,
    /// Cents needed per month to hit `target_date`; falls back to
    /// `monthly_plan` when no date is set. Computed here so the UI never
    /// divides money itself.
    pub monthly_needed: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

fn now() -> String {
    crate::utils::time::now_rfc3339()
}

fn account_name(name: &str) -> String {
    format!("{ACCOUNT_PREFIX}/{name}")
}

/// Validate and normalise the target before anything is written.
fn validate_target(target_amount: i64) -> Result<(), AppError> {
    if target_amount <= 0 {
        return Err(AppError::ValidationError(
            "errors.savings.invalidTarget".to_string(),
        ));
    }
    Ok(())
}

fn find_goal(conn: &Connection, id: &str) -> Result<SavingsGoalRecord, AppError> {
    let goal = conn
        .query_row(
            "SELECT g.id, g.name, g.account_id, g.target_amount, g.target_date, g.monthly_plan,
                g.status, COALESCE((SELECT SUM(p.amount) FROM postings p
                                     WHERE p.account_id = g.account_id), 0),
                g.created_at, g.updated_at
           FROM savings_goals g
          WHERE g.id = ?1",
            [id],
            |row| {
                Ok(SavingsGoalRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    account_id: row.get(2)?,
                    target_amount: row.get(3)?,
                    target_date: row.get(4)?,
                    monthly_plan: row.get(5)?,
                    status: row.get(6)?,
                    saved_amount: row.get(7)?,
                    remaining: 0,
                    monthly_needed: None,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("errors.savings.goalNotFound".to_string()))?;

    Ok(with_derived(goal))
}

/// Fill the derived money fields.
fn with_derived(mut goal: SavingsGoalRecord) -> SavingsGoalRecord {
    goal.remaining = (goal.target_amount - goal.saved_amount).max(0);
    goal.monthly_needed = match goal.target_date.as_deref() {
        Some(date) => months_until(date).map(|months| {
            // Round up: half a month of saving is not a plan.
            if months <= 0 {
                goal.remaining
            } else {
                (goal.remaining + months - 1) / months
            }
        }),
        None => goal.monthly_plan,
    };
    goal
}

/// Whole months between today and `date`; `None` if the date already passed.
fn months_until(date: &str) -> Option<i64> {
    use chrono::{Datelike, Local, NaiveDate};
    let target = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let today = Local::now().date_naive();
    if target <= today {
        return Some(0);
    }
    let months = (i64::from(target.year()) - i64::from(today.year())) * 12
        + (i64::from(target.month()) - i64::from(today.month()));
    Some(months.max(1))
}

/// Sync the derived `achieved` status.
///
/// Done on read rather than by a background job: the state is a pure function
/// of the balance, and a stale `active` badge is the only visible symptom.
fn refresh_status(conn: &Connection, goal: &mut SavingsGoalRecord) -> Result<(), AppError> {
    if goal.status == "active" && goal.saved_amount >= goal.target_amount {
        let stamp = now();
        conn.execute(
            "UPDATE savings_goals SET status = 'achieved', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![&stamp, &goal.id],
        )?;
        goal.status = "achieved".to_string();
        goal.updated_at = stamp;
    }
    Ok(())
}

/// All goals, active first then by name.
pub fn list_goals(conn: &Connection) -> Result<Vec<SavingsGoalRecord>, AppError> {
    let ids: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT id FROM savings_goals
              ORDER BY CASE status WHEN 'active' THEN 0 WHEN 'achieved' THEN 1 ELSE 2 END, name",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut goals = Vec::with_capacity(ids.len());
    for id in ids {
        let mut goal = find_goal(conn, &id)?;
        refresh_status(conn, &mut goal)?;
        goals.push(goal);
    }
    Ok(goals)
}

/// Create a goal together with its asset account.
pub fn create_goal(
    conn: &Connection,
    name: &str,
    target_amount: i64,
    target_date: Option<&str>,
    monthly_plan: Option<i64>,
) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::ValidationError(
            "errors.categoryNameRequired".to_string(),
        ));
    }
    validate_target(target_amount)?;

    let account = account_name(name);
    let taken: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE type = 'asset' AND name = ?1)",
        [&account],
        |row| row.get(0),
    )?;
    if taken {
        return Err(AppError::ValidationError(
            "errors.savings.duplicateName".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    let stamp = now();

    let account_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO accounts
             (id, name, type, currency, description, is_system, is_active,
              include_net_worth, created_at, updated_at)
         VALUES (?1, ?2, 'asset', 'CNY', '', 0, 1, 1, ?3, ?3)",
        rusqlite::params![&account_id, &account, &stamp],
    )?;

    let id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO savings_goals
             (id, name, account_id, target_amount, target_date, monthly_plan, status,
              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?7)",
        rusqlite::params![
            &id,
            name,
            &account_id,
            target_amount,
            target_date,
            monthly_plan,
            &stamp
        ],
    )?;

    tx.commit()?;
    Ok(id)
}

/// Partial update; `None` means "leave as is".
pub fn update_goal(
    conn: &Connection,
    id: &str,
    name: Option<&str>,
    target_amount: Option<i64>,
    target_date: Option<Option<&str>>,
    monthly_plan: Option<Option<i64>>,
) -> Result<(), AppError> {
    let goal = find_goal(conn, id)?;
    if let Some(amount) = target_amount {
        validate_target(amount)?;
    }

    let tx = conn.unchecked_transaction()?;
    let stamp = now();

    if let Some(raw_name) = name {
        let new_name = raw_name.trim();
        if new_name.is_empty() {
            return Err(AppError::ValidationError(
                "errors.categoryNameRequired".to_string(),
            ));
        }
        let account = account_name(new_name);
        let taken: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE type = 'asset' AND name = ?1 AND id <> ?2)",
            rusqlite::params![&account, &goal.account_id],
            |row| row.get(0),
        )?;
        if taken {
            return Err(AppError::ValidationError(
                "errors.savings.duplicateName".to_string(),
            ));
        }
        // Keep the account name in step, otherwise a rename resurrects the
        // duplicate-name drift this design exists to avoid.
        tx.execute(
            "UPDATE accounts SET name = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![&account, &stamp, &goal.account_id],
        )?;
        tx.execute(
            "UPDATE savings_goals SET name = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![new_name, &stamp, id],
        )?;
    }

    if let Some(amount) = target_amount {
        tx.execute(
            "UPDATE savings_goals SET target_amount = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![amount, &stamp, id],
        )?;
    }
    if let Some(date) = target_date {
        tx.execute(
            "UPDATE savings_goals SET target_date = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![date, &stamp, id],
        )?;
    }
    if let Some(plan) = monthly_plan {
        tx.execute(
            "UPDATE savings_goals SET monthly_plan = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![plan, &stamp, id],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Archive (or restore) a goal. The account stays so the history keeps working.
pub fn set_status(conn: &Connection, id: &str, status: &str) -> Result<(), AppError> {
    if !matches!(status, "active" | "achieved" | "archived") {
        return Err(AppError::ValidationError(
            "errors.savings.goalNotFound".to_string(),
        ));
    }
    let goal = find_goal(conn, id)?;
    // Restoring an archived goal must not claim it is achieved again.
    let status = if status == "achieved" && goal.saved_amount < goal.target_amount {
        "active"
    } else {
        status
    };
    conn.execute(
        "UPDATE savings_goals SET status = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![status, now(), id],
    )?;
    Ok(())
}

/// Delete a goal and its account.
///
/// Two refusals, for different reasons:
/// * a non-zero balance would vanish from the ledger — move it out first;
/// * any posting history means the account cannot be dropped at all (the
///   postings reference it), so the honest advice is to archive instead.
pub fn delete_goal(conn: &Connection, id: &str) -> Result<(), AppError> {
    let goal = find_goal(conn, id)?;
    if goal.saved_amount != 0 {
        return Err(AppError::ValidationError(
            "errors.savings.accountNotEmpty".to_string(),
        ));
    }

    let postings: i64 = conn.query_row(
        "SELECT COUNT(*) FROM postings WHERE account_id = ?1",
        [&goal.account_id],
        |row| row.get(0),
    )?;
    if postings > 0 {
        return Err(AppError::ValidationError(
            "errors.savings.goalHasHistory".to_string(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM savings_goals WHERE id = ?1", [id])?;
    tx.execute("DELETE FROM accounts WHERE id = ?1", [&goal.account_id])?;
    tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        crate::db::init_schema(&conn).unwrap();
        (dir, conn)
    }

    fn cash_account(conn: &Connection) -> String {
        let id = Uuid::new_v4().to_string();
        let stamp = now();
        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system,
                                   is_active, include_net_worth, created_at, updated_at)
             VALUES (?1, '现金', 'asset', 'CNY', '', 0, 1, 1, ?2, ?2)",
            rusqlite::params![&id, &stamp],
        )
        .unwrap();
        id
    }

    /// A transfer into the goal account, i.e. what "put money aside" does.
    fn deposit(conn: &Connection, from: &str, to: &str, date: &str, amount: i64) {
        let stamp = now();
        let tx_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO transactions (id, date, description, created_at, updated_at)
             VALUES (?1, ?2, '存入', ?3, ?3)",
            rusqlite::params![&tx_id, date, &stamp],
        )
        .unwrap();
        for (account, value, seq) in [(to, amount, 0), (from, -amount, 1)] {
            conn.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![Uuid::new_v4().to_string(), &tx_id, account, value, seq, &stamp],
            )
            .unwrap();
        }
    }

    #[test]
    fn test_create_goal_creates_asset_account() {
        let (_dir, conn) = test_conn();
        let id = create_goal(&conn, "日本旅行", 1_000_000, Some("2025-06-01"), None).unwrap();

        let goals = list_goals(&conn).unwrap();
        assert_eq!(goals.len(), 1);
        assert_eq!(goals[0].id, id);
        assert_eq!(goals[0].saved_amount, 0);
        assert_eq!(goals[0].status, "active");

        let (name, kind, net_worth): (String, String, i32) = conn
            .query_row(
                "SELECT name, type, include_net_worth FROM accounts WHERE id = ?1",
                [&goals[0].account_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(name, "储蓄目标/日本旅行");
        assert_eq!(kind, "asset");
        assert_eq!(net_worth, 1, "goal money is part of net worth");
    }

    #[test]
    fn test_progress_follows_transfers_and_flips_to_achieved() {
        let (_dir, conn) = test_conn();
        let cash = cash_account(&conn);
        let id = create_goal(&conn, "旅行", 100_000, None, None).unwrap();
        let account = list_goals(&conn).unwrap()[0].account_id.clone();

        deposit(&conn, &cash, &account, "2024-01-10", 60_000);
        let goals = list_goals(&conn).unwrap();
        assert_eq!(goals[0].saved_amount, 60_000);
        assert_eq!(goals[0].status, "active");

        deposit(&conn, &cash, &account, "2024-02-10", 40_000);
        let goals = list_goals(&conn).unwrap();
        assert_eq!(goals[0].saved_amount, 100_000);
        assert_eq!(goals[0].status, "achieved");

        // Withdrawing does not silently un-achieve; the user archives or edits.
        deposit(&conn, &account, &cash, "2024-03-10", 10_000);
        let goals = list_goals(&conn).unwrap();
        assert_eq!(goals[0].saved_amount, 90_000);
        assert_eq!(goals[0].status, "achieved");

        let _ = id;
    }

    #[test]
    fn test_update_renames_account_and_fields() {
        let (_dir, conn) = test_conn();
        create_goal(&conn, "旅行", 100_000, None, None).unwrap();
        let goal = list_goals(&conn).unwrap().remove(0);

        update_goal(
            &conn,
            &goal.id,
            Some("日本"),
            Some(200_000),
            Some(Some("2025-01-01")),
            Some(Some(20_000)),
        )
        .unwrap();

        let updated = list_goals(&conn).unwrap().remove(0);
        assert_eq!(updated.name, "日本");
        assert_eq!(updated.target_amount, 200_000);
        assert_eq!(updated.target_date.as_deref(), Some("2025-01-01"));
        assert_eq!(updated.monthly_plan, Some(20_000));

        let name: String = conn
            .query_row(
                "SELECT name FROM accounts WHERE id = ?1",
                [&updated.account_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "储蓄目标/日本");
    }

    #[test]
    fn test_create_rejects_duplicates_and_bad_targets() {
        let (_dir, conn) = test_conn();
        create_goal(&conn, "旅行", 100_000, None, None).unwrap();

        let dup = create_goal(&conn, "旅行", 100_000, None, None).unwrap_err();
        assert!(format!("{dup}").contains("同名"));

        let bad = create_goal(&conn, "车", 0, None, None).unwrap_err();
        assert!(format!("{bad}").contains("大于零"));

        let empty = create_goal(&conn, "  ", 1000, None, None).unwrap_err();
        assert!(format!("{empty}").contains("名称"));
    }

    #[test]
    fn test_delete_requires_empty_account() {
        let (_dir, conn) = test_conn();
        let cash = cash_account(&conn);
        create_goal(&conn, "旅行", 100_000, None, None).unwrap();
        let goal = list_goals(&conn).unwrap().remove(0);
        deposit(&conn, &cash, &goal.account_id, "2024-01-10", 5_000);

        let err = delete_goal(&conn, &goal.id).unwrap_err();
        assert!(format!("{err}").contains("余额"));

        // Moving the money out is not enough: the postings still reference the
        // account, so the honest fix is archiving, not deleting.
        deposit(&conn, &goal.account_id, &cash, "2024-02-10", 5_000);
        let err = delete_goal(&conn, &goal.id).unwrap_err();
        assert!(format!("{err}").contains("归档"));
    }

    #[test]
    fn test_delete_never_used_goal_removes_account() {
        let (_dir, conn) = test_conn();
        create_goal(&conn, "旅行", 100_000, None, None).unwrap();
        let goal = list_goals(&conn).unwrap().remove(0);

        delete_goal(&conn, &goal.id).unwrap();
        assert!(list_goals(&conn).unwrap().is_empty());
        let accounts: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE name LIKE ?1",
                [format!("{ACCOUNT_PREFIX}/%")],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(accounts, 0);
    }

    #[test]
    fn test_archive_and_restore() {
        let (_dir, conn) = test_conn();
        create_goal(&conn, "旅行", 100_000, None, None).unwrap();
        let goal = list_goals(&conn).unwrap().remove(0);

        set_status(&conn, &goal.id, "archived").unwrap();
        assert_eq!(list_goals(&conn).unwrap()[0].status, "archived");

        // Asking for `achieved` without the money falls back to `active`.
        set_status(&conn, &goal.id, "achieved").unwrap();
        assert_eq!(list_goals(&conn).unwrap()[0].status, "active");

        assert!(set_status(&conn, &goal.id, "bogus").is_err());
        assert!(set_status(&conn, "missing", "active").is_err());
    }

    #[test]
    fn test_missing_goal_errors() {
        let (_dir, conn) = test_conn();
        assert!(delete_goal(&conn, "missing").is_err());
        assert!(update_goal(&conn, "missing", Some("x"), None, None, None).is_err());
    }

    #[test]
    fn test_months_until() {
        use chrono::{Duration, Local};
        let future = (Local::now().date_naive() + Duration::days(60))
            .format("%Y-%m-%d")
            .to_string();
        let months = months_until(&future).unwrap();
        assert!((1..=3).contains(&months), "unexpected months: {months}");

        assert_eq!(
            months_until("2000-01-01"),
            Some(0),
            "past dates hit the floor"
        );
        assert_eq!(months_until("not-a-date"), None);
    }

    #[test]
    fn test_derived_remaining_and_monthly_needed() {
        use chrono::{Duration, Local};
        let (_dir, conn) = test_conn();
        let cash = cash_account(&conn);
        let date = (Local::now().date_naive() + Duration::days(60))
            .format("%Y-%m-%d")
            .to_string();

        create_goal(&conn, "旅行", 100_000, Some(&date), None).unwrap();
        let goal = list_goals(&conn).unwrap().remove(0);
        let months = months_until(&date).unwrap();

        assert_eq!(goal.remaining, 100_000);
        assert_eq!(
            goal.monthly_needed,
            Some((100_000 + months - 1) / months),
            "rounds up so the plan actually reaches the target"
        );

        deposit(&conn, &cash, &goal.account_id, "2024-01-10", 40_000);
        let goal = list_goals(&conn).unwrap().remove(0);
        assert_eq!(goal.remaining, 60_000);
    }

    #[test]
    fn test_update_rejects_duplicate_name_and_empty_name() {
        let (_dir, conn) = test_conn();
        create_goal(&conn, "旅行", 100_000, None, None).unwrap();
        create_goal(&conn, "买车", 100_000, None, None).unwrap();
        let second = list_goals(&conn)
            .unwrap()
            .into_iter()
            .find(|g| g.name == "买车")
            .unwrap();

        let dup = update_goal(&conn, &second.id, Some("旅行"), None, None, None).unwrap_err();
        assert!(format!("{dup}").contains("同名"));

        let empty = update_goal(&conn, &second.id, Some(" "), None, None, None).unwrap_err();
        assert!(format!("{empty}").contains("名称"));
    }
}
