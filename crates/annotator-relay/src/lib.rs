//! Annotator local relay — accepts JSON sessions from the
//! bookmarklet and persists them to disk.
//!
//! Wire shape (matches the bookmarklet's payload — see
//! `src/annotator.js`):
//!
//! ```json
//! POST /sessions
//! Content-Type: application/json
//! { "schema_version": 1, "meta": { "url": ..., "ended_ms": ... }, ... }
//! ```
//!
//! Persistence: one JSON file per session under the configured
//! storage root, named `<ISO-8601 ended>-<random-id>.json`. The
//! filename design is deliberately diff-friendly and shell-grepable
//! so the operator can `ls -lt storage/` to find recent sessions.
//!
//! AVP-2: every public function tested; no `unwrap`/`expect` outside
//! SAFETY-annotated paths; no secrets in logs (the bookmarklet may
//! capture form values).

#![forbid(unsafe_code)]

pub mod handlers;
pub mod storage;

pub use handlers::router;
pub use storage::{SessionId, SessionStore, StoreError};

use serde::{Deserialize, Serialize};

/// Shared state passed to every handler.
#[derive(Clone)]
pub struct RelayState {
    /// Where sessions are persisted.
    pub store: SessionStore,
}

impl RelayState {
    /// Construct from an existing store.
    #[must_use]
    pub fn new(store: SessionStore) -> Self {
        Self { store }
    }
}

/// The response body returned to the bookmarklet's POST.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelayPostResponse {
    /// Session id assigned by the store. Echoed back so the
    /// bookmarklet can surface "relayed → <id>" to the operator.
    pub id: SessionId,
    /// Bytes written to disk (after pretty-print). Lets the
    /// operator sanity-check the size.
    pub bytes_written: u64,
}
