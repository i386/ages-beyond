#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::BTreeMap;

use ages_beyond_protocol::{GameEvent, RequestBody};
use anyhow::Context;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::chronicle::{ChronicleWrite, ChronicleWriter};
use crate::director::DirectorState;
use crate::llm::LlmClient;
use crate::memory::{
    MemoryWriter, QuestDecisionResponseReader, QuestJournalWriter, QuestLogWriter,
};
use crate::notifications::{NotificationWriter, QuestDecisionWriter};
use crate::save_state::AgesBeyondSaveState;

#[derive(Debug, Clone)]
enum EventHandling {
    Chronicle {
        listener: &'static str,
        heading: String,
    },
    Rumor {
        listener: &'static str,
        heading: String,
        event: GameEvent,
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
    quest_notifications: Option<&NotificationWriter>,
    quest_decisions: Option<&QuestDecisionWriter>,
    memory: Option<&MemoryWriter>,
    quest_log: Option<&QuestLogWriter>,
    quest_journal: Option<&QuestJournalWriter>,
    director: &Mutex<DirectorState>,
    save_state: &mut AgesBeyondSaveState,
) -> anyhow::Result<bool>
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
            Ok(false)
        }
        EventHandling::Rumor {
            listener,
            heading,
            event: rumor_event,
        } => {
            if save_state.is_event_seen(&rumor_event) {
                debug!(
                    listener = listener,
                    event_type = %rumor_event.event_type,
                    event_id = ?event_id(&rumor_event),
                    "skipped duplicate rumor event from save state"
                );
                return Ok(false);
            }

            info!(
                listener = listener,
                source_event_type = %event.event_type,
                "projecting hidden event as rumor"
            );

            let rumor_event_for_prompt = {
                let director = director.lock().await;
                director.enrich_event(&rumor_event)
            };

            let text = llm
                .respond(&RequestBody::GameEvent {
                    event: rumor_event_for_prompt,
                })
                .await
                .with_context(|| format!("failed to render {listener} rumor"))?;

            if let Some(writer) = chronicle {
                match writer.append_event(&rumor_event, &heading, &text).await? {
                    ChronicleWrite::Appended => {}
                    ChronicleWrite::DuplicateSkipped => {
                        info!(
                            listener = listener,
                            event_type = %rumor_event.event_type,
                            event_id = ?event_id(&rumor_event),
                            "skipped duplicate rumor projection, keeping session notification"
                        );
                    }
                }
            }

            if let Some(writer) = notifications {
                let notification = notification_excerpt(&text);
                writer.append_event(&rumor_event, &notification).await?;
            }

            Ok(save_state.mark_event_seen(&rumor_event))
        }
        EventHandling::Chronicle { listener, heading } => {
            if save_state.is_event_seen(event) {
                debug!(
                    listener = listener,
                    event_type = %event.event_type,
                    event_id = ?event_id(event),
                    "skipped duplicate game event from save state"
                );
                return Ok(false);
            }

            debug!(
                listener = listener,
                event_type = %event.event_type,
                "handling game event"
            );

            let observation = {
                let mut director = director.lock().await;
                director.observe_event(event)
            };

            for proposal in observation.historical_names().iter().cloned() {
                let request = proposal.request().clone();
                let title = match llm
                    .respond(&RequestBody::HistoricalName {
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
                            "using fallback historical name"
                        );
                        request.fallback_title.clone()
                    }
                };

                let mut director = director.lock().await;
                director.apply_historical_name(&proposal, title);
            }

            if let Some(request) = observation.world_arc().cloned() {
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

            let era_transition = observation.era_transition().cloned();

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
                    ChronicleWrite::Appended => {
                        writer
                            .append_memory_projection(event, observation.memory_projections())
                            .await?;
                        writer
                            .append_quest_projection(event, observation.quest_projections())
                            .await?;
                    }
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

            if let Some(writer) = quest_notifications {
                for quest in observation.quest_projections() {
                    writer.append_event(event, quest).await?;
                }
            }

            if let Some(writer) = quest_decisions {
                for decision in observation.quest_decisions() {
                    writer.append_decision(decision).await?;
                }
            }

            if let Some(era_event) = era_transition {
                let era_event_for_prompt = {
                    let director = director.lock().await;
                    director.enrich_event(&era_event)
                };

                let era_text = llm
                    .respond(&RequestBody::GameEvent {
                        event: era_event_for_prompt,
                    })
                    .await
                    .with_context(|| format!("failed to render {listener} era transition"))?;

                if let Some(writer) = chronicle {
                    match writer
                        .append_event(&era_event, "Era Transition", &era_text)
                        .await?
                    {
                        ChronicleWrite::Appended => {}
                        ChronicleWrite::DuplicateSkipped => {
                            info!(
                                listener = listener,
                                event_type = %era_event.event_type,
                                event_id = ?event_id(&era_event),
                                "skipped duplicate era transition projection, keeping session notification"
                            );
                        }
                    }
                }

                if let Some(writer) = notifications {
                    let notification = notification_excerpt(&era_text);
                    writer.append_event(&era_event, &notification).await?;
                }
            }

            write_director_outputs(memory, quest_log, quest_journal, director).await?;

            let mut changed = save_state.mark_event_seen(event);
            changed |= save_state.record_pending_decisions(observation.quest_decisions());
            changed |= save_state.record_pending_rewards(observation.quest_rewards());

            Ok(changed)
        }
    }
}

pub async fn apply_quest_decision_responses(
    quest_decision_responses: Option<&QuestDecisionResponseReader>,
    memory: Option<&MemoryWriter>,
    quest_log: Option<&QuestLogWriter>,
    quest_journal: Option<&QuestJournalWriter>,
    director: &Mutex<DirectorState>,
    save_state: &mut AgesBeyondSaveState,
) -> anyhow::Result<(Vec<String>, bool)> {
    let Some(reader) = quest_decision_responses else {
        return Ok((Vec::new(), false));
    };

    let responses = reader.read_new().await?;
    if responses.is_empty() {
        return Ok((Vec::new(), false));
    }
    let (responses, save_changed) = save_state.apply_decision_responses(&responses);
    if responses.is_empty() {
        return Ok((Vec::new(), save_changed));
    }

    let projections = {
        let mut director = director.lock().await;
        director.apply_quest_decision_responses(&responses)
    };

    if !projections.is_empty() {
        write_director_outputs(memory, quest_log, quest_journal, director).await?;
    }

    Ok((projections, save_changed))
}

pub(crate) async fn write_director_outputs(
    memory: Option<&MemoryWriter>,
    quest_log: Option<&QuestLogWriter>,
    quest_journal: Option<&QuestJournalWriter>,
    director: &Mutex<DirectorState>,
) -> anyhow::Result<()> {
    if memory.is_none() && quest_log.is_none() && quest_journal.is_none() {
        return Ok(());
    }

    let snapshot = {
        let director = director.lock().await;
        director.memory_snapshot()
    };

    if let Some(writer) = memory {
        writer.write_snapshot(&snapshot).await?;
    }

    if let Some(writer) = quest_log {
        writer.write_snapshot(&snapshot).await?;
    }

    if let Some(writer) = quest_journal {
        writer.write_snapshot(&snapshot).await?;
    }

    Ok(())
}

fn classify_event(event: &GameEvent) -> EventHandling {
    if let Some(reason) = audience_visibility_reason(event) {
        if let Some(rumor_event) = rumor_event(event, &reason) {
            return EventHandling::Rumor {
                listener: "rumor",
                heading: "Rumor".to_owned(),
                event: rumor_event,
            };
        }

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
        "era_transition" => chronicle(event, "era", "Era Transition"),
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

fn rumor_event(source: &GameEvent, visibility_reason: &str) -> Option<GameEvent> {
    if fact_bool(source, "rumor_possible") != Some(true) {
        return None;
    }

    let channel = fact_str(source, "rumor_channel")?;
    let rumor_subject = rumor_subject(source.event_type.as_str())?;
    let mut facts = BTreeMap::new();

    facts.insert("event_id".to_owned(), json!(rumor_event_id(source)));
    facts.insert("internal_event".to_owned(), json!(true));
    facts.insert("contract_version".to_owned(), json!(3));
    facts.insert("importance".to_owned(), json!("minor"));
    facts.insert("chapter".to_owned(), json!("Rumors"));
    facts.insert("story_arc".to_owned(), json!("rumors"));
    facts.insert("audience".to_owned(), json!("active_player"));
    facts.insert("visibility_scope".to_owned(), json!("rumor"));
    facts.insert("known_to_active_player".to_owned(), json!(true));
    facts.insert("location_known_to_active_player".to_owned(), json!(false));
    facts.insert("plot_visibility".to_owned(), json!("rumor"));
    facts.insert("rumor".to_owned(), json!(true));
    facts.insert("rumor_channel".to_owned(), json!(channel));
    facts.insert("rumor_subject".to_owned(), json!(rumor_subject));
    facts.insert(
        "rumor_visibility_reason".to_owned(),
        json!(visibility_reason),
    );
    facts.insert(
        "source_event_type".to_owned(),
        json!(source.event_type.clone()),
    );

    Some(GameEvent {
        event_type: "rumor".to_owned(),
        turn: source.turn,
        actors: Vec::new(),
        summary: Some(rumor_summary(rumor_subject, channel)),
        facts,
    })
}

fn rumor_subject(event_type: &str) -> Option<&'static str> {
    match event_type {
        "city_founded" => Some("distant settlement"),
        "city_captured" | "city_acquired" => Some("distant banners changing"),
        "city_razed" => Some("distant devastation"),
        "great_person_born" => Some("a distant notable life"),
        "golden_age_started" => Some("distant flourishing"),
        _ => None,
    }
}

fn rumor_summary(subject: &str, channel: &str) -> String {
    format!("{channel} carry uncertain word of {subject}.")
}

fn rumor_event_id(source: &GameEvent) -> String {
    match source.facts.get("event_id") {
        Some(Value::Number(value)) => format!("rumor:{value}"),
        Some(Value::String(value)) => format!("rumor:{value}"),
        Some(value) => format!("rumor:{value}"),
        None => format!(
            "rumor:{}:{}",
            source.event_type,
            source.turn.unwrap_or_default()
        ),
    }
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
    fn hidden_event_without_rumor_channel_is_ignored() {
        let facts = BTreeMap::from([
            ("contract_version".to_owned(), json!(3)),
            ("known_to_active_player".to_owned(), json!(false)),
            ("rumor_possible".to_owned(), json!(false)),
        ]);

        assert!(matches!(
            classify_event(&event("city_founded", facts)),
            EventHandling::Ignore {
                listener: "audience",
                ..
            }
        ));
    }

    #[test]
    fn hidden_rumor_event_is_sanitized() {
        let facts = BTreeMap::from([
            ("contract_version".to_owned(), json!(3)),
            ("known_to_active_player".to_owned(), json!(false)),
            ("rumor_possible".to_owned(), json!(true)),
            ("rumor_channel".to_owned(), json!("travellers")),
            ("event_id".to_owned(), json!(42)),
            ("x".to_owned(), json!(10)),
            ("y".to_owned(), json!(12)),
            ("city_name".to_owned(), json!("Hiddenburg")),
            ("founder_civilization".to_owned(), json!("Rome")),
        ]);

        let EventHandling::Rumor { event, .. } = classify_event(&event("city_founded", facts))
        else {
            panic!("expected hidden city founding to become a rumor");
        };

        assert_eq!(event.event_type, "rumor");
        assert_eq!(
            event.facts.get("event_id").and_then(|value| value.as_str()),
            Some("rumor:42")
        );
        assert_eq!(
            event
                .facts
                .get("rumor_subject")
                .and_then(|value| value.as_str()),
            Some("distant settlement")
        );
        assert!(!event.facts.contains_key("x"));
        assert!(!event.facts.contains_key("y"));
        assert!(!event.facts.contains_key("city_name"));
        assert!(!event.facts.contains_key("founder_civilization"));
        assert!(event.summary.as_deref().unwrap().contains("uncertain word"));
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
