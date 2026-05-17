//! HTTP handlers for the annotator-relay.
//!
//! Three routes:
//!   POST /sessions          — bookmarklet uploads a session
//!   GET  /sessions          — list stored sessions (filenames)
//!   GET  /sessions/:name    — fetch one stored session
//!
//! CORS is wide-open by design — the bookmarklet runs on the
//! operator's target site (any origin) and POSTs to the local
//! relay. Operator-controlled local-only daemon; not exposed to
//! the internet.

use crate::{storage::StoreError, RelayPostResponse, RelayState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::CorsLayer;

/// Build the router with all relay endpoints wired.
#[must_use]
pub fn router(state: RelayState) -> Router {
    Router::new()
        .route("/sessions", post(post_session).get(list_sessions))
        .route("/sessions/:name", get(get_session))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn post_session(
    State(state): State<RelayState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<RelayPostResponse>, RelayError> {
    // Round-trip back to bytes so the store's idempotency hash is
    // deterministic regardless of JSON whitespace variation.
    let bytes = serde_json::to_vec(&payload).map_err(StoreError::from)?;
    let (id, bytes_written) = state.store.write_session(&bytes)?;
    tracing::info!(target: "annotator-relay", id = %id.0, bytes = bytes_written, "session stored");
    Ok(Json(RelayPostResponse { id, bytes_written }))
}

async fn list_sessions(State(state): State<RelayState>) -> Result<Json<Vec<String>>, RelayError> {
    Ok(Json(state.store.list()?))
}

async fn get_session(
    State(state): State<RelayState>,
    Path(name): Path<String>,
) -> Result<Response, RelayError> {
    let bytes = state.store.read_session(&name)?;
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        bytes,
    )
        .into_response())
}

/// Error wrapper that converts store errors into HTTP responses.
struct RelayError(StoreError);

impl From<StoreError> for RelayError {
    fn from(e: StoreError) -> Self {
        Self(e)
    }
}

impl IntoResponse for RelayError {
    fn into_response(self) -> Response {
        let (status, msg) = match self.0 {
            StoreError::Json(e) => (StatusCode::BAD_REQUEST, format!("invalid json: {e}")),
            StoreError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => {
                (StatusCode::NOT_FOUND, "not found".into())
            }
            StoreError::Io(e) if e.kind() == std::io::ErrorKind::InvalidInput => {
                (StatusCode::BAD_REQUEST, format!("{e}"))
            }
            StoreError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("io: {e}")),
            StoreError::RootUnavailable(p) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("store root unavailable: {}", p.display()),
            ),
        };
        (status, msg).into_response()
    }
}
