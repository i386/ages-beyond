#![cfg_attr(not(windows), allow(dead_code))]

use ages_beyond_protocol::{GameEvent, RequestBody};
use anyhow::Context;
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::chronicle::{ChronicleWrite, ChronicleWriter};
use crate::director::DirectorState;
use crate::llm::LlmClient;
use crate::notifications::NotificationWriter;

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
    notifications: Option<&NotificationWriter>,
    director: &Mutex<DirectorState>,
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

            let arc_request = {
                let mut director = director.lock().await;
                director.observe_event(event)
            };

            if let Some(request) = arc_request {
                let title = match llm
                    .respond(&RequestBody::WorldArcTitle {
                        request: request.clone(),
                    })
                    .await
                {
                    Ok(title) => title,
                    Err(err) => {
                        warn!(
                            listener = listener,
                            event_type = %event.event_type,
                            error = %err,
                            "using fallback world arc title"
                        );
                        request.fallback_title.clone()
                    }
                };

                let mut director = director.lock().await;
                director.apply_world_arc_title(&request, title);
            }

            let event_for_prompt = {
                let director = director.lock().await;
                director.enrich_event(event)
            };

            let text = llm
                .respond(&RequestBody::GameEvent {
                    event: event_for_prompt,
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
                            "skipped duplicate chronicle projection, keeping session notification"
                        );
                    }
                }
            }

            if let Some(writer) = notifications {
                let notification = notification_excerpt(&text);
                writer.append_event(event, &notification).await?;
            }

            Ok(text)
        }
    }
}

fn classify_event(event: &GameEvent) -> EventHandling {
    if let Some(reason) = audience_visibility_reason(event) {
        return EventHandling::Ignore {
            listener: "audience",
            reason,
        };
    }

    match event.event_type.as_str() {
        "game_started" => chronicle(event, "lifecycle", "Game Started"),
        "city_founded" => chronicle(event, "settlement", "City Founded"),
        "city_acquired" => chronicle(event, "territory", "City Acquired"),
        "city_captured" => chronicle(event, "territory", "City Captured"),
        "city_razed" => chronicle(event, "territory", "City Razed"),
        "religion_founded" => chronicle(event, "faith", "Religion Founded"),
        "tech_discovered" => chronicle(event, "knowledge", "Technology Discovered"),
        "wonder_built" => chronicle(event, "achievement", "World Wonder Built"),
        "project_built" => chronicle(event, "achievement", "Great Project Built"),
        "golden_age_started" => chronicle(event, "society", "Golden Age Started"),
        "great_person_born" => chronicle(event, "personage", "Great Person Born"),
        "quest_started" => chronicle(event, "quest", "Quest Started"),
        "event_triggered" => chronicle(event, "event", "Event Triggered"),
        "victory" => chronicle(event, "finale", "Victory"),
        "war_declared" => classify_diplomacy(event, "War Declared"),
        "peace_signed" => classify_diplomacy(event, "Peace Signed"),
        _ => chronicle(event, "generic", title_case_event_type(&event.event_type)),
    }
}

fn classify_diplomacy(event: &GameEvent, heading: &'static str) -> EventHandling {
    if let Some(reason) = internal_diplomacy_reason(event) {
        return EventHandling::Ignore {
            listener: "diplomacy",
            reason,
        };
    }

    chronicle(event, "diplomacy", heading)
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

fn audience_visibility_reason(event: &GameEvent) -> Option<String> {
    if fact_i64(event, "contract_version").unwrap_or_default() < 3 {
        return None;
    }

    if fact_bool(event, "known_to_active_player") == Some(false) {
        return Some("event is not known to the active player".to_owned());
    }

    let is_global = fact_bool(event, "is_global_announcement").unwrap_or(false);
    let involves_active_player = fact_bool(event, "involves_active_player").unwrap_or(false);
    let involves_active_team = fact_bool(event, "involves_active_team").unwrap_or(false);

    if !is_global && !involves_active_player && !involves_active_team {
        if fact_str(event, "plot_visibility") == Some("hidden") {
            return Some("event location is hidden by fog of war".to_owned());
        }
    }

    None
}

fn chronicle(
    _event: &GameEvent,
    listener: &'static str,
    heading: impl Into<String>,
) -> EventHandling {
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

fn fact_bool(event: &GameEvent, key: &str) -> Option<bool> {
    match event.facts.get(key) {
        Some(Value::Bool(value)) => Some(*value),
        Some(Value::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn fact_str<'a>(event: &'a GameEvent, key: &str) -> Option<&'a str> {
    match event.facts.get(key) {
        Some(Value::String(value)) => Some(value.as_str()),
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

fn notification_excerpt(text: &str) -> String {
    text.lines()
        .map(strip_experiment_label)
        .find(|line| !line.trim().is_empty())
        .unwrap_or_else(|| text.trim().to_owned())
}

fn strip_experiment_label(line: &str) -> String {
    for label in [
        "Chronicle:",
        "Council:",
        "Quest Hook:",
        "World Arc:",
        "Omen:",
    ] {
        if let Some(rest) = line.strip_prefix(label) {
            return rest.trim().to_owned();
        }
    }

    line.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ages_beyond_protocol::GameEvent;
    use serde_json::json;

    use super::{classify_event, notification_excerpt, EventHandling};

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

    #[test]
    fn classifies_city_capture_as_territory() {
        let facts = BTreeMap::new();

        assert!(matches!(
            classify_event(&event("city_captured", facts)),
            EventHandling::Chronicle {
                listener: "territory",
                ..
            }
        ));
    }

    #[test]
    fn notification_uses_first_rendered_line_without_label() {
        assert_eq!(
            notification_excerpt(
                "Chronicle: The city rose beside the river.\nCouncil: Let its walls speak first."
            ),
            "The city rose beside the river."
        );
    }
}
