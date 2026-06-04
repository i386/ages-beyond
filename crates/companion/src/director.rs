#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::{HashMap, HashSet, VecDeque};

use ages_beyond_protocol::{DiplomacyTextRequest, GameEvent, WorldArcRequest};
use serde_json::{json, Value};

const MAX_RELATIONSHIP_MEMORIES: usize = 8;
const MAX_RECENT_WORLD_EVENTS: usize = 12;

#[derive(Debug, Clone, Default)]
pub struct DirectorState {
    relationship_memories: HashMap<RelationshipKey, VecDeque<String>>,
    recent_world_events: VecDeque<String>,
    world_arc: Option<WorldArc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RelationshipKey {
    first_player: i32,
    second_player: i32,
}

#[derive(Debug, Clone)]
struct WorldArc {
    title: String,
    theme: String,
    pressure: i32,
}

impl DirectorState {
    pub fn observe_event(&mut self, event: &GameEvent) -> Option<WorldArcRequest> {
        if let Some(memory) = relationship_memory(event) {
            self.remember_relationship(memory.first_player, memory.second_player, memory.text);
        }

        if let Some(summary) = world_event_summary(event) {
            self.remember_world_event(summary);
        }

        self.propose_world_arc(event)
    }

    pub fn apply_world_arc_title(&mut self, request: &WorldArcRequest, title: String) {
        let title = if title.trim().is_empty() {
            request.fallback_title.clone()
        } else {
            title.trim().to_owned()
        };

        match &mut self.world_arc {
            Some(arc) if arc.title == title => {
                arc.pressure = (arc.pressure + 1).min(6);
                arc.theme = request.theme.clone();
            }
            _ => {
                self.world_arc = Some(WorldArc {
                    title,
                    theme: request.theme.clone(),
                    pressure: request.pressure,
                });
            }
        }
    }

    pub fn enrich_event(&self, event: &GameEvent) -> GameEvent {
        let mut enriched = event.clone();

        if let Some(arc) = &self.world_arc {
            enriched
                .facts
                .insert("world_arc_title".to_owned(), json!(arc.title));
            enriched
                .facts
                .insert("world_arc_theme".to_owned(), json!(arc.theme));
            enriched
                .facts
                .insert("world_arc_pressure".to_owned(), json!(arc.pressure));
        }

        let recent = self.recent_world_summary();
        if !recent.is_empty() {
            enriched
                .facts
                .insert("recent_world_events".to_owned(), json!(recent));
        }

        if let Some(memory) = event_relationship_memory(event, self) {
            enriched
                .facts
                .insert("diplomacy_memory".to_owned(), json!(memory));
        }

        enriched
    }

    pub fn enrich_diplomacy_request(&self, request: &DiplomacyTextRequest) -> DiplomacyTextRequest {
        let mut enriched = request.clone();
        enriched.diplomacy_memory =
            self.relationship_summary(request.active_player_id, request.leader_player_id);
        enriched.world_arc = self.world_arc_summary();
        enriched
    }

    fn remember_relationship(&mut self, first_player: i32, second_player: i32, text: String) {
        if first_player < 0 || second_player < 0 || first_player == second_player {
            return;
        }

        let key = RelationshipKey::new(first_player, second_player);
        let memories = self.relationship_memories.entry(key).or_default();
        if memories.iter().any(|memory| memory == &text) {
            return;
        }
        memories.push_back(text);
        while memories.len() > MAX_RELATIONSHIP_MEMORIES {
            memories.pop_front();
        }
    }

    fn remember_world_event(&mut self, text: String) {
        if self
            .recent_world_events
            .back()
            .is_some_and(|recent| recent == &text)
        {
            return;
        }

        self.recent_world_events.push_back(text);
        while self.recent_world_events.len() > MAX_RECENT_WORLD_EVENTS {
            self.recent_world_events.pop_front();
        }
    }

    fn propose_world_arc(&mut self, event: &GameEvent) -> Option<WorldArcRequest> {
        let (fallback_title, theme, pressure) = match event.event_type.as_str() {
            "war_declared" => (
                war_fallback_title(event),
                "escalating diplomacy and war".to_owned(),
                4,
            ),
            "peace_signed" => (
                peace_fallback_title(event),
                "settlement after conflict".to_owned(),
                2,
            ),
            "religion_founded" => (
                religion_fallback_title(event),
                "faith shaping politics".to_owned(),
                3,
            ),
            "wonder_built" => (
                wonder_fallback_title(event),
                "prestige and legacy".to_owned(),
                2,
            ),
            "city_razed" => (
                city_fallback_title(event, "Razing"),
                "devastation and reprisal".to_owned(),
                5,
            ),
            "city_captured" => (
                city_fallback_title(event, "Capture"),
                "contested rule and conquest".to_owned(),
                4,
            ),
            "golden_age_started" => (
                player_fallback_title(event, "Golden Age"),
                "prosperity seeking a legacy".to_owned(),
                2,
            ),
            _ => {
                if let Some(arc) = &mut self.world_arc {
                    arc.pressure = (arc.pressure - 1).max(1);
                }
                return None;
            }
        };

        Some(WorldArcRequest {
            trigger_event_type: event.event_type.clone(),
            turn: event.turn,
            fallback_title,
            theme,
            pressure,
            involved_civilizations: involved_civilizations(event),
            notable_places: notable_places(event),
            notable_terms: notable_terms(event),
            recent_events: self.recent_world_events.iter().cloned().collect(),
            current_title: self.world_arc.as_ref().map(|arc| arc.title.clone()),
        })
    }

    pub fn decay_world_arc(&mut self) {
        if let Some(arc) = &mut self.world_arc {
            arc.pressure = (arc.pressure - 1).max(1);
        }
    }

    fn relationship_summary(&self, first_player: i32, second_player: i32) -> Option<String> {
        let memories = self
            .relationship_memories
            .get(&RelationshipKey::new(first_player, second_player))?;

        if memories.is_empty() {
            None
        } else {
            Some(memories.iter().cloned().collect::<Vec<_>>().join(" | "))
        }
    }

    fn recent_world_summary(&self) -> String {
        self.recent_world_events
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn world_arc_summary(&self) -> Option<String> {
        self.world_arc
            .as_ref()
            .map(|arc| format!("{}: {}; pressure {}", arc.title, arc.theme, arc.pressure))
    }
}

impl RelationshipKey {
    fn new(first_player: i32, second_player: i32) -> Self {
        if first_player <= second_player {
            Self {
                first_player,
                second_player,
            }
        } else {
            Self {
                first_player: second_player,
                second_player: first_player,
            }
        }
    }
}

struct RelationshipMemory {
    first_player: i32,
    second_player: i32,
    text: String,
}

fn relationship_memory(event: &GameEvent) -> Option<RelationshipMemory> {
    match event.event_type.as_str() {
        "war_declared" => {
            let first = fact_i32(event, "declaring_team_leader_player_id")
                .or_else(|| fact_i32(event, "player_id"))?;
            let second = fact_i32(event, "target_team_leader_player_id").or_else(|| {
                team_leader_from_prefix(event, "target_team").or_else(|| fact_i32(event, "data1"))
            })?;
            Some(RelationshipMemory {
                first_player: first,
                second_player: second,
                text: format!(
                    "{} declared war on {}.",
                    fact_string(event, "declaring_team_civilization")
                        .or_else(|| fact_string(event, "declaring_team_name"))
                        .unwrap_or_else(|| "One realm".to_owned()),
                    fact_string(event, "target_team_civilization")
                        .or_else(|| fact_string(event, "target_team_name"))
                        .unwrap_or_else(|| "another realm".to_owned())
                ),
            })
        }
        "peace_signed" => {
            let first = team_leader_from_prefix(event, "first_team")
                .or_else(|| fact_i32(event, "player_id"))?;
            let second = team_leader_from_prefix(event, "second_team")
                .or_else(|| fact_i32(event, "data1"))?;
            Some(RelationshipMemory {
                first_player: first,
                second_player: second,
                text: format!(
                    "{} made peace with {}.",
                    fact_string(event, "first_team_civilization")
                        .or_else(|| fact_string(event, "first_team_name"))
                        .unwrap_or_else(|| "One realm".to_owned()),
                    fact_string(event, "second_team_civilization")
                        .or_else(|| fact_string(event, "second_team_name"))
                        .unwrap_or_else(|| "another realm".to_owned())
                ),
            })
        }
        "city_captured" | "city_acquired" => {
            let old_owner = fact_i32(event, "old_owner_id").or_else(|| fact_i32(event, "data1"))?;
            let new_owner =
                fact_i32(event, "new_owner_id").or_else(|| fact_i32(event, "player_id"))?;
            Some(RelationshipMemory {
                first_player: old_owner,
                second_player: new_owner,
                text: format!(
                    "{} came under {} control.",
                    fact_string(event, "city_name").unwrap_or_else(|| "A city".to_owned()),
                    fact_string(event, "new_owner_civilization")
                        .unwrap_or_else(|| "foreign".to_owned())
                ),
            })
        }
        "city_razed" => {
            let old_owner = fact_i32(event, "old_owner_id").or_else(|| fact_i32(event, "data1"))?;
            let razing_player =
                fact_i32(event, "razing_player_id").or_else(|| fact_i32(event, "player_id"))?;
            Some(RelationshipMemory {
                first_player: old_owner,
                second_player: razing_player,
                text: format!(
                    "{} was razed by {}.",
                    fact_string(event, "city_name").unwrap_or_else(|| "A city".to_owned()),
                    fact_string(event, "razing_player_civilization")
                        .unwrap_or_else(|| "a rival".to_owned())
                ),
            })
        }
        _ => None,
    }
}

fn event_relationship_memory(event: &GameEvent, director: &DirectorState) -> Option<String> {
    let player_id = fact_i32(event, "player_id")?;
    let other_id = fact_i32(event, "data1")?;
    director.relationship_summary(player_id, other_id)
}

fn world_event_summary(event: &GameEvent) -> Option<String> {
    match event.event_type.as_str() {
        "game_started" => Some("The world chronicle began.".to_owned()),
        "war_declared" => Some(format!(
            "{} declared war on {}.",
            fact_string(event, "declaring_team_civilization")
                .or_else(|| fact_string(event, "declaring_team_name"))
                .unwrap_or_else(|| "A realm".to_owned()),
            fact_string(event, "target_team_civilization")
                .or_else(|| fact_string(event, "target_team_name"))
                .unwrap_or_else(|| "a rival".to_owned())
        )),
        "peace_signed" => Some(format!(
            "{} made peace with {}.",
            fact_string(event, "first_team_civilization")
                .or_else(|| fact_string(event, "first_team_name"))
                .unwrap_or_else(|| "A realm".to_owned()),
            fact_string(event, "second_team_civilization")
                .or_else(|| fact_string(event, "second_team_name"))
                .unwrap_or_else(|| "a rival".to_owned())
        )),
        "city_founded" | "city_captured" | "city_acquired" | "city_razed" => {
            event.summary.clone().or_else(|| {
                fact_string(event, "city_name").map(|city| format!("{city} changed the map."))
            })
        }
        "religion_founded" => fact_string(event, "religion_name")
            .map(|religion| format!("{religion} entered the world.")),
        "wonder_built" => {
            fact_string(event, "building_name").map(|building| format!("{building} was completed."))
        }
        "project_built" => {
            fact_string(event, "project_name").map(|project| format!("{project} was completed."))
        }
        "golden_age_started" => fact_string(event, "player_civilization")
            .map(|civ| format!("{civ} entered a golden age.")),
        "victory" => fact_string(event, "victory_name")
            .map(|victory| format!("A victory was declared: {victory}.")),
        _ => None,
    }
}

fn war_fallback_title(event: &GameEvent) -> String {
    two_side_fallback_title(
        event,
        "declaring_team_civilization",
        "declaring_team_name",
        "target_team_civilization",
        "target_team_name",
        "War",
    )
}

fn peace_fallback_title(event: &GameEvent) -> String {
    two_side_fallback_title(
        event,
        "first_team_civilization",
        "first_team_name",
        "second_team_civilization",
        "second_team_name",
        "Peace",
    )
}

fn religion_fallback_title(event: &GameEvent) -> String {
    named_fallback_title(event, "religion_name", "Faith")
}

fn wonder_fallback_title(event: &GameEvent) -> String {
    named_fallback_title(event, "building_name", "Wonder")
}

fn city_fallback_title(event: &GameEvent, noun: &str) -> String {
    named_fallback_title(event, "city_name", noun)
}

fn player_fallback_title(event: &GameEvent, noun: &str) -> String {
    fact_string(event, "player_civilization")
        .or_else(|| fact_string(event, "player_name"))
        .map(|name| format!("{name} {noun}"))
        .unwrap_or_else(|| noun.to_owned())
}

fn two_side_fallback_title(
    event: &GameEvent,
    first_civ_key: &str,
    first_team_key: &str,
    second_civ_key: &str,
    second_team_key: &str,
    noun: &str,
) -> String {
    let first = fact_string(event, first_civ_key).or_else(|| fact_string(event, first_team_key));
    let second = fact_string(event, second_civ_key).or_else(|| fact_string(event, second_team_key));

    match (first, second) {
        (Some(first), Some(second)) => format!("{first}-{second} {noun}"),
        (Some(first), None) => format!("{first} {noun}"),
        (None, Some(second)) => format!("{second} {noun}"),
        (None, None) => noun.to_owned(),
    }
}

fn named_fallback_title(event: &GameEvent, fact_key: &str, noun: &str) -> String {
    fact_string(event, fact_key)
        .map(|name| format!("{name} {noun}"))
        .unwrap_or_else(|| noun.to_owned())
}

fn involved_civilizations(event: &GameEvent) -> Vec<String> {
    unique_nonempty([
        fact_string(event, "player_civilization"),
        fact_string(event, "active_civilization"),
        fact_string(event, "owner_civilization"),
        fact_string(event, "old_owner_civilization"),
        fact_string(event, "new_owner_civilization"),
        fact_string(event, "razing_player_civilization"),
        fact_string(event, "declaring_team_civilization"),
        fact_string(event, "target_team_civilization"),
        fact_string(event, "first_team_civilization"),
        fact_string(event, "second_team_civilization"),
    ])
}

fn notable_places(event: &GameEvent) -> Vec<String> {
    unique_nonempty([fact_string(event, "city_name")])
}

fn notable_terms(event: &GameEvent) -> Vec<String> {
    unique_nonempty([
        fact_string(event, "religion_name"),
        fact_string(event, "building_name"),
        fact_string(event, "project_name"),
        fact_string(event, "tech_name"),
        fact_string(event, "victory_name"),
        fact_string(event, "great_person_name"),
        fact_string(event, "unit_name"),
    ])
}

fn unique_nonempty<const N: usize>(values: [Option<String>; N]) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn team_leader_from_prefix(event: &GameEvent, prefix: &str) -> Option<i32> {
    fact_i32(event, &format!("{prefix}_leader_player_id"))
}

fn fact_i32(event: &GameEvent, key: &str) -> Option<i32> {
    match event.facts.get(key) {
        Some(Value::Number(value)) => value.as_i64().map(|value| value as i32),
        Some(Value::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn fact_string(event: &GameEvent, key: &str) -> Option<String> {
    match event.facts.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ages_beyond_protocol::{DiplomacyTextRequest, GameEvent};
    use serde_json::json;

    use super::DirectorState;

    fn event(event_type: &str, facts: BTreeMap<String, serde_json::Value>) -> GameEvent {
        GameEvent {
            event_type: event_type.to_owned(),
            turn: Some(10),
            actors: Vec::new(),
            summary: None,
            facts,
        }
    }

    #[test]
    fn diplomacy_memory_records_wars_for_later_diplomacy() {
        let mut director = DirectorState::default();
        let arc_request = director
            .observe_event(&event(
                "war_declared",
                BTreeMap::from([
                    ("declaring_team_leader_player_id".to_owned(), json!(1)),
                    ("target_team_leader_player_id".to_owned(), json!(0)),
                    ("declaring_team_civilization".to_owned(), json!("Rome")),
                    ("target_team_civilization".to_owned(), json!("Egypt")),
                ]),
            ))
            .unwrap();
        director.apply_world_arc_title(&arc_request, "The Roman-Egyptian War".to_owned());

        let request = DiplomacyTextRequest {
            comment_type: "AI_DIPLOCOMMENT_GREETINGS".to_owned(),
            active_player_id: 0,
            leader_player_id: 1,
            turn: Some(10),
            active_player_name: None,
            active_civilization: None,
            leader_name: None,
            leader_civilization: None,
            attitude: None,
            at_war: true,
            power_relation: None,
            fallback_text: None,
            diplomacy_memory: None,
            world_arc: None,
        };

        let enriched = director.enrich_diplomacy_request(&request);
        assert!(enriched
            .diplomacy_memory
            .unwrap()
            .contains("Rome declared war on Egypt."));
        assert!(enriched
            .world_arc
            .unwrap()
            .contains("The Roman-Egyptian War"));
    }

    #[test]
    fn world_arc_enriches_future_events() {
        let mut director = DirectorState::default();
        let arc_request = director
            .observe_event(&event(
                "religion_founded",
                BTreeMap::from([("religion_name".to_owned(), json!("Buddhism"))]),
            ))
            .unwrap();
        director.apply_world_arc_title(&arc_request, "The Saffron Turning".to_owned());

        let enriched = director.enrich_event(&event("city_founded", BTreeMap::new()));

        assert_eq!(
            enriched
                .facts
                .get("world_arc_title")
                .and_then(|value| value.as_str()),
            Some("The Saffron Turning")
        );
    }

    #[test]
    fn world_arc_request_carries_relevant_game_terms() {
        let mut director = DirectorState::default();
        let arc_request = director
            .observe_event(&event(
                "wonder_built",
                BTreeMap::from([
                    ("player_civilization".to_owned(), json!("Mali")),
                    ("city_name".to_owned(), json!("Timbuktu")),
                    ("building_name".to_owned(), json!("The Oracle")),
                ]),
            ))
            .unwrap();

        assert_eq!(arc_request.fallback_title, "The Oracle Wonder");
        assert_eq!(arc_request.involved_civilizations, vec!["Mali"]);
        assert_eq!(arc_request.notable_places, vec!["Timbuktu"]);
        assert_eq!(arc_request.notable_terms, vec!["The Oracle"]);
    }
}
