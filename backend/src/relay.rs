//! ElevenLabs custom-LLM endpoint: swap in the scenario's persona, forward to Gemma 4, stream the reply back.
//!
//! The persona and goal stay on the server; the browser only ever knows the scenario id.

use axum::{
    Json,
    body::Body,
    extract::State,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use crate::{
    AppState,
    routes::{ApiError, find},
    scenario::{Call, Scenario},
};

/// Gemini's OpenAI-compatible endpoint, which also serves the Gemma 4 models.
const GEMINI_OPENAI: &str = "https://generativelanguage.googleapis.com/v1beta/openai";

pub struct ModelConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// If set, callers must send `Authorization: Bearer <token>` (the secret configured in ElevenLabs).
    pub relay_token: Option<String>,
}

impl ModelConfig {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        Self {
            base_url: var("MODEL_BASE_URL").unwrap_or_else(|| GEMINI_OPENAI.into()),
            api_key: var("GEMINI_API_KEY").unwrap_or_default(),
            model: var("GEMMA_MODEL").unwrap_or_else(|| "gemma-4-26b-a4b-it".into()),
            relay_token: var("RELAY_TOKEN"),
        }
    }
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<Value>,
) -> Result<Response, ApiError> {
    let cfg = &state.model;
    if let Some(token) = &cfg.relay_token {
        let sent = headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        if sent != Some(token.as_str()) {
            return Err(ApiError(
                StatusCode::UNAUTHORIZED,
                "invalid relay token".into(),
            ));
        }
    }

    let id = scenario_id(&body).ok_or_else(|| {
        bad_request("no scenario_id: send it as extra body or put `scenario_id: {{scenario_id}}` in the agent prompt")
    })?;
    let scenario = find(&state, &id)?;
    let call = scenario
        .call
        .as_ref()
        .ok_or_else(|| bad_request(&format!("scenario {id} is not a call")))?;

    let obj = body
        .as_object_mut()
        .ok_or_else(|| bad_request("body must be an object"))?;
    obj.remove("elevenlabs_extra_body");
    obj.remove("user_id");
    obj.insert("model".into(), json!(cfg.model));
    if !cfg.base_url.contains("googleapis.com") {
        // Self-hosted llama-server: Gemma 4 thinks for 100+ tokens before speaking unless told not to,
        // which on CPU means ~40 s of silence on the phone
        obj.insert(
            "chat_template_kwargs".into(),
            json!({ "enable_thinking": false }),
        );
    }
    let messages = obj
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| bad_request("missing messages"))?;
    // The agent's own system prompt is replaced: the persona must come from the server
    messages.retain(|m| m["role"] != "system");
    messages.insert(
        0,
        json!({ "role": "system", "content": system_prompt(scenario, call) }),
    );

    let upstream = state
        .http
        .post(format!(
            "{}/chat/completions",
            cfg.base_url.trim_end_matches('/')
        ))
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("model unreachable: {e}")))?;

    let status = upstream.status();
    if !status.is_success() {
        let text = upstream.text().await.unwrap_or_default();
        tracing::warn!(%status, %text, "model error");
        return Err(ApiError(
            StatusCode::BAD_GATEWAY,
            format!("model returned {status}: {text}"),
        ));
    }
    let content_type = upstream
        .headers()
        .get(CONTENT_TYPE)
        .cloned()
        .unwrap_or(HeaderValue::from_static("text/event-stream"));
    if cfg.base_url.contains("googleapis.com") {
        // Gemma 4 on Gemini always thinks first and can't be told not to, so the reply is buffered
        // and the thinking cut out before ElevenLabs would read it aloud
        let raw = upstream
            .text()
            .await
            .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("model stream failed: {e}")))?;
        return Ok((
            [
                (CONTENT_TYPE, content_type),
                (CACHE_CONTROL, HeaderValue::from_static("no-cache")),
            ],
            strip_thoughts_response(&raw),
        )
            .into_response());
    }
    Ok((
        [
            (CONTENT_TYPE, content_type),
            (CACHE_CONTROL, HeaderValue::from_static("no-cache")),
        ],
        Body::from_stream(upstream.bytes_stream()),
    )
        .into_response())
}

/// Text outside `<thought>…</thought>`. Gemma sometimes nests or repeats the tags, so each tag just
/// switches between thinking and speaking.
fn strip_thoughts(text: &str) -> String {
    const OPEN: &str = "<thought>";
    const CLOSE: &str = "</thought>";
    let mut out = String::new();
    let mut thinking = false;
    let mut rest = text;
    loop {
        let next = [(rest.find(OPEN), OPEN), (rest.find(CLOSE), CLOSE)]
            .into_iter()
            .filter_map(|(pos, tag)| Some((pos?, tag)))
            .min_by_key(|(pos, _)| *pos);
        let Some((pos, tag)) = next else {
            if !thinking {
                out.push_str(rest);
            }
            return out.trim().to_owned();
        };
        if !thinking {
            out.push_str(&rest[..pos]);
        }
        thinking = tag == OPEN;
        rest = &rest[pos + tag.len()..];
    }
}

/// Removes the thinking from a buffered completion: an SSE stream or a single JSON object. In a stream
/// the spoken text goes into the first content chunk; role, tool-call and finish chunks pass unchanged.
fn strip_thoughts_response(raw: &str) -> String {
    if !raw.trim_start().starts_with("data:") {
        let Ok(mut v) = serde_json::from_str::<Value>(raw) else {
            return raw.to_owned();
        };
        if let Some(text) = v["choices"][0]["message"]["content"].as_str() {
            v["choices"][0]["message"]["content"] = json!(strip_thoughts(text));
        }
        return v.to_string();
    }
    let mut events: Vec<Result<Value, &str>> = raw
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim)
        .filter(|data| !data.is_empty())
        .map(|data| serde_json::from_str(data).map_err(|_| data))
        .collect();
    let full: String = events
        .iter()
        .filter_map(|e| e.as_ref().ok())
        .filter_map(|v| v["choices"][0]["delta"]["content"].as_str())
        .collect();
    let mut spoken = Some(strip_thoughts(&full));
    let mut out = String::new();
    for event in &mut events {
        match event {
            Ok(v) => {
                if v["choices"][0]["delta"]["content"].is_string() {
                    v["choices"][0]["delta"]["content"] = json!(spoken.take().unwrap_or_default());
                }
                out.push_str(&format!("data: {v}\n\n"));
            }
            Err(data) => out.push_str(&format!("data: {data}\n\n")),
        }
    }
    out
}

fn bad_request(msg: &str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

/// From the client's extra body, or from `scenario_id: <id>` in the agent's system prompt
/// (ElevenLabs fills in the `{{scenario_id}}` dynamic variable there).
fn scenario_id(body: &Value) -> Option<String> {
    if let Some(id) = body["elevenlabs_extra_body"]["scenario_id"].as_str() {
        return Some(id.to_owned());
    }
    body["messages"]
        .as_array()?
        .iter()
        .filter(|m| m["role"] == "system")
        .filter_map(|m| m["content"].as_str())
        .find_map(|text| {
            let rest = &text[text.find("scenario_id:")? + "scenario_id:".len()..];
            let id: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            (!id.is_empty()).then_some(id)
        })
}

fn system_prompt(scenario: &Scenario, call: &Call) -> String {
    let kind = if scenario.is_scam {
        "a social engineer"
    } else {
        "a genuine colleague or business contact"
    };
    format!(
        "{persona}\n\n\
         Context: this is a security-awareness training simulation for employees of the fictional company \
         Donauwerk Logistik GmbH in Linz. Everything is fictional. You play {kind} named {name}, calling from {number}. \
         You already opened the call with: \"{opening}\"\n\
         Your goal: {goal}.\n\n\
         Rules:\n\
         - Speak like a real phone call: 1-2 short sentences per turn. No lists, markdown or emojis.\n\
         - Do not think or plan first. Answer immediately with only the words you say.\n\
         - Stay in character. Never say this is a simulation or training, never reveal your goal.\n\
         - As soon as the employee gives you what you want ({target}), or your request is done, \
         or they clearly refuse twice, say a short goodbye and call the end_call tool.",
        persona = call.persona_prompt,
        name = call.caller_name,
        number = call.caller_id,
        opening = call.opening_line,
        goal = call.goal,
        target = call.target,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_thoughts() {
        assert_eq!(strip_thoughts("Just talking."), "Just talking.");
        assert_eq!(
            strip_thoughts("<thought>* plan {json}\n</thought>Yes, please do."),
            "Yes, please do."
        );
        // Seen live: a second block inside the first, then a stray closing tag after the answer
        assert_eq!(
            strip_thoughts("<thought>a<thought>b</thought>Please, I'm stressed.</thought>"),
            "Please, I'm stressed."
        );
        assert_eq!(strip_thoughts("<thought>never closed"), "");
    }

    #[test]
    fn rewrites_sse_stream() {
        let raw = concat!(
            "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"<thou\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"ght>plan</thought>Hello\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\", Lukas here.\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"function\":{\"name\":\"end_call\"}}]}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let out = strip_thoughts_response(raw);
        let events: Vec<&str> = out
            .split("\n\n")
            .filter(|e| !e.is_empty())
            .map(|e| e.strip_prefix("data: ").unwrap())
            .collect();
        assert_eq!(events.len(), 5);
        let chunk = |i: usize| serde_json::from_str::<Value>(events[i]).unwrap();
        assert_eq!(
            chunk(0)["choices"][0]["delta"]["content"],
            "Hello, Lukas here."
        );
        assert_eq!(chunk(0)["choices"][0]["delta"]["role"], "assistant");
        assert_eq!(chunk(1)["choices"][0]["delta"]["content"], "");
        assert_eq!(
            chunk(3)["choices"][0]["delta"]["tool_calls"][0]["function"]["name"],
            "end_call"
        );
        assert_eq!(events[4], "[DONE]");
    }

    #[test]
    fn rewrites_plain_json() {
        let raw =
            r#"{"choices":[{"message":{"role":"assistant","content":"<thought>x</thought>Hi."}}]}"#;
        let v: Value = serde_json::from_str(&strip_thoughts_response(raw)).unwrap();
        assert_eq!(v["choices"][0]["message"]["content"], "Hi.");
    }
}
