//! Round picking (#9) and the ElevenLabs custom-LLM relay (#12) against a fake model server.

use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode, header::AUTHORIZATION},
    routing::post,
};
use backend::{
    AppState, app,
    relay::ModelConfig,
    routes::pick_round,
    scenario::{Channel, Scenario},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const SSE: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\ndata: [DONE]\n\n";

/// 4 scam + 4 legit emails, 1 scam + 1 legit call, built from the fixtures.
fn pool() -> Vec<Scenario> {
    let fx = AppState::with_fixtures().scenarios;
    let email = fx.iter().find(|s| s.channel == Channel::Email).unwrap();
    let call = fx.iter().find(|s| s.channel == Channel::Call).unwrap();
    let mut out = vec![];
    for (i, (base, scam)) in [(email, true); 4]
        .into_iter()
        .chain([(email, false); 4])
        .chain([(call, true), (call, false)])
        .enumerate()
    {
        let mut s = base.clone();
        s.id = format!("s{i}");
        s.is_scam = scam;
        out.push(s);
    }
    out
}

#[test]
fn round_is_mixed() {
    let pool = pool();
    for _ in 0..200 {
        let round = pick_round(&pool, 5);
        assert_eq!(round.len(), 5);
        let mut ids: Vec<_> = round.iter().map(|s| &s.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), 5, "no duplicates");
        assert!(round.iter().any(|s| s.channel == Channel::Call));
        let scams = round.iter().filter(|s| s.is_scam).count();
        assert!((2..=3).contains(&scams), "{scams} scams");
    }
}

#[test]
fn round_with_small_pool() {
    let fx = AppState::with_fixtures().scenarios;
    assert_eq!(pick_round(&fx, 5).len(), fx.len().min(5));
    assert!(pick_round(&[], 5).is_empty());
}

type Captured = Arc<Mutex<Option<(HeaderMap, Value)>>>;

/// Fake OpenAI-compatible model server; returns its base URL and what it received.
async fn fake_model() -> (String, Captured) {
    let captured: Captured = Arc::default();
    let router = Router::new()
        .route(
            "/chat/completions",
            post(
                |State(c): State<Captured>, headers: HeaderMap, body: axum::Json<Value>| async move {
                    *c.lock().unwrap() = Some((headers, body.0));
                    ([("content-type", "text/event-stream")], SSE)
                },
            ),
        )
        .with_state(captured.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (url, captured)
}

fn relay_app(base_url: String, relay_token: Option<&str>) -> Router {
    let cfg = ModelConfig {
        base_url,
        api_key: "test-key".into(),
        model: "gemma-test".into(),
        relay_token: relay_token.map(Into::into),
    };
    app(AppState::with_fixtures().with_model(cfg), vec![])
}

async fn post_json(router: Router, body: Value, auth: Option<&str>) -> (StatusCode, String) {
    let mut req = Request::post("/v1/chat/completions").header("content-type", "application/json");
    if let Some(a) = auth {
        req = req.header(AUTHORIZATION, a);
    }
    let res = router
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn eleven_request(extra: Value, system: &str) -> Value {
    json!({
        "model": "custom",
        "stream": true,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": "Hello?" }
        ],
        "tools": [{ "type": "function", "function": { "name": "end_call", "parameters": {} } }],
        "user_id": "u1",
        "elevenlabs_extra_body": extra
    })
}

#[tokio::test]
async fn relay_injects_persona_and_streams() {
    let (url, captured) = fake_model().await;
    let req = eleven_request(
        json!({ "scenario_id": "fx_call_mfa" }),
        "agent prompt from the dashboard",
    );
    let (status, body) = post_json(relay_app(url, None), req, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, SSE, "SSE is passed through unchanged");

    let (headers, sent) = captured.lock().unwrap().take().unwrap();
    assert_eq!(headers[AUTHORIZATION], "Bearer test-key");
    assert_eq!(sent["model"], "gemma-test");
    assert!(sent.get("elevenlabs_extra_body").is_none());
    assert!(sent.get("user_id").is_none());
    assert_eq!(
        sent["tools"][0]["function"]["name"], "end_call",
        "tools pass through"
    );
    let msgs = sent["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 2, "agent system prompt replaced, not added");
    let system = msgs[0]["content"].as_str().unwrap();
    assert!(system.contains("authenticator app"), "persona injected");
    assert!(system.contains("end_call"));
    assert_eq!(msgs[1]["content"], "Hello?");
}

#[tokio::test]
async fn relay_reads_scenario_id_from_agent_prompt() {
    let (url, captured) = fake_model().await;
    let req = eleven_request(
        json!(null),
        "You are a caller. scenario_id: fx_call_mfa\nBe brief.",
    );
    let (status, _) = post_json(relay_app(url, None), req, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(captured.lock().unwrap().is_some());
}

#[tokio::test]
async fn relay_rejects_bad_requests() {
    let (url, captured) = fake_model().await;
    let call = json!({ "scenario_id": "fx_call_mfa" });

    let (s, _) = post_json(
        relay_app(url.clone(), Some("sekrit")),
        eleven_request(call.clone(), ""),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = post_json(
        relay_app(url.clone(), Some("sekrit")),
        eleven_request(call, ""),
        Some("Bearer sekrit"),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, _) = post_json(
        relay_app(url.clone(), None),
        eleven_request(json!(null), "no id here"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = post_json(
        relay_app(url.clone(), None),
        eleven_request(json!({ "scenario_id": "fx_email_m365" }), ""),
        None,
    )
    .await;
    assert_eq!(
        s,
        StatusCode::BAD_REQUEST,
        "email scenarios can't be called"
    );
    let (s, _) = post_json(
        relay_app(url, None),
        eleven_request(json!({ "scenario_id": "nope" }), ""),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    captured.lock().unwrap().take();
}
