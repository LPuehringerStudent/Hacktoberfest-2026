//! Route smoke test. Any server implementation (Axum now, io_uring later) must keep these passing.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use backend::{AppState, app};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

fn router() -> Router {
    app(AppState::with_fixtures(), vec![])
}

async fn call(method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    let req = req
        .body(body.map_or(Body::empty(), |b| Body::from(b.to_string())))
        .unwrap();
    let res = router().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn health() {
    assert_eq!(
        call("GET", "/health", None).await,
        (StatusCode::OK, json!({ "ok": true }))
    );
}

#[tokio::test]
async fn round_lists_scenarios() {
    let (status, body) = call("POST", "/api/round", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body["scenarios"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn scenario_hides_answers() {
    for id in ["fx_email_m365", "fx_call_mfa"] {
        let (status, body) = call("GET", &format!("/api/scenario/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        let text = body.to_string();
        for secret in [
            "isScam",
            "redFlags",
            "correctAction",
            "\"trap\"",
            "personaPrompt",
            "\"goal\"",
        ] {
            assert!(!text.contains(secret), "{id} leaks {secret}");
        }
    }
}

#[tokio::test]
async fn unknown_scenario_is_404() {
    let (status, body) = call("GET", "/api/scenario/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn event_is_accepted() {
    let ev = json!({ "type": "hover_link", "detail": { "linkId": "l1" } });
    let (status, _) = call("POST", "/api/scenario/fx_email_m365/event", Some(ev)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn decision_debrief() {
    let (_, body) = call(
        "POST",
        "/api/scenario/fx_email_m365/decision",
        Some(json!({ "action": "report" })),
    )
    .await;
    assert_eq!(body["correct"], true);
    assert_eq!(body["missed"].as_array().unwrap().len(), 0);

    let (_, body) = call(
        "POST",
        "/api/scenario/fx_email_m365/decision",
        Some(json!({ "action": "fell_for_it" })),
    )
    .await;
    assert_eq!(body["correct"], false);
    assert!(!body["missed"].as_array().unwrap().is_empty());

    let (_, body) = call(
        "POST",
        "/api/scenario/fx_email_invoice/decision",
        Some(json!({ "action": "comply" })),
    )
    .await;
    assert_eq!(body["correct"], true);
}
