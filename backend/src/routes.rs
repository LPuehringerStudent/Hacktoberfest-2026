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

pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

pub fn find<'a>(state: &'a AppState, id: &str) -> Result<&'a Scenario, ApiError> {
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

pub async fn new_round(State(state): State<AppState>) -> Json<Round> {
    let n = state.rounds.fetch_add(1, Ordering::Relaxed);
    let scenarios = pick_round(&state.scenarios, ROUND_SIZE)
        .into_iter()
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

/// Random scenarios: at least one call (if any exist), and 2 or 3 scams out of 5.
pub fn pick_round(pool: &[Scenario], size: usize) -> Vec<&Scenario> {
    let mut left: Vec<&Scenario> = pool.iter().collect();
    fastrand::shuffle(&mut left);
    let size = size.min(left.len());
    let want_scams = (size + fastrand::usize(0..=1)) / 2;

    let mut picked = Vec::with_capacity(size);
    if let Some(i) = left.iter().position(|s| s.channel == Channel::Call) {
        picked.push(left.remove(i));
    }
    // First pass steers towards the scam target, second fills with whatever is left
    for strict in [true, false] {
        let mut i = 0;
        while picked.len() < size && i < left.len() {
            let scams = picked.iter().filter(|s| s.is_scam).count();
            if !strict || left[i].is_scam == (scams < want_scams) {
                picked.push(left.remove(i));
            } else {
                i += 1;
            }
        }
    }
    fastrand::shuffle(&mut picked);
    picked
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
