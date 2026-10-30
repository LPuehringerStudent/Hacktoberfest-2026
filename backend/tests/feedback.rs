//! Smarter debrief (tips, noticed flags) and the round summary with rule-based fallback.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use backend::{AppState, app, relay::ModelConfig};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

/// Model pointed at a closed port, so the summary must fall back to rules.
fn router() -> Router {
    let cfg = ModelConfig {
        base_url: "http://127.0.0.1:9/v1".into(),
        api_key: String::new(),
        model: "m".into(),
        relay_token: None,
    };
    app(AppState::with_fixtures().with_model(cfg), vec![])
}

async fn post(uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
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
async fn debrief_has_tips_and_noticed() {
    let (_, d) = post(
        "/api/scenario/fx_email_m365/decision",
        json!({ "action": "fell_for_it", "observed": ["open", "hover_link"] }),
    )
    .await;
    assert_eq!(d["correct"], false);
    assert_eq!(
        d["tips"].as_array().unwrap().len(),
        d["missed"].as_array().unwrap().len()
    );
    assert_eq!(d["noticed"][0]["type"], "link_mismatch");
    assert!(d["explanation"].as_str().unwrap().contains("noticed"));

    // Old clients (no `observed`) still work
    let (s, d) = post(
        "/api/scenario/fx_email_m365/decision",
        json!({ "action": "report" }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(d["tips"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn round_summary_scores_server_side() {
    let (s, sum) = post(
        "/api/round/summary",
        json!({ "results": [
            { "id": "fx_email_m365", "action": "fell_for_it" },
            { "id": "fx_email_invoice", "action": "comply" },
            { "id": "fx_call_mfa", "action": "report" }
        ]}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(sum["total"], 3);
    assert_eq!(sum["correct"], 2);
    assert_eq!(sum["scamsCaught"], 1);
    assert_eq!(sum["scamsMissed"], 1);
    assert_eq!(sum["falseAlarms"], 0);
    assert_eq!(sum["callsCorrect"], 1);
    assert_eq!(sum["rating"], "solid");
    assert!(!sum["weakSpots"].as_array().unwrap().is_empty());
    assert_eq!(sum["feedbackSource"], "rules");
    assert!(
        sum["feedback"]
            .as_str()
            .unwrap()
            .starts_with("You got 2 of 3")
    );

    let (s, _) = post("/api/round/summary", json!({ "results": [] })).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}
