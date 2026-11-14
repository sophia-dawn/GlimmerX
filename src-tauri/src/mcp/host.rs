//! The seam between the MCP server and the Tauri app (design doc 25, §5.1).
//!
//! The server runs in-process, but `AppHandle<Wry>` cannot be constructed in
//! tests. Everything the server needs from the app is therefore funnelled
//! through this trait, implemented blanket-style for every
//! `AppHandle<R: Runtime>` — Wry in production, `MockRuntime` in tests.

use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::db::{AppError, Database};

use super::approval::{ApprovalRequest, StageOutcome};
use super::settings::McpConfig;
use super::{McpRuntime, McpRuntimeStatus};

/// A boxed database closure: the only way a `&Connection` can leave the
/// guards that own it.
pub type DbFn =
    Box<dyn FnOnce(&rusqlite::Connection) -> Result<serde_json::Value, AppError> + Send>;

pub trait McpHost: Send + Sync + 'static {
    /// Run a closure with the ledger connection. `Err` carries a
    /// user-facing message (locked / connection failure / app error).
    fn with_db(&self, f: DbFn) -> Result<serde_json::Value, String>;

    /// The path of the active database, if unlocked.
    fn db_path(&self) -> Option<String>;

    /// Load the MCP configuration from the settings table.
    fn mcp_config(&self) -> Result<McpConfig, String>;

    /// Stage a pending approval in the single slot.
    fn try_stage_approval(&self, request: ApprovalRequest) -> StageOutcome;

    /// Broadcast the approval request to the webview.
    fn emit_approval(&self, request: &ApprovalRequest);

    /// Record the serve task + optimistic port; marks the runtime running.
    fn set_runtime_task(&self, task: JoinHandle<()>, port: u16);

    /// Correct the port to the actually-bound one.
    fn set_runtime_port(&self, port: u16);

    /// Abort the serve task and mark the runtime stopped.
    fn abort_runtime(&self);

    /// Mark the runtime stopped (the serve loop exited on its own).
    fn mark_runtime_stopped(&self);

    /// Running + port snapshot.
    fn runtime_status(&self) -> McpRuntimeStatus;
}

impl<R: Runtime> McpHost for AppHandle<R> {
    fn with_db(&self, f: DbFn) -> Result<serde_json::Value, String> {
        let state = self.state::<crate::AppState>();
        let db_state = state.database.lock().map_err(|e| e.to_string())?;
        let db = db_state
            .as_ref()
            .ok_or_else(|| crate::errors::message("errors.databaseLocked").to_string())?;
        call_with_db(db, f)
    }

    fn db_path(&self) -> Option<String> {
        let state = self.state::<crate::AppState>();
        state.database.lock().ok().and_then(|guard| {
            guard
                .as_ref()
                .map(|db| db.path.to_string_lossy().into_owned())
        })
    }

    fn mcp_config(&self) -> Result<McpConfig, String> {
        let state = self.state::<crate::AppState>();
        let db_state = state.database.lock().map_err(|e| e.to_string())?;
        let db = db_state
            .as_ref()
            .ok_or_else(|| crate::errors::message("errors.databaseLocked").to_string())?;
        with_conn(db, |conn| {
            super::settings::load_config(conn).map_err(|e| e.to_string())
        })
    }

    fn try_stage_approval(&self, request: ApprovalRequest) -> StageOutcome {
        self.state::<super::approval::ApprovalSlot>()
            .try_insert(request)
    }

    fn emit_approval(&self, request: &ApprovalRequest) {
        // A missing listener (headless test, closed window) is fine: the
        // timeout on the tool side cancels the write.
        let _ = self.emit("ledger://approval", request);
    }

    fn set_runtime_task(&self, task: JoinHandle<()>, port: u16) {
        let runtime = self.state::<McpRuntime>();
        let mut inner = runtime.inner_mut();
        inner.task = Some(task);
        inner.port = Some(port);
        inner.running = true;
    }

    fn set_runtime_port(&self, port: u16) {
        let runtime = self.state::<McpRuntime>();
        let mut inner = runtime.inner_mut();
        inner.port = Some(port);
    }

    fn abort_runtime(&self) {
        let runtime = self.state::<McpRuntime>();
        let mut inner = runtime.inner_mut();
        if let Some(task) = inner.task.take() {
            task.abort();
        }
        inner.port = None;
        inner.running = false;
    }

    fn mark_runtime_stopped(&self) {
        let runtime = self.state::<McpRuntime>();
        let mut inner = runtime.inner_mut();
        inner.running = false;
        inner.port = None;
    }

    fn runtime_status(&self) -> McpRuntimeStatus {
        let runtime = self.state::<McpRuntime>();
        let inner = runtime.inner_mut();
        McpRuntimeStatus {
            running: inner.running,
            port: if inner.running { inner.port } else { None },
        }
    }
}

fn with_conn<T>(
    db: &Database,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;
    f(&conn)
}

fn call_with_db(db: &Database, f: DbFn) -> Result<serde_json::Value, String> {
    let conn = db.get_conn().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}
