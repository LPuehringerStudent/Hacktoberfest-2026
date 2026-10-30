//! SQLite event log with group commit + /api/stats.

use std::{path::PathBuf, sync::atomic::Ordering};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use backend::{AppState, app, db::EventLog};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Temp DB file, removed (with its WAL files) on drop.
struct TempDb(PathBuf);

impl TempDb {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "hacktober-{name}-{}-{nanos}.db",
            std::process::id()
        )))
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", self.0.display()));
        }
    }
}

async fn call(
    router: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[test]
fn group_commit_batches() {
    let db = TempDb::new("batch");
    let log = EventLog::open(&db.0).unwrap();
    for i in 0..1000 {
        log.record("fx_email_m365", "hover_link", &json!({ "n": i }));
    }
    log.flush();
    let events = log.stats.events.load(Ordering::Relaxed);
    let commits = log.stats.commits.load(Ordering::Relaxed);
    assert_eq!(events, 1000);
    assert!(commits <= 10, "1000 events in {commits} transactions");
    let stored: i64 = log
        .reader()
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stored, 1000);
}

#[tokio::test]
async fn stats_from_events_and_decisions() {
    let db = TempDb::new("stats");
    let router = app(
        AppState::with_fixtures().with_event_log(EventLog::open(&db.0).unwrap()),
        vec![],
    );
    let id = "fx_email_m365";
    for kind in ["open", "hover_link", "click_link"] {
        let (s, _) = call(
            &router,
            "POST",
            &format!("/api/scenario/{id}/event"),
            Some(json!({ "type": kind })),
        )
        .await;
        assert_eq!(s, StatusCode::NO_CONTENT);
    }
    for action in ["fell_for_it", "report", "report"] {
        call(
            &router,
            "POST",
            &format!("/api/scenario/{id}/decision"),
            Some(json!({ "action": action })),
        )
        .await;
    }

    let (status, stats) = call(&router, "GET", "/api/stats", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["totals"]["decisions"], 3);
    assert_eq!(stats["totals"]["correct"], 2);
    assert_eq!(stats["totals"]["fellForIt"], 1);
    let sc = &stats["scenarios"][0];
    assert_eq!(sc["id"], id);
    assert_eq!(sc["isScam"], true);
    assert_eq!(sc["clickedLink"], 1);
    assert_eq!(sc["hoveredLink"], 1);
    assert_eq!(stats["storage"]["events"], 6);
    assert!(stats["storage"]["commits"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn oversized_event_rejected_and_stats_need_log() {
    let router = app(AppState::with_fixtures(), vec![]);
    let big = json!({ "type": "open", "detail": { "x": "a".repeat(5000) } });
    let (s, _) = call(
        &router,
        "POST",
        "/api/scenario/fx_email_m365/event",
        Some(big),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = call(&router, "GET", "/api/stats", None).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
}
