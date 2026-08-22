use crate::db::AppError;
use chrono::{Datelike, Local, NaiveDate};

// ---------------------------------------------------------------------------
// Configuration & data structures
// ---------------------------------------------------------------------------

/// AI service configuration loaded from settings.
#[derive(Debug)]
pub(crate) struct AiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

/// Context provided to the AI (available categories + accounts).
#[derive(Debug)]
pub(crate) struct AiContext {
    pub today: String,
    pub timezone_offset: String,
    pub expense_categories: Vec<String>,
    pub income_categories: Vec<String>,
    pub asset_accounts: Vec<String>,
}

/// Raw parsed result from AI JSON response.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiParseResult {
    pub mode: String,
    pub amount: String,
    pub category_name: Option<String>,
    pub account_hint: Option<String>,
    pub date: Option<String>,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Validation pure functions
// ---------------------------------------------------------------------------

/// Validate AI-returned mode. Only expense/income allowed (transfer is phase 2).
pub(crate) fn validate_mode(mode: &str) -> Result<(), AppError> {
    match mode {
        "expense" | "income" => Ok(()),
        _ => Err(AppError::AiError("invalidMode".to_string())),
    }
}

/// Validate AI-returned amount string. Must parse to a positive number.
pub(crate) fn validate_amount(amount: &str) -> Result<f64, AppError> {
    match amount.parse::<f64>() {
        Ok(v) if v > 0.0 => Ok(v),
        _ => Err(AppError::AiError("invalidAmount".to_string())),
    }
}

/// Validate AI-returned date string. Must be YYYY-MM-DD, parseable, not too
/// far future (>1 day) or before 1900. null/empty -> missingDate error.
pub(crate) fn validate_date(date: &Option<String>) -> Result<String, AppError> {
    let date_str = match date {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return Err(AppError::AiError("missingDate".to_string())),
    };

    if date_str.len() != 10
        || date_str.chars().nth(4) != Some('-')
        || date_str.chars().nth(7) != Some('-')
    {
        return Err(AppError::AiError("invalidDate".to_string()));
    }

    let parsed = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|_| AppError::AiError("invalidDate".to_string()))?;

    if parsed.year() < 1900 {
        return Err(AppError::AiError("invalidDate".to_string()));
    }

    let today = Local::now().date_naive();
    let tomorrow = today.succ_opt().unwrap_or(today);
    if parsed > tomorrow {
        return Err(AppError::AiError("invalidDate".to_string()));
    }

    Ok(date_str)
}

// ---------------------------------------------------------------------------
// JSON response parsing
// ---------------------------------------------------------------------------

/// Parse the AI message content into AiParseResult.
/// Tries direct JSON parse first; falls back to extracting {...} block.
fn parse_json_response(content: &str) -> Result<AiParseResult, AppError> {
    if let Ok(result) = serde_json::from_str::<AiParseResult>(content) {
        return Ok(result);
    }

    if let Some(json_str) = extract_json_from_content(content) {
        if let Ok(result) = serde_json::from_str::<AiParseResult>(&json_str) {
            return Ok(result);
        }
    }

    Err(AppError::AiError("parseFailed".to_string()))
}

/// Extract the first balanced `{...}` JSON object from a text string.
fn extract_json_from_content(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let mut depth = 0;
    let mut end = start;
    for (i, ch) in content[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth == 0 {
        Some(content[start..end].to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Prompt construction
// ---------------------------------------------------------------------------

/// Build the system prompt with context injected.
fn build_system_prompt(context: &AiContext) -> String {
    let join = |v: &[String]| v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
    format!(
        r#"You are a bookkeeping assistant. Parse the user's natural-language bookkeeping text into structured JSON.

Current date: {today}, timezone offset: {tz}.
Resolve relative dates (e.g. "昨天"->yesterday, "前天"->day before yesterday, "3号"->the 3rd of current month) relative to the current date. If no date given, use today ({today}).

Available expense categories: {exp}
Available income categories: {inc}
Available asset accounts: {acct}

Rules:
1. mode: "expense" or "income" only
2. amount: positive string, two decimals, e.g. "18.00"
3. categoryName: best match from categories above; if none, propose a name (auto-created)
4. accountHint: if user mentions an account, give keyword; else null
5. date: YYYY-MM-DD format, always a concrete date
6. description: brief summary excluding amount and date

Return ONLY JSON:
{{"mode":"expense","amount":"18.00","categoryName":"餐饮","accountHint":null,"date":"{today}","description":"中午吃饭"}}"#,
        today = context.today,
        tz = context.timezone_offset,
        exp = join(&context.expense_categories),
        inc = join(&context.income_categories),
        acct = join(&context.asset_accounts),
    )
}

// ---------------------------------------------------------------------------
// HTTP call
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    temperature: f64,
}

#[derive(serde::Serialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(serde::Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(serde::Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(serde::Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(serde::Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

fn map_reqwest_err(e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::AiError("timeout".to_string())
    } else {
        AppError::AiError("networkError".to_string())
    }
}

/// Call the AI API and return the parsed result.
/// First tries with response_format=json_object; if the provider returns 400
/// (unsupported), retries without it and relies on extract_json_from_content.
pub(crate) async fn parse_transaction(
    config: &AiConfig,
    text: &str,
    context: &AiContext,
) -> Result<AiParseResult, AppError> {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| AppError::AiError("networkError".to_string()))?;

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: build_system_prompt(context),
        },
        ChatMessage {
            role: "user".to_string(),
            content: text.to_string(),
        },
    ];

    // Attempt 1: with response_format json_object (best for providers that support it)
    let body_with_fmt = ChatRequest {
        model: config.model.clone(),
        messages: messages.clone(),
        response_format: Some(ResponseFormat {
            format_type: "json_object".to_string(),
        }),
        temperature: 0.1,
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body_with_fmt)
        .send()
        .await
        .map_err(map_reqwest_err)?;

    let status = resp.status();

    if status.is_success() {
        let chat: ChatResponse = resp
            .json()
            .await
            .map_err(|_| AppError::AiError("parseFailed".to_string()))?;
        let content = chat
            .choices
            .first()
            .ok_or_else(|| AppError::AiError("parseFailed".to_string()))?
            .message
            .content
            .clone();
        return parse_json_response(&content);
    }

    // Log the error for debugging
    let error_body = resp.text().await.unwrap_or_default();
    eprintln!(
        "[ai] parse_transaction first attempt failed: HTTP {} body: {}",
        status, error_body
    );

    // If 400, retry without response_format (provider may not support JSON mode)
    if status.as_u16() == 400 {
        let body_without_fmt = ChatRequest {
            model: config.model.clone(),
            messages,
            response_format: None,
            temperature: 0.1,
        };

        let resp2 = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .json(&body_without_fmt)
            .send()
            .await
            .map_err(map_reqwest_err)?;

        if resp2.status().is_success() {
            let chat: ChatResponse = resp2
                .json()
                .await
                .map_err(|_| AppError::AiError("parseFailed".to_string()))?;
            let content = chat
                .choices
                .first()
                .ok_or_else(|| AppError::AiError("parseFailed".to_string()))?
                .message
                .content
                .clone();
            return parse_json_response(&content);
        }

        let status2 = resp2.status();
        let error_body2 = resp2.text().await.unwrap_or_default();
        eprintln!(
            "[ai] parse_transaction retry (no response_format) also failed: HTTP {} body: {}",
            status2, error_body2
        );
    }

    Err(AppError::AiError("apiCallFailed".to_string()))
}

/// Test connection by sending a minimal chat request (no response_format for
/// maximum provider compatibility).
pub(crate) async fn test_connection(config: &AiConfig) -> Result<(), AppError> {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| AppError::AiError("networkError".to_string()))?;

    let body = ChatRequest {
        model: config.model.clone(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hi".to_string(),
        }],
        response_format: None,
        temperature: 0.0,
    };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(map_reqwest_err)?;

    if !resp.status().is_success() {
        let status = resp.status();
        let error_body = resp.text().await.unwrap_or_default();
        eprintln!(
            "[ai] test_connection failed: HTTP {} body: {}",
            status, error_body
        );
        return Err(AppError::AiError("apiCallFailed".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_mode_expense() {
        assert!(validate_mode("expense").is_ok());
    }

    #[test]
    fn test_validate_mode_income() {
        assert!(validate_mode("income").is_ok());
    }

    #[test]
    fn test_validate_mode_transfer_rejected() {
        assert!(validate_mode("transfer").is_err());
    }

    #[test]
    fn test_validate_mode_invalid() {
        assert!(validate_mode("foo").is_err());
    }

    #[test]
    fn test_validate_amount_valid() {
        assert_eq!(validate_amount("18.00").unwrap(), 18.0);
    }

    #[test]
    fn test_validate_amount_integer() {
        assert_eq!(validate_amount("100").unwrap(), 100.0);
    }

    #[test]
    fn test_validate_amount_zero() {
        assert!(validate_amount("0").is_err());
    }

    #[test]
    fn test_validate_amount_negative() {
        assert!(validate_amount("-5.00").is_err());
    }

    #[test]
    fn test_validate_amount_non_numeric() {
        assert!(validate_amount("abc").is_err());
    }

    #[test]
    fn test_validate_date_valid_today() {
        let today = Local::now().format("%Y-%m-%d").to_string();
        assert!(validate_date(&Some(today)).is_ok());
    }

    #[test]
    fn test_validate_date_invalid_format() {
        assert!(validate_date(&Some("2026-8-1".to_string())).is_err());
        assert!(validate_date(&Some("昨天".to_string())).is_err());
        assert!(validate_date(&Some("2026/08/01".to_string())).is_err());
    }

    #[test]
    fn test_validate_date_future() {
        let future = (Local::now() + chrono::Duration::days(10))
            .format("%Y-%m-%d")
            .to_string();
        assert!(validate_date(&Some(future)).is_err());
    }

    #[test]
    fn test_validate_date_too_old() {
        assert!(validate_date(&Some("1899-01-01".to_string())).is_err());
    }

    #[test]
    fn test_validate_date_missing() {
        assert!(validate_date(&None).is_err());
        assert!(validate_date(&Some("".to_string())).is_err());
        assert!(validate_date(&Some("   ".to_string())).is_err());
    }

    #[test]
    fn test_parse_json_response_success() {
        let content = r#"{"mode":"expense","amount":"18.00","categoryName":"餐饮","accountHint":null,"date":"2026-08-21","description":"中午吃饭"}"#;
        let result = parse_json_response(content).unwrap();
        assert_eq!(result.mode, "expense");
        assert_eq!(result.amount, "18.00");
        assert_eq!(result.category_name, Some("餐饮".to_string()));
        assert_eq!(result.account_hint, None);
        assert_eq!(result.date, Some("2026-08-21".to_string()));
        assert_eq!(result.description, "中午吃饭");
    }

    #[test]
    fn test_parse_json_response_malformed() {
        assert!(parse_json_response("not json").is_err());
    }

    #[test]
    fn test_parse_json_response_extract_from_text() {
        let content = r#"Result: {"mode":"income","amount":"5000","categoryName":"工资","accountHint":null,"date":"2026-08-21","description":"工资"} done."#;
        let result = parse_json_response(content).unwrap();
        assert_eq!(result.mode, "income");
        assert_eq!(result.amount, "5000");
    }

    #[test]
    fn test_parse_json_response_missing_fields() {
        assert!(parse_json_response(r#"{"mode":"expense"}"#).is_err());
    }

    #[test]
    fn test_extract_json_simple() {
        assert_eq!(
            extract_json_from_content(r#"prefix {"a":1} suffix"#).unwrap(),
            r#"{"a":1}"#
        );
    }

    #[test]
    fn test_extract_json_nested() {
        assert_eq!(
            extract_json_from_content(r#"{"a":{"b":2}}"#).unwrap(),
            r#"{"a":{"b":2}}"#
        );
    }

    #[test]
    fn test_extract_json_no_braces() {
        assert!(extract_json_from_content("no json here").is_none());
    }

    #[test]
    fn test_extract_json_unbalanced() {
        assert!(extract_json_from_content(r#"{"a":1"#).is_none());
    }

    #[test]
    fn test_build_system_prompt_includes_context() {
        let context = AiContext {
            today: "2026-08-22".to_string(),
            timezone_offset: "+08:00".to_string(),
            expense_categories: vec!["餐饮".to_string(), "交通".to_string()],
            income_categories: vec!["工资".to_string()],
            asset_accounts: vec!["现金".to_string(), "微信钱包".to_string()],
        };
        let prompt = build_system_prompt(&context);
        assert!(prompt.contains("2026-08-22"));
        assert!(prompt.contains("餐饮"));
        assert!(prompt.contains("交通"));
        assert!(prompt.contains("工资"));
        assert!(prompt.contains("现金"));
        assert!(prompt.contains("微信钱包"));
        assert!(prompt.contains("YYYY-MM-DD"));
    }
}
