use std::sync::atomic::Ordering;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    AppState,
    scenario::{Action, Channel, RedFlag, Scenario},
};

const ROUND_SIZE: usize = 5;

pub struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

fn find<'a>(state: &'a AppState, id: &str) -> Result<&'a Scenario, ApiError> {
    state
        .scenarios
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("unknown scenario {id}")))
}

pub async fn health() -> Json<Value> {
    Json(json!({ "ok": true }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundRef {
    id: String,
    channel: Channel,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Round {
    round_id: String,
    scenarios: Vec<RoundRef>,
}

// TODO(#pool): draw from the generated scenario pool instead of the fixtures.
pub async fn new_round(State(state): State<AppState>) -> Json<Round> {
    let n = state.rounds.fetch_add(1, Ordering::Relaxed);
    let scenarios = state
        .scenarios
        .iter()
        .cycle()
        .skip(n as usize % state.scenarios.len())
        .take(ROUND_SIZE.min(state.scenarios.len()))
        .map(|s| RoundRef {
            id: s.id.clone(),
            channel: s.channel,
        })
        .collect();
    Json(Round {
        round_id: format!("rd_{n}"),
        scenarios,
    })
}

pub async fn get_scenario(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(find(&state, &id)?.public()))
}

#[derive(Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub detail: Value,
}

// TODO(#sqlite): append to the event log through the group-commit writer.
pub async fn record_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(event): Json<Event>,
) -> Result<StatusCode, ApiError> {
    find(&state, &id)?;
    tracing::info!(scenario = %id, event = %event.kind, detail = %event.detail, "event");
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAction {
    Comply,
    Report,
    FellForIt,
}

#[derive(Deserialize)]
pub struct Decision {
    pub action: DecisionAction,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Debrief {
    correct: bool,
    is_scam: bool,
    correct_action: Action,
    red_flags: Vec<RedFlag>,
    caught: Vec<RedFlag>,
    missed: Vec<RedFlag>,
    safe_signals: Vec<String>,
    explanation: String,
}

// TODO(#debrief): per-flag caught/missed from the event log + Gemma-written explanation.
pub async fn decide(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(decision): Json<Decision>,
) -> Result<Json<Debrief>, ApiError> {
    let s = find(&state, &id)?;
    let reported = decision.action == DecisionAction::Report;
    let correct = reported == s.is_scam;
    let (caught, missed) = if reported {
        (s.red_flags.clone(), vec![])
    } else {
        (vec![], s.red_flags.clone())
    };
    let explanation = match (s.is_scam, correct) {
        (true, true) => "Well spotted: this was a scam.",
        (true, false) => "This was a scam. Look at the red flags you missed.",
        (false, true) => "Correct: this was a normal request.",
        (false, false) => {
            "This one was legitimate. Reporting real requests slows colleagues down too."
        }
    };
    Ok(Json(Debrief {
        correct,
        is_scam: s.is_scam,
        correct_action: s.correct_action,
        red_flags: s.red_flags.clone(),
        caught,
        missed,
        safe_signals: s.safe_signals.clone(),
        explanation: explanation.into(),
    }))
}
