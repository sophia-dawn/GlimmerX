//! User-facing error messages (internationalized).
//!
//! AGENTS.md hard rule: the backend produces friendly, localized messages and
//! the frontend displays them verbatim. `errors.*` keys are an internal
//! representation; they are rendered here before leaving a Tauri command.
//! The zh/en tables are generated from `src/i18n/locales/*.json`.

mod messages;

use std::sync::atomic::{AtomicU8, Ordering};

use messages::Lang;

/// 0 = zh (default), 1 = en. Synchronized from the frontend on startup and
/// whenever the UI language changes.
static UI_LANGUAGE: AtomicU8 = AtomicU8::new(0);

pub fn set_language(language: &str) {
    UI_LANGUAGE.store(if language == "en" { 1 } else { 0 }, Ordering::Relaxed);
}

fn current_lang() -> Lang {
    if UI_LANGUAGE.load(Ordering::Relaxed) == 1 {
        Lang::En
    } else {
        Lang::Zh
    }
}

/// Render a message key into a localized, user-facing string.
/// Plain text (already friendly) is returned unchanged.
pub fn render(input: &str) -> String {
    let Some(rest) = input.strip_prefix("errors.") else {
        return input.to_string();
    };
    // Some call sites append diagnostics ("errors.key: detail"); keep only the
    // key itself so those details never leak to the UI.
    let key: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
        .collect();
    match messages::lookup(current_lang(), &key) {
        Some(text) => text.to_string(),
        None => {
            // Never return the raw key: it would show up in the UI as
            // "errors.something". Log it and fall back to a generic message.
            eprintln!("[glimmerx] missing error message for key: {key}");
            fallback()
        }
    }
}

/// Generic localized message used when a specific one cannot be resolved.
fn fallback() -> String {
    messages::lookup(current_lang(), "unknownError")
        .unwrap_or("Operation failed")
        .to_string()
}

/// Render a known key for call sites that build a message directly.
pub fn message(key: &str) -> String {
    render(key)
}

/// Convert raw backend text into a user-facing message.
///
/// Already-friendly text (typically localized, non-ASCII) passes through.
/// English internals (rusqlite errors, legacy English literals) are replaced by
/// a friendly message and written to stderr for diagnostics.
pub fn user_text(raw: &str) -> String {
    let rendered = render(raw);
    if rendered == raw && !raw.is_empty() && raw.is_ascii() && !raw.starts_with("errors.") {
        eprintln!("[glimmerx] internal error text: {raw}");
        return render("errors.databaseError");
    }
    rendered
}

/// Boundary helper for `Result<_, String>` paths.
///
/// Most db modules (reports / export / import) still return raw `String`
/// errors; this renders keys and replaces untranslated English internals so
/// they never reach the UI as-is. Idempotent: localized text passes through.
pub fn from_error(err: impl std::fmt::Display) -> String {
    user_text(&err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zh_and_en_tables() {
        // Test the tables directly so the process-wide language is never
        // mutated (tests run in parallel).
        assert_eq!(
            messages::lookup(Lang::Zh, "databaseLocked"),
            Some("数据库未解锁，请先解锁")
        );
        assert_eq!(
            messages::lookup(Lang::En, "databaseLocked"),
            Some("Database is locked. Please unlock it first")
        );
    }

    #[test]
    fn test_plain_and_key_with_details() {
        assert_eq!(render("分类名称不能为空"), "分类名称不能为空");
        assert_eq!(
            render("errors.transactionUnbalanced: sum=12"),
            "交易分录必须总和为零"
        );
        assert_eq!(
            user_text("Failed to open database file: permission denied"),
            render("errors.databaseError")
        );
    }

    #[test]
    fn test_missing_key_never_leaks_the_key() {
        let text = render("errors.thisKeyDoesNotExist");
        assert!(
            !text.contains("errors."),
            "internal key leaked to the UI: {text}"
        );
        assert_eq!(text, render("errors.unknownError"));
    }

    #[test]
    fn test_already_localized_text_passes_through() {
        let chinese = "数据库由更新版本创建，请升级应用后再打开";
        assert_eq!(render(chinese), chinese);
        assert_eq!(user_text(chinese), chinese);
    }
}
