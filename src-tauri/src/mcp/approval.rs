//! Approval proxy for MCP writes in "confirm" trust mode (design doc 25, §5.4).
//!
//! Single slot, no queue: a second concurrent write request is an explicit
//! error rather than a queued wait — queueing would blur the timeout
//! semantics on the agent side. The flow is:
//!
//! ```text
//! MCP write tool -> slot.try_insert(pending) -> emit "ledger://approval"
//!   -> webview confirmation card -> mcp_approve_request(id, approved)
//!   -> oneshot resolves -> tool proceeds (or returns the rejection)
//! ```

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::oneshot;

/// How long a write waits for user approval before it is cancelled.
/// Two seconds under test so a missed approval cannot stall the suite.
pub const APPROVAL_TIMEOUT: Duration = if cfg!(test) {
    Duration::from_secs(2)
} else {
    Duration::from_secs(120)
};

/// The single pending approval, as broadcast to the webview.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub request_id: String,
    /// `record_transaction` or `create_account`.
    pub tool: String,
    /// The exact request payload the tool received, verbatim.
    pub request: serde_json::Value,
    pub created_at: String,
}

/// Slot + responder side, never serialized.
pub(crate) struct PendingApproval {
    pub request: ApprovalRequest,
    pub responder: oneshot::Sender<bool>,
}

/// Managed single-slot approval state.
#[derive(Default)]
pub struct ApprovalSlot {
    pending: Mutex<Option<PendingApproval>>,
}

/// Outcome of trying to stage a new approval.
pub(crate) enum StageOutcome {
    /// Someone else is already waiting; the id of the blocking request is
    /// included so the agent's error is actionable.
    Busy(String),
    /// Staged; await the returned receiver for the user's decision.
    Waiting(oneshot::Receiver<bool>),
}

impl ApprovalSlot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage a request in the slot. Fails if the slot is occupied.
    pub(crate) fn try_insert(&self, request: ApprovalRequest) -> StageOutcome {
        let mut guard = self.pending.lock().expect("approval slot poisoned");
        if let Some(existing) = guard.as_ref() {
            return StageOutcome::Busy(existing.request.request_id.clone());
        }
        let (tx, rx) = oneshot::channel();
        *guard = Some(PendingApproval {
            request,
            responder: tx,
        });
        StageOutcome::Waiting(rx)
    }

    /// Resolve (and clear) the pending request by id. Returns whether the id
    /// matched; a mismatch means the card is stale and was replaced/expired,
    /// and the pending request stays untouched.
    pub fn resolve(&self, request_id: &str, approved: bool) -> bool {
        let mut guard = self.pending.lock().expect("approval slot poisoned");
        match guard.take() {
            Some(pending) if pending.request.request_id == request_id => {
                // A dropped receiver just means the tool already timed out;
                // the decision is then irrelevant, not an error.
                let _ = pending.responder.send(approved);
                true
            }
            // Stale id: put the request back; it is still waiting.
            other => {
                *guard = other;
                false
            }
        }
    }

    /// The pending request for the webview (e.g. to re-render after focus).
    pub fn current(&self) -> Option<ApprovalRequest> {
        self.pending
            .lock()
            .expect("approval slot poisoned")
            .as_ref()
            .map(|p| p.request.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str) -> ApprovalRequest {
        ApprovalRequest {
            request_id: id.to_string(),
            tool: "record_transaction".into(),
            request: serde_json::json!({ "kind": "expense" }),
            created_at: "2026-11-14T12:00:00Z".into(),
        }
    }

    #[test]
    fn test_single_slot_rejects_second() {
        let slot = ApprovalSlot::new();
        let StageOutcome::Waiting(rx) = slot.try_insert(request("a")) else {
            panic!("first insert must succeed");
        };
        match slot.try_insert(request("b")) {
            StageOutcome::Busy(id) => assert_eq!(id, "a"),
            _ => panic!("second insert must be busy"),
        }
        // Unblock the slot and try again.
        assert!(slot.resolve("a", true));
        std::mem::forget(rx); // receiver may outlive the test body
        assert!(matches!(
            slot.try_insert(request("b")),
            StageOutcome::Waiting(_)
        ));
    }

    #[test]
    fn test_resolve_delivers_decision() {
        let slot = ApprovalSlot::new();
        let StageOutcome::Waiting(rx) = slot.try_insert(request("x")) else {
            panic!("insert failed");
        };
        assert!(slot.resolve("x", true));
        // A blocking wait on a resolved oneshot returns immediately.
        assert_eq!(
            rx.blocking_recv(),
            Ok(true),
            "approval decision must reach the tool"
        );
        assert!(slot.current().is_none(), "slot cleared after resolve");
    }

    #[test]
    fn test_stale_id_is_rejected() {
        let slot = ApprovalSlot::new();
        let StageOutcome::Waiting(rx) = slot.try_insert(request("live")) else {
            panic!("insert failed");
        };
        assert!(!slot.resolve("stale", true), "unknown id must not resolve");
        assert!(slot.current().is_some(), "pending request stays");
        std::mem::forget(rx);
    }

    #[test]
    fn test_drop_of_receiver_is_not_an_error() {
        let slot = ApprovalSlot::new();
        let StageOutcome::Waiting(rx) = slot.try_insert(request("gone")) else {
            panic!("insert failed");
        };
        drop(rx); // tool timed out and went away
                  // Resolving the stale approval reports success (the id matched) —
                  // the decision simply has no one left to reach.
        assert!(slot.resolve("gone", false));
    }
}
