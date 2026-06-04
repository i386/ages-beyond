#![cfg_attr(not(windows), allow(dead_code))]

use ages_beyond_protocol::{GameEvent, RequestBody};
use anyhow::Context;
use serde_json::Value;
use tracing::{debug, info};

use crate::chronicle::{ChronicleWrite, ChronicleWriter};
use crate::llm::LlmClient;

#[derive(Debug, Clone, PartialEq, Eq)]
enum EventHandling {
    Chronicle {
        listener: &'static str,
        heading: String,
    },
    Ignore {
        listener: &'static str,
        reason: String,
    },
}

pub async fn process_game_event<L>(
    event: &GameEvent,
    llm: &L,
    chronicle: Option<&ChronicleWriter>,
) -> anyhow::Result<String>
where
    L: LlmClient,
{
    match classify_event(event) {
        EventHandling::Ignore { listener, reason } => {
            info!(
                listener = listener,
                event_type = %event.event_type,
                reason = %reason,
                "ignored game event"
            );
            Ok(format!("ignored {listener} event: {reason}"))
        }
        EventHandling::Chronicle { listener, heading } => {
            debug!(
                listener = listener,
                event_type = %event.event_type,
                "handling game event"
            );

            let text = llm
                .respond(&RequestBody::GameEvent {
                    event: event.clone(),
                })
                .await
                .with_context(|| format!("failed to render {listener} event"))?;

            if let Some(writer) = chronicle {
                match writer.append_event(event, &heading, &text).await? {
                    ChronicleWrite::Appended => {}
                    ChronicleWrite::DuplicateSkipped => {
                        info!(
                            listener = listener,
                            event_type = %event.event_type,
                            event_id = ?event_id(event),
                            "skipped duplicate chronicle projection"
                        );
                    }
                }
            }

            Ok(text)
        }
    }
}

fn classify_event(event: &GameEvent) -> EventHandling {
    match event.event_type.as_str() {
        "game_started" => chronicle("lifecycle", "Game Started"),
        "city_founded" => chronicle("settlement", "City Founded"),
        "religion_founded" => chronicle("faith", "Religion Founded"),
        "tech_discovered" => chronicle("knowledge", "Technology Discovered"),
        "wonder_built" => chronicle("achievement", "World Wonder Built"),
        "war_declared" => classify_diplomacy(event, "War Declared"),
        "peace_signed" => classify_diplomacy(event, "Peace Signed"),
        _ => chronicle("generic", title_case_event_type(&event.event_type)),
    }
}

fn classify_diplomacy(event: &GameEvent, heading: &'static str) -> EventHandling {
    if let Some(reason) = internal_diplomacy_reason(event) {
        return EventHandling::Ignore {
            listener: "diplomacy",
            reason,
        };
    }

    chronicle("diplomacy", heading)
}

fn internal_diplomacy_reason(event: &GameEvent) -> Option<String> {
    let primary_team_id = fact_i64(event, "team_id")?;
    let secondary_team_id = fact_i64(event, "data1")?;

    if primary_team_id < 0 || secondary_team_id < 0 {
        return Some("missing diplomacy teams".to_owned());
    }

    if let Some(barbarian_team_id) = fact_i64(event, "barbarian_team_id") {
        if primary_team_id == barbarian_team_id || secondary_team_id == barbarian_team_id {
            return Some("barbarian team diplomacy is engine state".to_owned());
        }
    }

    if let Some(max_civ_players) = fact_i64(event, "max_civ_players") {
        if primary_team_id >= max_civ_players || secondary_team_id >= max_civ_players {
            return Some("non-civilization team diplomacy is engine state".to_owned());
        }
    }

    None
}

fn chronicle(listener: &'static str, heading: impl Into<String>) -> EventHandling {
    EventHandling::Chronicle {
        listener,
        heading: heading.into(),
    }
}

fn event_id(event: &GameEvent) -> Option<i64> {
    fact_i64(event, "event_id")
}

fn fact_i64(event: &GameEvent, key: &str) -> Option<i64> {
    match event.facts.get(key) {
        Some(Value::Number(value)) => value.as_i64(),
        Some(Value::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn title_case_event_type(event_type: &str) -> String {
    event_type
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ages_beyond_protocol::GameEvent;
    use serde_json::json;

    use super::{classify_event, EventHandling};

    fn event(event_type: &str, facts: BTreeMap<String, serde_json::Value>) -> GameEvent {
        GameEvent {
            event_type: event_type.to_owned(),
            turn: Some(0),
            actors: Vec::new(),
            summary: None,
            facts,
        }
    }

    #[test]
    fn ignores_barbarian_setup_war() {
        let facts = BTreeMap::from([
            ("team_id".to_owned(), json!(18)),
            ("data1".to_owned(), json!(0)),
            ("max_civ_players".to_owned(), json!(18)),
            ("barbarian_team_id".to_owned(), json!(18)),
        ]);

        assert!(matches!(
            classify_event(&event("war_declared", facts)),
            EventHandling::Ignore { .. }
        ));
    }

    #[test]
    fn chronicles_major_team_war() {
        let facts = BTreeMap::from([
            ("team_id".to_owned(), json!(0)),
            ("data1".to_owned(), json!(1)),
            ("max_civ_players".to_owned(), json!(18)),
            ("barbarian_team_id".to_owned(), json!(18)),
        ]);

        assert!(matches!(
            classify_event(&event("war_declared", facts)),
            EventHandling::Chronicle {
                listener: "diplomacy",
                ..
            }
        ));
    }
}
