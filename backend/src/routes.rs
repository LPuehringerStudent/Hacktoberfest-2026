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

pub async fn record_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(event): Json<Event>,
) -> Result<StatusCode, ApiError> {
    find(&state, &id)?;
    if event.kind.is_empty() || event.kind.len() > 32 || event.detail.to_string().len() > 2048 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "event too large".into()));
    }
    match &state.events {
        Some(log) => log.record(&id, &event.kind, &event.detail),
        None => {
            tracing::info!(scenario = %id, event = %event.kind, detail = %event.detail, "event")
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
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
    if let Some(log) = &state.events {
        log.record(
            &id,
            "decision",
            &json!({ "action": decision.action, "correct": correct }),
        );
    }
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioStats {
    id: String,
    channel: Channel,
    is_scam: bool,
    title: String,
    opens: u64,
    hovered_link: u64,
    clicked_link: u64,
    decisions: u64,
    correct: u64,
    fell_for_it: u64,
}

/// Aggregates from the event log, e.g. "60% clicked the fake login link". Also reports how well
/// group commit is batching (events per SQLite transaction).
pub async fn stats(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let log = state
        .events
        .clone()
        .ok_or_else(|| ApiError(StatusCode::SERVICE_UNAVAILABLE, "event log disabled".into()))?;
    let rows = tokio::task::spawn_blocking(move || {
        log.flush();
        let rows = crate::db::scenario_counts(&log.reader());
        (rows, log)
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (rows, log) = rows;
    let rows = rows.map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut scenarios: Vec<ScenarioStats> = rows
        .into_iter()
        .filter_map(|r| {
            let s = state.scenarios.iter().find(|s| s.id == r.scenario_id)?;
            let title = s
                .email
                .as_ref()
                .map(|e| e.subject.clone())
                .or_else(|| s.call.as_ref().map(|c| c.caller_name.clone()))
                .unwrap_or_default();
            Some(ScenarioStats {
                id: r.scenario_id,
                channel: s.channel,
                is_scam: s.is_scam,
                title,
                opens: r.opens,
                hovered_link: r.hovered_link,
                clicked_link: r.clicked_link,
                decisions: r.decisions,
                correct: r.correct,
                fell_for_it: r.fell_for_it,
            })
        })
        .collect();
    scenarios.sort_by_key(|s| std::cmp::Reverse(s.decisions));

    let sum = |f: fn(&ScenarioStats) -> u64| scenarios.iter().map(f).sum::<u64>();
    let (decisions, correct) = (sum(|s| s.decisions), sum(|s| s.correct));
    let events = log.stats.events.load(Ordering::Relaxed);
    let commits = log.stats.commits.load(Ordering::Relaxed);
    Ok(Json(json!({
        "totals": {
            "decisions": decisions,
            "correct": correct,
            "correctRate": if decisions == 0 { 0.0 } else { correct as f64 / decisions as f64 },
            "fellForIt": sum(|s| s.fell_for_it),
        },
        "storage": {
            "events": events,
            "commits": commits,
            "eventsPerCommit": if commits == 0 { 0.0 } else { events as f64 / commits as f64 },
        },
        "scenarios": scenarios,
    })))
}
