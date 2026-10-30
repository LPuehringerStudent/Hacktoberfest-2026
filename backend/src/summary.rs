//! `POST /api/round/summary`: feedback after a round. Scores are recomputed server-side from the
//! scenarios (the client only says what it did); self-hosted Gemma 4 writes a short personal
//! coaching note, with a rule-based fallback if the model is slow or unreachable.

use std::{collections::BTreeMap, time::Duration};

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    AppState,
    routes::{ApiError, DecisionAction, debrief, find, tip},
    scenario::{Channel, RedFlagType},
};

const GEMMA_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Deserialize)]
pub struct SummaryRequest {
    pub results: Vec<RoundResult>,
}

#[derive(Deserialize)]
pub struct RoundResult {
    pub id: String,
    pub action: DecisionAction,
    #[serde(default)]
    pub observed: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeakSpot {
    #[serde(rename = "type")]
    kind: RedFlagType,
    missed: usize,
    tip: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    total: usize,
    correct: usize,
    scams_caught: usize,
    scams_missed: usize,
    false_alarms: usize,
    calls_correct: usize,
    calls_total: usize,
    /// "expert" | "solid" | "at_risk"
    rating: &'static str,
    weak_spots: Vec<WeakSpot>,
    feedback: String,
    /// "gemma" or "rules"
    feedback_source: &'static str,
}

pub async fn round_summary(
    State(state): State<AppState>,
    Json(req): Json<SummaryRequest>,
) -> Result<Json<Summary>, ApiError> {
    if req.results.is_empty() || req.results.len() > 20 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "send 1-20 results".into(),
        ));
    }
    let (mut correct, mut caught, mut missed_scams, mut false_alarms) = (0, 0, 0, 0);
    let (mut calls_total, mut calls_correct) = (0, 0);
    let mut weak: BTreeMap<RedFlagType, usize> = BTreeMap::new();
    let mut lines = Vec::new();
    for r in &req.results {
        let s = find(&state, &r.id)?;
        let d = debrief(s, r.action, &r.observed);
        let ok = d.correct;
        correct += ok as usize;
        if s.channel == Channel::Call {
            calls_total += 1;
            calls_correct += ok as usize;
        }
        match (s.is_scam, ok) {
            (true, true) => caught += 1,
            (true, false) => {
                missed_scams += 1;
                for f in &s.red_flags {
                    *weak.entry(f.kind).or_default() += 1;
                }
            }
            (false, false) => false_alarms += 1,
            _ => {}
        }
        let title = s
            .email
            .as_ref()
            .map(|e| format!("email \"{}\"", e.subject))
            .or_else(|| {
                s.call
                    .as_ref()
                    .map(|c| format!("call from \"{}\"", c.caller_name))
            })
            .unwrap_or_default();
        let flags: Vec<String> = s
            .red_flags
            .iter()
            .map(|f| {
                serde_json::to_value(f.kind)
                    .unwrap()
                    .as_str()
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        lines.push(format!(
            "- {title}: {} -> user {} ({}){}",
            if s.is_scam { "SCAM" } else { "legitimate" },
            match r.action {
                DecisionAction::Report => "reported it",
                DecisionAction::Comply => "did what was asked",
                DecisionAction::FellForIt => "fell for it",
            },
            if ok { "correct" } else { "wrong" },
            if flags.is_empty() {
                String::new()
            } else {
                format!(", red flags: {}", flags.join(", "))
            }
        ));
    }
    let total = req.results.len();
    let rating = match (correct * 100) / total {
        90.. => "expert",
        60..=89 => "solid",
        _ => "at_risk",
    };
    let mut weak_spots: Vec<WeakSpot> = weak
        .into_iter()
        .map(|(kind, missed)| WeakSpot {
            kind,
            missed,
            tip: tip(kind),
        })
        .collect();
    weak_spots.sort_by_key(|w| std::cmp::Reverse(w.missed));
    weak_spots.truncate(3);

    let fallback = rules_feedback(correct, total, missed_scams, false_alarms, &weak_spots);
    let (feedback, feedback_source) = match tokio::time::timeout(
        GEMMA_TIMEOUT,
        gemma_feedback(&state, &lines, correct, total),
    )
    .await
    {
        Ok(Some(text)) => (text, "gemma"),
        _ => (fallback, "rules"),
    };

    Ok(Json(Summary {
        total,
        correct,
        scams_caught: caught,
        scams_missed: missed_scams,
        false_alarms,
        calls_correct,
        calls_total,
        rating,
        weak_spots,
        feedback,
        feedback_source,
    }))
}

fn rules_feedback(
    correct: usize,
    total: usize,
    missed: usize,
    false_alarms: usize,
    weak: &[WeakSpot],
) -> String {
    let mut out = format!("You got {correct} of {total} right. ");
    out.push_str(match (missed, false_alarms) {
        (0, 0) => "Perfect round: every scam reported, every real request handled.",
        (0, _) => "No scam got past you, but you also reported real requests. Check for actual red flags before reporting.",
        (_, 0) => "Some scams got through. ",
        _ => "Some scams got through, and some real requests were reported. ",
    });
    if let Some(w) = weak.first() {
        out.push_str(&format!("Focus next time: {}", w.tip));
    }
    out.trim().to_owned()
}

async fn gemma_feedback(
    state: &AppState,
    lines: &[String],
    correct: usize,
    total: usize,
) -> Option<String> {
    let cfg = &state.model;
    let mut body = json!({
        "model": cfg.model,
        "max_tokens": 160,
        "temperature": 0.6,
        "messages": [
            { "role": "system", "content": "You are a friendly security-awareness coach for employees of the fictional company Donauwerk Logistik GmbH. After a training round, write 2-3 short sentences of personal feedback in plain English: name what they did well, the one habit to improve (refer to the specific red flags they missed), and one concrete action for next time. No lists, no markdown, no greeting." },
            { "role": "user", "content": format!("Score: {correct}/{total}\n{}", lines.join("\n")) }
        ]
    });
    if !cfg.base_url.contains("googleapis.com") {
        body["chat_template_kwargs"] = json!({ "enable_thinking": false });
    }
    let res = state
        .http
        .post(format!(
            "{}/chat/completions",
            cfg.base_url.trim_end_matches('/')
        ))
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
        .ok()?;
    if !res.status().is_success() {
        return None;
    }
    let v: Value = res.json().await.ok()?;
    let text = v["choices"][0]["message"]["content"].as_str()?;
    // Gemini-hosted Gemma prepends its reasoning
    let text = match text.rfind("</thought>") {
        Some(i) => &text[i + "</thought>".len()..],
        None => text,
    };
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}
