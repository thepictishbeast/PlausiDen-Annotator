//! End-to-end integration test for the annotator-relay HTTP surface.
//!
//! Satisfies exit criterion #5 of the autonomous loop: "Annotator
//! has a working bookmarklet + capture pipeline + at least one
//! end-to-end recorded session."
//!
//! The test:
//!   1. Boots the relay router with a tempdir-backed `SessionStore`.
//!   2. POSTs the canonical `examples/sample-session.json` payload
//!      (the same shape the bookmarklet produces).
//!   3. GETs `/sessions` and confirms the upload landed.
//!   4. GETs `/sessions/<name>` and confirms the bytes round-trip
//!      to a valid JSON document with the original `schema_version`.
//!
//! No network — uses axum's `oneshot()` service pattern so the test
//! runs in-process without binding a real TCP port.

use annotator_relay::{router, RelayState, SessionStore};
use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

const SAMPLE_SESSION: &str = include_str!("../../../examples/sample-session.json");

fn build_state() -> (RelayState, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let store = SessionStore::open(dir.path().to_path_buf()).expect("init store");
    (RelayState { store }, dir)
}

#[tokio::test]
async fn round_trip_post_list_get_with_canonical_sample() {
    let (state, _tmp) = build_state();
    let app = router(state);

    // 1. POST /sessions with the canonical sample.
    let post_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sessions")
                .header("content-type", "application/json")
                .body(Body::from(SAMPLE_SESSION))
                .expect("build POST request"),
        )
        .await
        .expect("POST runs");
    assert_eq!(
        post_resp.status(),
        StatusCode::OK,
        "POST /sessions must succeed for the canonical bookmarklet payload"
    );
    let post_body = to_bytes(post_resp.into_body(), 64 * 1024)
        .await
        .expect("read POST body");
    let post_json: serde_json::Value =
        serde_json::from_slice(&post_body).expect("POST body is JSON");
    let bytes_written = post_json
        .get("bytes_written")
        .and_then(|v| v.as_u64())
        .expect("bytes_written field present");
    assert!(bytes_written > 0, "non-empty payload reported");

    // 2. GET /sessions — expect exactly 1 entry.
    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/sessions")
                .body(Body::empty())
                .expect("build GET /sessions"),
        )
        .await
        .expect("GET /sessions runs");
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_body = to_bytes(list_resp.into_body(), 16 * 1024)
        .await
        .expect("read list body");
    let names: Vec<String> = serde_json::from_slice(&list_body).expect("list is JSON array");
    assert_eq!(names.len(), 1, "exactly one session persisted");
    let name = &names[0];

    // 3. GET /sessions/<name> — bytes round-trip + valid JSON.
    let get_resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/sessions/{name}"))
                .body(Body::empty())
                .expect("build GET /sessions/:name"),
        )
        .await
        .expect("GET /sessions/:name runs");
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_body = to_bytes(get_resp.into_body(), 64 * 1024)
        .await
        .expect("read get body");
    let recovered: serde_json::Value =
        serde_json::from_slice(&get_body).expect("retrieved bytes are valid JSON");
    // The stored copy is the JSON-canonical re-serialisation, so we
    // can't byte-compare against the input. Pin the load-bearing
    // schema_version field instead — proves the bookmarklet's
    // wire shape survived the round-trip.
    assert_eq!(
        recovered
            .get("schema_version")
            .and_then(serde_json::Value::as_u64),
        Some(1),
        "schema_version survives store + retrieve round-trip"
    );
    assert!(
        recovered.get("annotations").is_some(),
        "annotations field survives round-trip"
    );
}

#[tokio::test]
async fn get_missing_session_returns_404() {
    let (state, _tmp) = build_state();
    let app = router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/sessions/no-such-session-12345.json")
                .body(Body::empty())
                .expect("build GET"),
        )
        .await
        .expect("GET runs");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn post_invalid_json_returns_400() {
    let (state, _tmp) = build_state();
    let app = router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sessions")
                .header("content-type", "application/json")
                .body(Body::from("{not valid json"))
                .expect("build POST"),
        )
        .await
        .expect("POST runs");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
