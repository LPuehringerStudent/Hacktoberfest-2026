//! The io_uring server over real HTTP: same routes as Axum, keep-alive, concurrency, bodies larger
//! than one receive buffer, and SSE streaming through the relay. Skipped if io_uring is unavailable.

use axum::{Router, routing::post};
use backend::{AppState, app, relay::ModelConfig, uring};
use serde_json::{Value, json};

const SSE: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\ndata: [DONE]\n\n";

async fn fake_model() -> String {
    let router = Router::new().route(
        "/chat/completions",
        post(|| async { ([("content-type", "text/event-stream")], SSE) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    url
}

async fn start() -> Option<String> {
    if !uring::available() {
        eprintln!("io_uring unavailable, skipping");
        return None;
    }
    let cfg = ModelConfig {
        base_url: fake_model().await,
        api_key: "k".into(),
        model: "m".into(),
        relay_token: None,
    };
    let router = app(AppState::with_fixtures().with_model(cfg), vec![]);
    let addr = uring::serve(
        router,
        "127.0.0.1:0".parse().unwrap(),
        2,
        tokio::runtime::Handle::current(),
    )
    .unwrap();
    Some(format!("http://{addr}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn routes_over_io_uring() {
    let Some(base) = start().await else { return };
    let http = reqwest::Client::new();

    let health: Value = http
        .get(format!("{base}/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(health, json!({ "ok": true }));

    // Keep-alive: many requests on the same client connection
    for _ in 0..20 {
        let round: Value = http
            .post(format!("{base}/api/round"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(round["scenarios"].as_array().unwrap().len(), 5);
    }

    let s: Value = http
        .get(format!("{base}/api/scenario/fx_call_mfa"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(s.get("isScam").is_none(), "answers stay hidden");

    let r = http
        .get(format!("{base}/api/scenario/nope"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);

    let d: Value = http
        .post(format!("{base}/api/scenario/fx_email_m365/decision"))
        .json(&json!({ "action": "report" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(d["correct"], true);

    // Body spanning several 4 KiB provided buffers
    let big =
        json!({ "type": "open", "detail": { "x": "a".repeat(1500) }, "pad": "b".repeat(12_000) });
    let r = http
        .post(format!("{base}/api/scenario/fx_email_m365/event"))
        .json(&big)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_requests() {
    let Some(base) = start().await else { return };
    let http = reqwest::Client::new();
    let tasks: Vec<_> = (0..300)
        .map(|_| {
            let (http, url) = (http.clone(), format!("{base}/health"));
            tokio::spawn(async move { http.get(url).send().await.unwrap().status() })
        })
        .collect();
    for t in tasks {
        assert_eq!(t.await.unwrap(), 200);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn relay_streams_sse() {
    let Some(base) = start().await else { return };
    let res = reqwest::Client::new()
        .post(format!("{base}/v1/chat/completions"))
        .json(&json!({
            "stream": true,
            "elevenlabs_extra_body": { "scenario_id": "fx_call_mfa" },
            "messages": [{ "role": "user", "content": "hi" }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "text/event-stream");
    assert_eq!(res.text().await.unwrap(), SSE);
}
