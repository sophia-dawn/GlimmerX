//! MCP server settings (design doc 25, §8.1).
//!
//! All four keys live in the encrypted `settings` table, so they are only
//! readable while the ledger is unlocked — which is also when the server
//! itself is allowed to exist. Defaults: disabled, port 17823, a random
//! token generated on first read, trust mode "confirm".

use rusqlite::Connection;
use serde::Serialize;

use crate::db::settings::{get_setting, set_setting};
use crate::db::AppError;

pub const KEY_ENABLED: &str = "mcp.enabled";
pub const KEY_PORT: &str = "mcp.port";
pub const KEY_TOKEN: &str = "mcp.token";
pub const KEY_TRUST: &str = "mcp.trust";

pub const DEFAULT_PORT: u16 = 17823;

/// Trust tier for MCP write tools (design doc 25, §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    /// Write tools are rejected outright. The default for unknown agents.
    ReadOnly,
    /// Writes wait for user approval (single slot, 120s timeout).
    Confirm,
    /// Writes go straight through; the audit log is the only trace.
    Trust,
}

impl TrustMode {
    pub fn as_str(self) -> &'static str {
        match self {
            TrustMode::ReadOnly => "readonly",
            TrustMode::Confirm => "confirm",
            TrustMode::Trust => "trust",
        }
    }

    pub(in crate::mcp) fn parse(value: &str) -> Option<Self> {
        match value {
            "readonly" => Some(TrustMode::ReadOnly),
            "confirm" => Some(TrustMode::Confirm),
            "trust" => Some(TrustMode::Trust),
            _ => None,
        }
    }
}

/// Effective MCP configuration, as loaded from the settings table.
#[derive(Debug, Clone)]
pub struct McpConfig {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
    pub trust: TrustMode,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            token: new_token(),
            trust: TrustMode::Confirm,
        }
    }
}

/// Generate a fresh 64-hex-char token. `Uuid::v4` twice, concatenated —
/// cheap, and the token never leaves loopback + the encrypted settings row.
fn new_token() -> String {
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("{a}{b}")
}

/// Load the configuration, materializing defaults (token) into the settings
/// table so the value survives restarts and stays identical between reads.
pub fn load_config(conn: &Connection) -> Result<McpConfig, AppError> {
    let enabled = get_setting(conn, KEY_ENABLED)?
        .map(|v| v == "true")
        .unwrap_or(false);
    let port = get_setting(conn, KEY_PORT)?
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let trust = get_setting(conn, KEY_TRUST)?
        .and_then(|v| TrustMode::parse(&v))
        .unwrap_or(TrustMode::Confirm);
    // The token is generated once and persisted; a missing row means the
    // server was never enabled, so the generated token is written back.
    let token = match get_setting(conn, KEY_TOKEN)? {
        Some(existing) if !existing.is_empty() => existing,
        _ => {
            let token = new_token();
            set_setting(conn, KEY_TOKEN, &token)?;
            token
        }
    };

    Ok(McpConfig {
        enabled,
        port,
        token,
        trust,
    })
}

/// Replace the token with a fresh one and return it.
pub fn reset_token(conn: &Connection) -> Result<String, AppError> {
    let token = new_token();
    set_setting(conn, KEY_TOKEN, &token)?;
    Ok(token)
}

/// Persist the enabled flag.
pub fn save_enabled(conn: &Connection, enabled: bool) -> Result<(), AppError> {
    set_setting(conn, KEY_ENABLED, if enabled { "true" } else { "false" })
}

/// Persist the port.
pub fn save_port(conn: &Connection, port: u16) -> Result<(), AppError> {
    set_setting(conn, KEY_PORT, &port.to_string())
}

/// Persist the trust mode. Invalid values are an error, never a silent
/// fallback — a mistyped trust tier is a security decision.
pub fn save_trust(conn: &Connection, trust: &str) -> Result<(), AppError> {
    match TrustMode::parse(trust) {
        Some(_) => set_setting(conn, KEY_TRUST, trust),
        None => Err(AppError::ValidationError(
            "errors.mcp.invalidTrustMode".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open(dir.path().join("test.db")).unwrap();
        conn.execute_batch("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .unwrap();
        (dir, conn)
    }

    #[test]
    fn test_defaults_with_token_persisted() {
        let (_dir, conn) = test_conn();
        let config = load_config(&conn).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.trust, TrustMode::Confirm);
        assert_eq!(config.token.len(), 64);
        // Second load returns the same token (persisted, not regenerated).
        assert_eq!(load_config(&conn).unwrap().token, config.token);
    }

    #[test]
    fn test_roundtrip_all_fields() {
        let (_dir, conn) = test_conn();
        save_enabled(&conn, true).unwrap();
        save_port(&conn, 19000).unwrap();
        save_trust(&conn, "trust").unwrap();
        let config = load_config(&conn).unwrap();
        assert!(config.enabled);
        assert_eq!(config.port, 19000);
        assert_eq!(config.trust, TrustMode::Trust);
    }

    #[test]
    fn test_invalid_trust_rejected() {
        let (_dir, conn) = test_conn();
        assert!(save_trust(&conn, "yolo").is_err());
        // Corrupt value in the table falls back to the safe default.
        set_setting(&conn, KEY_TRUST, "garbage").unwrap();
        assert_eq!(load_config(&conn).unwrap().trust, TrustMode::Confirm);
    }

    #[test]
    fn test_invalid_port_falls_back() {
        let (_dir, conn) = test_conn();
        set_setting(&conn, KEY_PORT, "not-a-port").unwrap();
        assert_eq!(load_config(&conn).unwrap().port, DEFAULT_PORT);
    }

    #[test]
    fn test_reset_token_changes_value() {
        let (_dir, conn) = test_conn();
        let first = load_config(&conn).unwrap().token;
        let second = reset_token(&conn).unwrap();
        assert_ne!(first, second);
        assert_eq!(load_config(&conn).unwrap().token, second);
    }
}
