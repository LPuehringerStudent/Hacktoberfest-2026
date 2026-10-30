pub mod relay;
pub mod routes;
pub mod scenario;

use std::{
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};

use axum::{
    Router,
    http::HeaderValue,
    routing::{get, post},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use relay::ModelConfig;
use scenario::Scenario;

/// Gemma-generated scenarios, committed to the repo (no live generation). Also the reference for the frontend.
const FIXTURES: &str = include_str!("../fixtures/scenarios.json");

#[derive(Clone)]
pub struct AppState {
    pub scenarios: Arc<Vec<Scenario>>,
    pub rounds: Arc<AtomicU64>,
    pub model: Arc<ModelConfig>,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn with_fixtures() -> Self {
        let scenarios =
            serde_json::from_str(FIXTURES).expect("fixtures/scenarios.json matches the schema");
        Self {
            scenarios: Arc::new(scenarios),
            rounds: Arc::default(),
            model: Arc::new(ModelConfig::from_env()),
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .build()
                .expect("http client"),
        }
    }

    pub fn with_model(mut self, model: ModelConfig) -> Self {
        self.model = Arc::new(model);
        self
    }
}

pub fn app(state: AppState, extra_origins: Vec<String>) -> Router {
    Router::new()
        .route("/health", get(routes::health))
        .route("/api/round", post(routes::new_round))
        .route("/api/scenario/{id}", get(routes::get_scenario))
        .route("/api/scenario/{id}/event", post(routes::record_event))
        .route("/api/scenario/{id}/decision", post(routes::decide))
        // ElevenLabs custom LLM: the server URL may be configured with or without /v1
        .route("/v1/chat/completions", post(relay::chat_completions))
        .route("/chat/completions", post(relay::chat_completions))
        .layer(cors(extra_origins))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Lovable preview + published origins, plus anything in CORS_ORIGINS.
fn cors(extra: Vec<String>) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
            let Ok(o) = origin.to_str() else { return false };
            extra.iter().any(|e| e == o)
                || (o.starts_with("https://")
                    && (o.ends_with(".lovable.app") || o.ends_with(".lovableproject.com")))
        }))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
}
