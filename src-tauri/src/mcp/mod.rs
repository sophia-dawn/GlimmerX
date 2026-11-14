//! MCP server assembly and lifecycle (design doc 25, §5 / §10.5).
//!
//! The server runs inside the app process: the SQLCipher key, the
//! `Mutex<Option<Database>>` lock semantics and the localized error text are
//! all shared with the UI for free. Streamable HTTP on `127.0.0.1` only,
//! stateless (`legacy_session_mode(false)`, JSON responses), guarded by a
//! Bearer token that lives in the encrypted settings table.
//!
//! Everything the server needs from the app goes through [`McpHost`], so
//! the whole assembly is testable with a mock app.
//!
//! Lifecycle: the settings (token included) are only readable while the
//! ledger is unlocked, so the server materializes on first unlock and on
//! every config change; it keeps running while the app runs — a locked
//! ledger simply makes every tool report "the ledger is locked".

pub mod approval;
pub mod host;
pub mod settings;
pub mod tools;

#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex, OnceLock};

use tauri::async_runtime::JoinHandle;

use host::McpHost;
use settings::McpConfig;

/// Runtime state of the MCP server, managed by Tauri.
///
/// The [`McpHost`] is injected in the app's `.setup()` hook (a Wry handle in
/// production, a mock handle in tests) so commands never carry `AppHandle`
/// parameters, which tauri's mock runtime cannot bind in tests.
pub struct McpRuntime {
    host: OnceLock<Arc<dyn McpHost>>,
    inner: Mutex<McpRuntimeInner>,
}

impl Default for McpRuntime {
    fn default() -> Self {
        Self {
            host: OnceLock::new(),
            inner: Mutex::new(McpRuntimeInner::default()),
        }
    }
}

impl McpRuntime {
    /// Called once from the app's setup hook (or from a test harness).
    pub fn set_handle(&self, host: Arc<dyn McpHost>) {
        let _ = self.host.set(host);
    }

    pub fn handle(&self) -> Option<&Arc<dyn McpHost>> {
        self.host.get()
    }

    pub(crate) fn inner_mut(&self) -> std::sync::MutexGuard<'_, McpRuntimeInner> {
        self.inner.lock().expect("mcp runtime poisoned")
    }
}

#[derive(Default)]
pub(crate) struct McpRuntimeInner {
    /// Handle of the axum serve task; dropping it does NOT stop the server,
    /// so `abort` terminates it explicitly.
    pub(crate) task: Option<JoinHandle<()>>,
    /// The port actually bound (may drift from the configured one when the
    /// preferred port was occupied).
    pub(crate) port: Option<u16>,
    /// `true` from spawn until `serve` exits or the task is aborted.
    pub(crate) running: bool,
}

/// How many ports above the configured one to try before giving up.
const PORT_ATTEMPTS: u16 = 10;

/// (Re)start or stop the server according to the current settings. Called
/// after unlock and after every config change; safe to call repeatedly.
pub fn sync(host: &Arc<dyn McpHost>) {
    let Some(config) = host.mcp_config().ok() else {
        // Locked or unreadable: nothing to run. The tools report the lock
        // themselves; the server has no secrets to serve.
        host.abort_runtime();
        return;
    };

    if !config.enabled {
        host.abort_runtime();
        return;
    }

    let status = host.runtime_status();
    // Already running with the same port? Nothing to do. The token is not
    // re-checked here on purpose: rotating it restarts the server through
    // `restart`, which always rebuilds.
    if status.running && status.port == Some(config.port) {
        return;
    }
    host.abort_runtime();

    let task_host = host.clone();
    let token = config.token.clone();
    let preferred_port = config.port;
    let task = tauri::async_runtime::spawn(async move {
        if let Err(e) = serve(task_host.clone(), token, preferred_port).await {
            eprintln!("[glimmerx] MCP server stopped: {e}");
        }
    });
    // Optimistic port: `serve` rewrites it with the actually-bound one (the
    // drift window is covered by the status refresh in the settings UI).
    host.set_runtime_task(task, config.port);
}

/// Hard restart, used when the token changed (the auth middleware captured
/// the old token in its closure).
pub fn restart(host: &Arc<dyn McpHost>) {
    host.abort_runtime();
    sync(host);
}

/// Current runtime snapshot for the settings UI.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRuntimeStatus {
    pub running: bool,
    pub port: Option<u16>,
}

async fn serve(host: Arc<dyn McpHost>, token: String, preferred_port: u16) -> Result<(), String> {
    use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
    };

    // Try the configured port, then a few above it, so a user's mcp.json
    // stays valid across the occasional collision.
    let mut listener = None;
    let mut bound_port = preferred_port;
    for offset in 0..PORT_ATTEMPTS {
        let candidate = preferred_port.saturating_add(offset);
        match tokio::net::TcpListener::bind(("127.0.0.1", candidate)).await {
            Ok(l) => {
                listener = Some(l);
                bound_port = candidate;
                break;
            }
            Err(_) => continue,
        }
    }
    let listener = listener.ok_or_else(|| {
        format!(
            "no free port in 127.0.0.1:{preferred_port}..{}",
            preferred_port + PORT_ATTEMPTS - 1
        )
    })?;

    // Write the actually-bound port back for the settings UI.
    host.set_runtime_port(bound_port);

    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    // The factory closure runs per request; the only shared state is the
    // host, which is the whole point of running in-process.
    let tools_host = host.clone();
    let service = StreamableHttpService::new(
        move || Ok(tools::LedgerTools::new(tools_host.clone())),
        LocalSessionManager::default().into(),
        config,
    );

    // Bearer-token gate. Loopback-only + a per-install random token: any
    // other local process is the adversary model.
    let expected = format!("Bearer {token}");
    let auth = axum::middleware::from_fn(
        move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
            // Clone per call: the async block below is `move`, and moving
            // the captured string into it would make the closure FnOnce.
            let expected = expected.clone();
            async move {
                let ok = req
                    .headers()
                    .get(axum::http::header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v == expected);
                if !ok {
                    return axum::http::StatusCode::UNAUTHORIZED.into_response();
                }
                next.run(req).await
            }
        },
    );
    use axum::response::IntoResponse;

    let router = axum::Router::new()
        .route_service("/mcp", service)
        .layer(auth);

    println!("[glimmerx] MCP server listening on http://127.0.0.1:{bound_port}/mcp");
    let result = axum::serve(listener, router)
        .await
        .map_err(|e| e.to_string());
    // The task leaving the loop means the server is down (bind error, or a
    // stray call after the runtime was aborted).
    host.mark_runtime_stopped();
    result
}

// ---------------------------------------------------------------------------
// Managed-state helpers used by the commands module: they resolve the host
// from the runtime so command signatures stay mock-friendly.
// ---------------------------------------------------------------------------

pub fn current_config(runtime: &McpRuntime) -> Result<McpConfig, crate::db::AppError> {
    let host = runtime.handle().ok_or(crate::db::AppError::IoError(
        "MCP runtime not initialized".into(),
    ))?;
    host.mcp_config().map_err(crate::db::AppError::IoError)
}

/// `sync` via the injected host; a no-op before setup (tests, headless).
pub fn sync_from(runtime: &McpRuntime) {
    if let Some(host) = runtime.handle() {
        sync(host);
    }
}

/// `restart` via the injected host.
pub fn restart_from(runtime: &McpRuntime) {
    if let Some(host) = runtime.handle() {
        restart(host);
    }
}

/// `status` via the injected host.
pub fn status_from(runtime: &McpRuntime) -> McpRuntimeStatus {
    match runtime.handle() {
        Some(host) => host.runtime_status(),
        None => McpRuntimeStatus {
            running: false,
            port: None,
        },
    }
}
