use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionRequest {
    pub version: u16,
    pub id: String,
    #[serde(flatten)]
    pub body: RequestBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RequestBody {
    Ping,
    GameEvent { event: GameEvent },
    DiplomacyText { request: DiplomacyTextRequest },
    WorldArcTitle { request: WorldArcRequest },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_type: String,
    pub turn: Option<i32>,
    #[serde(default)]
    pub actors: Vec<EventActor>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub facts: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventActor {
    pub role: String,
    pub name: String,
    #[serde(default)]
    pub civilization: Option<String>,
    #[serde(default)]
    pub leader: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomacyTextRequest {
    pub comment_type: String,
    pub active_player_id: i32,
    pub leader_player_id: i32,
    pub turn: Option<i32>,
    #[serde(default)]
    pub active_player_name: Option<String>,
    #[serde(default)]
    pub active_civilization: Option<String>,
    #[serde(default)]
    pub leader_name: Option<String>,
    #[serde(default)]
    pub leader_civilization: Option<String>,
    #[serde(default)]
    pub attitude: Option<String>,
    #[serde(default)]
    pub at_war: bool,
    #[serde(default)]
    pub power_relation: Option<String>,
    #[serde(default)]
    pub fallback_text: Option<String>,
    #[serde(default)]
    pub diplomacy_memory: Option<String>,
    #[serde(default)]
    pub world_arc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldArcRequest {
    pub trigger_event_type: String,
    pub turn: Option<i32>,
    pub fallback_title: String,
    pub theme: String,
    pub pressure: i32,
    #[serde(default)]
    pub involved_civilizations: Vec<String>,
    #[serde(default)]
    pub notable_places: Vec<String>,
    #[serde(default)]
    pub notable_terms: Vec<String>,
    #[serde(default)]
    pub recent_events: Vec<String>,
    #[serde(default)]
    pub current_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionResponse {
    pub version: u16,
    pub id: String,
    pub status: ResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Ok,
    Error,
}

impl CompanionResponse {
    pub fn ok(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            id: id.into(),
            status: ResponseStatus::Ok,
            text: Some(text.into()),
            error: None,
        }
    }

    pub fn error(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            id: id.into(),
            status: ResponseStatus::Error,
            text: None,
            error: Some(error.into()),
        }
    }
}
