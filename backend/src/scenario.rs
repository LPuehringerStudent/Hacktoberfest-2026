//! Scenario types: the contract from docs/PLAN.md. Gemma output is parsed into these.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Email,
    Call,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Comply,
    Report,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedFlagType {
    Urgency,
    Authority,
    CredentialRequest,
    PaymentChange,
    SenderMismatch,
    LinkMismatch,
    RiskyAttachment,
    Secrecy,
    ProcessBypass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlag {
    #[serde(rename = "type")]
    pub kind: RedFlagType,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub id: String,
    pub channel: Channel,
    pub is_scam: bool,
    pub red_flags: Vec<RedFlag>,
    #[serde(default)]
    pub safe_signals: Vec<String>,
    pub correct_action: Action,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<Email>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<Call>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Email {
    pub from_name: String,
    pub from_address: String,
    pub subject: String,
    pub body_markdown: String,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: String,
    pub shown_href: String,
    pub real_href: String,
    pub landing: Landing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Landing {
    /// m365_login | intranet_page | invoice_portal | parcel_tracking
    pub template: String,
    pub brand: String,
    pub trap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub filename: String,
    /// pdf | docx_macro | double_extension | html_login | zip_password
    pub kind: String,
    pub preview: Preview,
    pub trap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preview {
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Call {
    pub caller_name: String,
    pub caller_id: String,
    pub goal: String,
    pub target: String,
    pub opening_line: String,
    pub voice_id: String,
    pub persona_prompt: String,
}

/// Fields that would give the answer away. Stripped (at any depth) before a scenario reaches the client.
const SECRET_FIELDS: &[&str] = &[
    "isScam",
    "redFlags",
    "safeSignals",
    "correctAction",
    "trap",
    "goal",
    "target",
    "personaPrompt",
];

impl Scenario {
    pub fn public(&self) -> Value {
        let mut v = serde_json::to_value(self).expect("scenario serializes");
        strip(&mut v);
        v
    }
}

fn strip(v: &mut Value) {
    match v {
        Value::Object(map) => {
            map.retain(|k, _| !SECRET_FIELDS.contains(&k.as_str()));
            map.values_mut().for_each(strip);
        }
        Value::Array(items) => items.iter_mut().for_each(strip),
        _ => {}
    }
}
