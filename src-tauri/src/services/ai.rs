use crate::db::AppError;
use chrono::Local;

// ---------------------------------------------------------------------------
// Configuration & data structures
// ---------------------------------------------------------------------------

/// AI service configuration loaded from settings.
///
/// `Debug` is implemented manually so the API key can never leak into logs.
pub(crate) struct AiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl std::fmt::Debug for AiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &"***")
            .field("model", &self.model)
            .finish()
    }
}

/// Maximum number of characters of an upstream body kept in logs.
const LOG_BODY_LIMIT: usize = 200;

/// Truncate an upstream response body so diagnostics stay useful without
/// dumping a full provider payload (which can echo user data) into stderr.
fn log_body(body: &str) -> String {
    let mut out: String = body.chars().take(LOG_BODY_LIMIT).collect();
    if body.chars().nth(LOG_BODY_LIMIT).is_some() {
        out.push('…');
    }
    out
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "[::1]" | "::1")
}

/// Enforce `https://` for the AI endpoint. Plain `http://` is only allowed for
/// loopback hosts (local mock servers and development).
pub(crate) fn validate_base_url(raw: &str) -> Result<(), AppError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let (scheme, rest) = if let Some(rest) = lower.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = lower.strip_prefix("http://") {
        ("http", rest)
    } else {
        return Err(AppError::ValidationError(
            "errors.ai.insecureBaseUrl".to_string(),
        ));
    };

    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // Drop userinfo if present, then isolate the host from the port.
    let authority = authority.rsplit('@').next().unwrap_or_default();
    let host = if let Some(stripped) = authority.strip_prefix('[') {
        stripped.split(']').next().unwrap_or_default().to_string()
    } else {
        authority.split(':').next().unwrap_or_default().to_string()
    };
    let host = if authority.starts_with('[') {
        format!("[{}]", host)
    } else {
        host
    };

    if scheme == "http" && !is_loopback_host(&host) {
        return Err(AppError::ValidationError(
            "errors.ai.insecureBaseUrl".to_string(),
        ));
    }
    if scheme == "https" && host.is_empty() {
        return Err(AppError::ValidationError(
            "errors.ai.insecureBaseUrl".to_string(),
        ));
    }
    Ok(())
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
///
/// Reuses the decimal-only parser so an LLM response such as `"1e309"` can
/// never reach the database as a saturated `i64::MAX` amount.
pub(crate) fn validate_amount(amount: &str) -> Result<(), AppError> {
    let cents = crate::utils::money::parse_amount_to_cents(amount)
        .map_err(|_| AppError::AiError("invalidAmount".to_string()))?;
    if cents <= 0 {
        return Err(AppError::AiError("invalidAmount".to_string()));
    }
    Ok(())
}

/// Validate AI-returned date string. Must be YYYY-MM-DD, parseable, not too
/// far future (>1 day) or before 1900. null/empty -> missingDate error.
pub(crate) fn validate_date(date: &Option<String>) -> Result<String, AppError> {
    let date_str = match date {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return Err(AppError::AiError("missingDate".to_string())),
    };

    // Reuse the shared strict-format parser; the "not too far in the future"
    // rule below is AI-specific and intentionally not part of it.
    let parsed = crate::utils::validation::parse_business_date(&date_str)
        .map_err(|_| AppError::AiError("invalidDate".to_string()))?;

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
        status,
        log_body(&error_body)
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
            status2,
            log_body(&error_body2)
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
            status,
            log_body(&error_body)
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
        assert!(validate_amount("18.00").is_ok());
    }

    #[test]
    fn test_validate_amount_integer() {
        assert!(validate_amount("100").is_ok());
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
    fn test_validate_amount_rejects_non_finite_and_overflow() {
        for value in ["inf", "NaN", "1e309", "9e18", "9223372036854775807"] {
            assert!(
                validate_amount(value).is_err(),
                "expected {value:?} to be rejected"
            );
        }
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

    // -----------------------------------------------------------------------
    // HTTP tests (local TCP mock server)
    // -----------------------------------------------------------------------

    use std::io::{Read, Write};

    static NO_PROXY_ONCE: std::sync::Once = std::sync::Once::new();

    /// Bypass any ambient HTTP proxy so requests hit the local mock directly.
    /// Must run before reqwest builds its client (i.e. before each request).
    fn ensure_no_proxy() {
        NO_PROXY_ONCE.call_once(|| {
            for var in [
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "ALL_PROXY",
                "http_proxy",
                "https_proxy",
                "all_proxy",
            ] {
                std::env::remove_var(var);
            }
        });
    }

    /// Serve `responses` sequentially on a random local port.
    /// Returns the base URL to use as ai.base_url.
    fn spawn_mock_server(responses: Vec<(u16, &'static str, String)>) -> String {
        ensure_no_proxy();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for (status, reason, body) in responses {
                let (mut stream, _) = match listener.accept() {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let mut buf = [0u8; 8192];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    reason,
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("http://{}", addr)
    }

    fn chat_body(content: &str) -> String {
        serde_json::json!({ "choices": [{ "message": { "content": content } }] }).to_string()
    }

    fn parse_result_json(mode: &str, amount: &str) -> String {
        format!(
            r#"{{"mode":"{}","amount":"{}","categoryName":null,"accountHint":null,"date":"2026-01-15","description":"lunch"}}"#,
            mode, amount
        )
    }

    fn test_config(base_url: String) -> AiConfig {
        AiConfig {
            base_url,
            api_key: "sk-test".to_string(),
            model: "test-model".to_string(),
        }
    }

    fn test_context() -> AiContext {
        AiContext {
            today: "2026-01-15".to_string(),
            timezone_offset: "+08:00".to_string(),
            expense_categories: vec![],
            income_categories: vec![],
            asset_accounts: vec![],
        }
    }

    #[tokio::test]
    async fn test_parse_transaction_success_first_attempt() {
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&parse_result_json("expense", "12.00")),
        )]);
        let config = test_config(base_url);
        let result = parse_transaction(&config, "lunch 12", &test_context())
            .await
            .unwrap();
        assert_eq!(result.mode, "expense");
        assert_eq!(result.amount, "12.00");
    }

    #[tokio::test]
    async fn test_parse_transaction_retries_without_response_format_on_400() {
        let base_url = spawn_mock_server(vec![
            (400, "Bad Request", "{}".to_string()),
            (
                200,
                "OK",
                chat_body(&format!("ok {}", parse_result_json("income", "5000"))),
            ),
        ]);
        let config = test_config(base_url);
        let result = parse_transaction(&config, "salary", &test_context())
            .await
            .unwrap();
        assert_eq!(result.mode, "income");
        assert_eq!(result.amount, "5000");
    }

    #[tokio::test]
    async fn test_parse_transaction_both_attempts_fail() {
        let base_url = spawn_mock_server(vec![
            (400, "Bad Request", "{}".to_string()),
            (500, "Internal Server Error", "{}".to_string()),
        ]);
        let config = test_config(base_url);
        let err = parse_transaction(&config, "x", &test_context())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::AiError(ref m) if m == "apiCallFailed"),
            "unexpected error: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_parse_transaction_non_400_error_no_retry() {
        let base_url = spawn_mock_server(vec![(401, "Unauthorized", "{}".to_string())]);
        let config = test_config(base_url);
        let err = parse_transaction(&config, "x", &test_context())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::AiError(ref m) if m == "apiCallFailed"),
            "unexpected error: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_parse_transaction_empty_choices_is_parse_failed() {
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            serde_json::json!({ "choices": [] }).to_string(),
        )]);
        let config = test_config(base_url);
        let err = parse_transaction(&config, "x", &test_context())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::AiError(ref m) if m == "parseFailed"),
            "unexpected error: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_parse_transaction_invalid_content_is_parse_failed() {
        let base_url = spawn_mock_server(vec![(200, "OK", chat_body("no json inside"))]);
        let config = test_config(base_url);
        let err = parse_transaction(&config, "x", &test_context())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::AiError(ref m) if m == "parseFailed"),
            "unexpected error: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_parse_transaction_malformed_json_body_is_parse_failed() {
        let base_url = spawn_mock_server(vec![(200, "OK", "not-json".to_string())]);
        let config = test_config(base_url);
        let err = parse_transaction(&config, "x", &test_context())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::AiError(ref m) if m == "parseFailed"),
            "unexpected error: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_parse_transaction_connection_refused_is_network_error() {
        ensure_no_proxy();
        // Bind then immediately drop the listener so nothing accepts connections.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let config = test_config(format!("http://{}", addr));
        let err = parse_transaction(&config, "x", &test_context())
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::AiError(ref m) if m == "networkError"),
            "unexpected error: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_test_connection_success() {
        let base_url = spawn_mock_server(vec![(200, "OK", "{}".to_string())]);
        test_connection(&test_config(base_url)).await.unwrap();
    }

    #[tokio::test]
    async fn test_test_connection_failure() {
        let base_url = spawn_mock_server(vec![(500, "Internal Server Error", "{}".to_string())]);
        let err = test_connection(&test_config(base_url)).await.unwrap_err();
        assert!(
            matches!(err, AppError::AiError(ref m) if m == "apiCallFailed"),
            "unexpected error: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_test_connection_network_error() {
        ensure_no_proxy();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let err = test_connection(&test_config(format!("http://{}", addr)))
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::AiError(ref m) if m == "networkError"),
            "unexpected error: {:?}",
            err
        );
    }

    #[test]
    fn test_ai_config_debug_redacts_api_key() {
        let config = AiConfig {
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "sk-super-secret".to_string(),
            model: "gpt-4o-mini".to_string(),
        };
        let rendered = format!("{:?}", config);
        assert!(!rendered.contains("sk-super-secret"), "{rendered}");
        assert!(rendered.contains("api_key: \"***\""));
        assert!(rendered.contains("https://api.example.com/v1"));
    }

    #[test]
    fn test_log_body_truncates_long_payloads() {
        let long = "a".repeat(LOG_BODY_LIMIT + 50);
        let logged = log_body(&long);
        assert_eq!(logged.chars().count(), LOG_BODY_LIMIT + 1);
        assert!(logged.ends_with('…'));
        assert_eq!(log_body("short"), "short");
    }

    #[test]
    fn test_validate_base_url_allows_https_and_loopback() {
        for url in [
            "https://api.openai.com/v1",
            "https://api.example.com/v1/",
            "http://127.0.0.1:9",
            "http://localhost:8080/v1",
            "http://[::1]:1234",
        ] {
            assert!(validate_base_url(url).is_ok(), "should allow {url}");
        }
    }

    #[test]
    fn test_validate_base_url_rejects_insecure_remote_hosts() {
        for url in [
            "http://api.example.com/v1",
            "http://10.0.0.2:8080",
            "ftp://api.example.com",
            "api.example.com/v1",
            "",
            "   ",
            "https://",
        ] {
            assert!(validate_base_url(url).is_err(), "should reject {url}");
        }
    }
}
