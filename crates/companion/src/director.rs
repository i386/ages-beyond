#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::{HashMap, HashSet, VecDeque};

use ages_beyond_protocol::{
    DiplomacyTextRequest, GameEvent, HistoricalNameRequest, WorldArcRequest,
};
use serde_json::{json, Value};

const MAX_RELATIONSHIP_MEMORIES: usize = 8;
const MAX_RECENT_WORLD_EVENTS: usize = 12;
const MAX_RECENT_CONFLICTS: usize = 8;

#[derive(Debug, Clone, Default)]
pub struct DirectorState {
    relationship_memories: HashMap<RelationshipKey, VecDeque<String>>,
    recent_world_events: VecDeque<String>,
    active_conflicts: HashMap<RelationshipKey, NamedConflict>,
    recent_conflicts: VecDeque<NamedConflict>,
    civilization_arcs: HashMap<i32, CivilizationArc>,
    world_arc: Option<WorldArc>,
}

#[derive(Debug, Clone, Default)]
pub struct DirectorObservation {
    historical_names: Vec<HistoricalNameProposal>,
    world_arc: Option<WorldArcRequest>,
}

#[derive(Debug, Clone)]
pub struct HistoricalNameProposal {
    request: HistoricalNameRequest,
    target: HistoricalNameTarget,
}

#[derive(Debug, Clone)]
enum HistoricalNameTarget {
    Relationship(RelationshipKey),
    Civilization(i32),
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

#[derive(Debug, Clone)]
struct NamedConflict {
    key: RelationshipKey,
    title: String,
    first_civilization: String,
    second_civilization: String,
    started_turn: Option<i32>,
    ended_turn: Option<i32>,
    treaty_title: Option<String>,
}

#[derive(Debug, Clone)]
struct CivilizationArc {
    player_id: i32,
    civilization: String,
    title: String,
    theme: String,
    pressure: i32,
    updated_turn: Option<i32>,
}

impl DirectorState {
    pub fn observe_event(&mut self, event: &GameEvent) -> DirectorObservation {
        if let Some(memory) = relationship_memory(event) {
            self.remember_relationship(memory.first_player, memory.second_player, memory.text);
        }

        if let Some(summary) = world_event_summary(event) {
            self.remember_world_event(summary);
        }

        DirectorObservation {
            historical_names: self.propose_historical_names(event),
            world_arc: self.propose_world_arc(event),
        }
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

    pub fn apply_historical_name(&mut self, proposal: &HistoricalNameProposal, title: String) {
        let title = if title.trim().is_empty() {
            proposal.request.fallback_title.clone()
        } else {
            title.trim().to_owned()
        };

        match proposal.request.name_kind.as_str() {
            "war" => {
                let HistoricalNameTarget::Relationship(relationship_key) = proposal.target else {
                    return;
                };
                let conflict = NamedConflict {
                    key: relationship_key,
                    title: title.clone(),
                    first_civilization: proposal
                        .request
                        .involved_civilizations
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "One realm".to_owned()),
                    second_civilization: proposal
                        .request
                        .involved_civilizations
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| "another realm".to_owned()),
                    started_turn: proposal.request.turn,
                    ended_turn: None,
                    treaty_title: None,
                };
                self.active_conflicts
                    .insert(relationship_key, conflict.clone());
                self.remember_relationship(
                    relationship_key.first_player,
                    relationship_key.second_player,
                    format!(
                        "{} began between {} and {}.",
                        conflict.title, conflict.first_civilization, conflict.second_civilization
                    ),
                );
            }
            "treaty" => {
                let HistoricalNameTarget::Relationship(relationship_key) = proposal.target else {
                    return;
                };
                let mut conflict = self
                    .active_conflicts
                    .remove(&relationship_key)
                    .unwrap_or_else(|| NamedConflict {
                        key: relationship_key,
                        title: proposal
                            .request
                            .current_name
                            .clone()
                            .unwrap_or_else(|| proposal.request.fallback_title.clone()),
                        first_civilization: proposal
                            .request
                            .involved_civilizations
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "One realm".to_owned()),
                        second_civilization: proposal
                            .request
                            .involved_civilizations
                            .get(1)
                            .cloned()
                            .unwrap_or_else(|| "another realm".to_owned()),
                        started_turn: None,
                        ended_turn: None,
                        treaty_title: None,
                    });
                conflict.ended_turn = proposal.request.turn;
                conflict.treaty_title = Some(title.clone());
                self.remember_relationship(
                    relationship_key.first_player,
                    relationship_key.second_player,
                    format!("{} ended under {}.", conflict.title, title),
                );
                self.remember_recent_conflict(conflict);
            }
            "civilization_arc" => {
                let HistoricalNameTarget::Civilization(player_id) = proposal.target else {
                    return;
                };
                let civilization = proposal
                    .request
                    .involved_civilizations
                    .first()
                    .cloned()
                    .unwrap_or_else(|| format!("Player {player_id}"));
                self.civilization_arcs.insert(
                    player_id,
                    CivilizationArc {
                        player_id,
                        civilization,
                        title,
                        theme: proposal.request.theme.clone(),
                        pressure: civilization_arc_pressure(&proposal.request.trigger_event_type),
                        updated_turn: proposal.request.turn,
                    },
                );
            }
            _ => {}
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

        if let Some(conflict) = self.event_conflict(event) {
            enriched
                .facts
                .insert("named_conflict_title".to_owned(), json!(conflict.title));
            enriched.facts.insert(
                "named_conflict_participants".to_owned(),
                json!(format!(
                    "{} and {}",
                    conflict.first_civilization, conflict.second_civilization
                )),
            );
            if let Some(started_turn) = conflict.started_turn {
                enriched.facts.insert(
                    "named_conflict_started_turn".to_owned(),
                    json!(started_turn),
                );
            }
            if let Some(ended_turn) = conflict.ended_turn {
                enriched
                    .facts
                    .insert("named_conflict_ended_turn".to_owned(), json!(ended_turn));
            }
            if let Some(treaty_title) = &conflict.treaty_title {
                enriched
                    .facts
                    .insert("named_treaty_title".to_owned(), json!(treaty_title));
            }
        }

        let event_arcs = self.event_civilization_arcs(event);
        if !event_arcs.is_empty() {
            enriched.facts.insert(
                "civilization_arcs".to_owned(),
                json!(event_arcs
                    .iter()
                    .map(|arc| civilization_arc_summary(arc))
                    .collect::<Vec<_>>()
                    .join(" | ")),
            );
            if let Some(primary_arc) = event_arcs.first() {
                enriched.facts.insert(
                    "civilization_arc_title".to_owned(),
                    json!(primary_arc.title),
                );
                enriched.facts.insert(
                    "civilization_arc_civilization".to_owned(),
                    json!(primary_arc.civilization),
                );
            }
        }

        enriched
    }

    pub fn enrich_diplomacy_request(&self, request: &DiplomacyTextRequest) -> DiplomacyTextRequest {
        let mut enriched = request.clone();
        enriched.diplomacy_memory = self
            .relationship_summary_with_conflict(request.active_player_id, request.leader_player_id);
        enriched.world_arc =
            self.arc_context_for_diplomacy(request.active_player_id, request.leader_player_id);
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

    fn remember_recent_conflict(&mut self, conflict: NamedConflict) {
        self.recent_conflicts
            .retain(|existing| existing.key != conflict.key);
        self.recent_conflicts.push_back(conflict);
        while self.recent_conflicts.len() > MAX_RECENT_CONFLICTS {
            self.recent_conflicts.pop_front();
        }
    }

    fn propose_historical_names(&self, event: &GameEvent) -> Vec<HistoricalNameProposal> {
        let mut proposals = Vec::new();

        match event.event_type.as_str() {
            "war_declared" => {
                if let Some(key) = war_relationship_key(event) {
                    proposals.push(HistoricalNameProposal {
                        target: HistoricalNameTarget::Relationship(key),
                        request: HistoricalNameRequest {
                            name_kind: "war".to_owned(),
                            trigger_event_type: event.event_type.clone(),
                            turn: event.turn,
                            fallback_title: war_fallback_title(event),
                            subject: war_subject(event),
                            theme: "a newly declared war".to_owned(),
                            involved_civilizations: involved_civilizations(event),
                            notable_places: notable_places(event),
                            notable_terms: notable_terms(event),
                            recent_events: self.recent_world_events.iter().cloned().collect(),
                            current_name: None,
                        },
                    });
                }
            }
            "peace_signed" => {
                if let Some(key) = peace_relationship_key(event) {
                    let current_name = self
                        .active_conflicts
                        .get(&key)
                        .map(|conflict| conflict.title.clone());
                    proposals.push(HistoricalNameProposal {
                        target: HistoricalNameTarget::Relationship(key),
                        request: HistoricalNameRequest {
                            name_kind: "treaty".to_owned(),
                            trigger_event_type: event.event_type.clone(),
                            turn: event.turn,
                            fallback_title: peace_fallback_title(event),
                            subject: peace_subject(event, current_name.as_deref()),
                            theme: "a peace settlement ending a war".to_owned(),
                            involved_civilizations: involved_civilizations(event),
                            notable_places: notable_places(event),
                            notable_terms: notable_terms(event),
                            recent_events: self.recent_world_events.iter().cloned().collect(),
                            current_name,
                        },
                    });
                }
            }
            _ => {}
        }

        for player_id in civilization_arc_player_ids(event) {
            proposals.push(HistoricalNameProposal {
                target: HistoricalNameTarget::Civilization(player_id),
                request: HistoricalNameRequest {
                    name_kind: "civilization_arc".to_owned(),
                    trigger_event_type: event.event_type.clone(),
                    turn: event.turn,
                    fallback_title: civilization_arc_fallback_title(event, player_id),
                    subject: civilization_arc_subject(event, player_id),
                    theme: civilization_arc_theme(event),
                    involved_civilizations: civilization_arc_civilizations(event, player_id),
                    notable_places: notable_places(event),
                    notable_terms: notable_terms(event),
                    recent_events: self.recent_world_events.iter().cloned().collect(),
                    current_name: self
                        .civilization_arcs
                        .get(&player_id)
                        .map(|arc| arc.title.clone()),
                },
            });
        }

        proposals
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

    fn relationship_summary_with_conflict(
        &self,
        first_player: i32,
        second_player: i32,
    ) -> Option<String> {
        let key = RelationshipKey::new(first_player, second_player);
        let mut memories = self
            .relationship_memories
            .get(&key)
            .map(|memories| memories.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        if let Some(conflict) = self.active_conflicts.get(&key) {
            memories.push(format!(
                "Active named conflict: {} between {} and {}.",
                conflict.title, conflict.first_civilization, conflict.second_civilization
            ));
        } else if let Some(conflict) = self
            .recent_conflicts
            .iter()
            .rev()
            .find(|conflict| conflict.key == key)
        {
            if let Some(treaty) = &conflict.treaty_title {
                memories.push(format!(
                    "Recent named conflict: {} ended under {}.",
                    conflict.title, treaty
                ));
            }
        }

        if memories.is_empty() {
            None
        } else {
            Some(memories.join(" | "))
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

    fn arc_context_for_diplomacy(
        &self,
        active_player_id: i32,
        leader_player_id: i32,
    ) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(world_arc) = self.world_arc_summary() {
            parts.push(format!("World arc: {world_arc}"));
        }

        if let Some(active_arc) = self.civilization_arcs.get(&active_player_id) {
            parts.push(format!(
                "Active player civilization arc: {}",
                civilization_arc_summary(active_arc)
            ));
        }

        if let Some(leader_arc) = self.civilization_arcs.get(&leader_player_id) {
            parts.push(format!(
                "Rival leader civilization arc: {}",
                civilization_arc_summary(leader_arc)
            ));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" | "))
        }
    }

    fn event_civilization_arcs(&self, event: &GameEvent) -> Vec<&CivilizationArc> {
        civilization_arc_player_ids(event)
            .into_iter()
            .filter_map(|player_id| self.civilization_arcs.get(&player_id))
            .collect()
    }
}

impl DirectorObservation {
    pub fn historical_names(&self) -> &[HistoricalNameProposal] {
        &self.historical_names
    }

    pub fn historical_name(&self) -> Option<&HistoricalNameProposal> {
        self.historical_names.first()
    }

    pub fn world_arc(&self) -> Option<&WorldArcRequest> {
        self.world_arc.as_ref()
    }
}

impl HistoricalNameProposal {
    pub fn request(&self) -> &HistoricalNameRequest {
        &self.request
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

impl DirectorState {
    fn event_conflict(&self, event: &GameEvent) -> Option<&NamedConflict> {
        let key = match event.event_type.as_str() {
            "war_declared" => war_relationship_key(event),
            "peace_signed" => peace_relationship_key(event),
            _ => event_relationship_key(event),
        }?;

        self.active_conflicts.get(&key).or_else(|| {
            self.recent_conflicts
                .iter()
                .rev()
                .find(|conflict| conflict.key == key)
        })
    }
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

fn war_relationship_key(event: &GameEvent) -> Option<RelationshipKey> {
    let first = fact_i32(event, "declaring_team_leader_player_id")
        .or_else(|| fact_i32(event, "player_id"))?;
    let second = fact_i32(event, "target_team_leader_player_id").or_else(|| {
        team_leader_from_prefix(event, "target_team").or_else(|| fact_i32(event, "data1"))
    })?;
    Some(RelationshipKey::new(first, second))
}

fn peace_relationship_key(event: &GameEvent) -> Option<RelationshipKey> {
    let first =
        team_leader_from_prefix(event, "first_team").or_else(|| fact_i32(event, "player_id"))?;
    let second =
        team_leader_from_prefix(event, "second_team").or_else(|| fact_i32(event, "data1"))?;
    Some(RelationshipKey::new(first, second))
}

fn event_relationship_key(event: &GameEvent) -> Option<RelationshipKey> {
    let first = fact_i32(event, "player_id")?;
    let second = fact_i32(event, "data1")?;
    Some(RelationshipKey::new(first, second))
}

fn war_subject(event: &GameEvent) -> String {
    let first = fact_string(event, "declaring_team_civilization")
        .or_else(|| fact_string(event, "declaring_team_name"))
        .unwrap_or_else(|| "one realm".to_owned());
    let second = fact_string(event, "target_team_civilization")
        .or_else(|| fact_string(event, "target_team_name"))
        .unwrap_or_else(|| "another realm".to_owned());
    format!("{first} declared war on {second}")
}

fn peace_subject(event: &GameEvent, current_name: Option<&str>) -> String {
    let first = fact_string(event, "first_team_civilization")
        .or_else(|| fact_string(event, "first_team_name"))
        .unwrap_or_else(|| "one realm".to_owned());
    let second = fact_string(event, "second_team_civilization")
        .or_else(|| fact_string(event, "second_team_name"))
        .unwrap_or_else(|| "another realm".to_owned());

    match current_name {
        Some(current_name) => format!("{first} and {second} ended {current_name}"),
        None => format!("{first} made peace with {second}"),
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

fn civilization_arc_player_ids(event: &GameEvent) -> Vec<i32> {
    let ids = match event.event_type.as_str() {
        "war_declared" => vec![
            fact_i32(event, "declaring_team_leader_player_id"),
            fact_i32(event, "target_team_leader_player_id"),
        ],
        "peace_signed" => vec![
            team_leader_from_prefix(event, "first_team"),
            team_leader_from_prefix(event, "second_team"),
        ],
        "city_captured" | "city_acquired" => vec![
            fact_i32(event, "new_owner_id").or_else(|| fact_i32(event, "player_id")),
            fact_i32(event, "old_owner_id").or_else(|| fact_i32(event, "data1")),
        ],
        "city_razed" => vec![
            fact_i32(event, "razing_player_id").or_else(|| fact_i32(event, "player_id")),
            fact_i32(event, "old_owner_id").or_else(|| fact_i32(event, "data1")),
        ],
        "city_founded" | "religion_founded" | "wonder_built" | "project_built"
        | "golden_age_started" | "great_person_born" | "tech_discovered" => {
            vec![fact_i32(event, "player_id")
                .or_else(|| fact_i32(event, "owner_id"))
                .or_else(|| fact_i32(event, "discoverer_id"))]
        }
        _ => Vec::new(),
    };

    unique_i32(ids.into_iter().flatten().filter(|id| *id >= 0).collect())
}

fn civilization_arc_civilizations(event: &GameEvent, player_id: i32) -> Vec<String> {
    let matched = [
        ("player_id", "player_civilization"),
        ("active_player_id", "active_civilization"),
        ("owner_id", "owner_civilization"),
        ("old_owner_id", "old_owner_civilization"),
        ("new_owner_id", "new_owner_civilization"),
        ("razing_player_id", "razing_player_civilization"),
        ("discoverer_id", "discoverer_civilization"),
        (
            "declaring_team_leader_player_id",
            "declaring_team_civilization",
        ),
        ("target_team_leader_player_id", "target_team_civilization"),
        ("first_team_leader_player_id", "first_team_civilization"),
        ("second_team_leader_player_id", "second_team_civilization"),
    ];

    let civilizations = matched
        .iter()
        .filter_map(|(id_key, civ_key)| {
            (fact_i32(event, id_key) == Some(player_id))
                .then(|| fact_string(event, civ_key))
                .flatten()
        })
        .collect::<Vec<_>>();

    let civilizations = unique_strings(civilizations);
    if civilizations.is_empty() {
        involved_civilizations(event)
            .into_iter()
            .next()
            .into_iter()
            .collect()
    } else {
        civilizations
    }
}

fn civilization_arc_fallback_title(event: &GameEvent, player_id: i32) -> String {
    let civilization = civilization_arc_civilizations(event, player_id)
        .into_iter()
        .next()
        .unwrap_or_else(|| format!("Player {player_id}"));
    let noun = match event.event_type.as_str() {
        "war_declared" => "War Road",
        "peace_signed" => "Settlement",
        "religion_founded" => "Faith",
        "wonder_built" => "Monument Age",
        "project_built" => "Great Work",
        "city_founded" => "Founding",
        "city_captured" | "city_acquired" => "Conquest",
        "city_razed" => "Reckoning",
        "tech_discovered" => "Discovery",
        "golden_age_started" => "Golden Age",
        "great_person_born" => "Patronage",
        _ => "Arc",
    };
    format!("{civilization} {noun}")
}

fn civilization_arc_subject(event: &GameEvent, player_id: i32) -> String {
    let civilization = civilization_arc_civilizations(event, player_id)
        .into_iter()
        .next()
        .unwrap_or_else(|| format!("Player {player_id}"));
    let summary = event
        .summary
        .clone()
        .or_else(|| world_event_summary(event))
        .unwrap_or_else(|| event.event_type.replace('_', " "));
    format!("{civilization}: {summary}")
}

fn civilization_arc_theme(event: &GameEvent) -> String {
    match event.event_type.as_str() {
        "war_declared" => "civilization identity under war pressure",
        "peace_signed" => "civilization identity after settlement",
        "religion_founded" => "faith reshaping civilization identity",
        "wonder_built" => "prestige becoming national memory",
        "project_built" => "a great work defining public ambition",
        "city_founded" => "settlement and expansion",
        "city_captured" | "city_acquired" => "conquest and legitimacy",
        "city_razed" => "loss, vengeance, and survival",
        "tech_discovered" => "knowledge changing civic ambition",
        "golden_age_started" => "prosperity seeking legacy",
        "great_person_born" => "genius becoming public myth",
        _ => "civilization continuity",
    }
    .to_owned()
}

fn civilization_arc_pressure(event_type: &str) -> i32 {
    match event_type {
        "city_razed" => 5,
        "war_declared" | "city_captured" | "city_acquired" => 4,
        "religion_founded" | "wonder_built" | "project_built" | "golden_age_started" => 3,
        _ => 2,
    }
}

fn civilization_arc_summary(arc: &CivilizationArc) -> String {
    let turn = arc
        .updated_turn
        .map(|turn| format!("; updated turn {turn}"))
        .unwrap_or_default();
    format!(
        "{}: {} for {}; pressure {}{}",
        arc.title, arc.theme, arc.civilization, arc.pressure, turn
    )
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

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn unique_i32(values: Vec<i32>) -> Vec<i32> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(*value))
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
        let observation = director.observe_event(&event(
            "war_declared",
            BTreeMap::from([
                ("declaring_team_leader_player_id".to_owned(), json!(1)),
                ("target_team_leader_player_id".to_owned(), json!(0)),
                ("declaring_team_civilization".to_owned(), json!("Rome")),
                ("target_team_civilization".to_owned(), json!("Egypt")),
            ]),
        ));
        let historical_name = observation.historical_name().unwrap().clone();
        let arc_request = observation.world_arc().unwrap().clone();
        director.apply_historical_name(&historical_name, "The Nile Iron War".to_owned());
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
            .contains("Active named conflict: The Nile Iron War"));
        assert!(enriched
            .world_arc
            .unwrap()
            .contains("The Roman-Egyptian War"));
    }

    #[test]
    fn world_arc_enriches_future_events() {
        let mut director = DirectorState::default();
        let observation = director.observe_event(&event(
            "religion_founded",
            BTreeMap::from([("religion_name".to_owned(), json!("Buddhism"))]),
        ));
        let arc_request = observation.world_arc().unwrap().clone();
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
        let observation = director.observe_event(&event(
            "wonder_built",
            BTreeMap::from([
                ("player_civilization".to_owned(), json!("Mali")),
                ("city_name".to_owned(), json!("Timbuktu")),
                ("building_name".to_owned(), json!("The Oracle")),
            ]),
        ));
        let arc_request = observation.world_arc().unwrap();

        assert_eq!(arc_request.fallback_title, "The Oracle Wonder");
        assert_eq!(arc_request.involved_civilizations, vec!["Mali"]);
        assert_eq!(arc_request.notable_places, vec!["Timbuktu"]);
        assert_eq!(arc_request.notable_terms, vec!["The Oracle"]);
    }

    #[test]
    fn named_conflict_closes_with_named_treaty() {
        let mut director = DirectorState::default();
        let war_observation = director.observe_event(&event(
            "war_declared",
            BTreeMap::from([
                ("declaring_team_leader_player_id".to_owned(), json!(1)),
                ("target_team_leader_player_id".to_owned(), json!(0)),
                ("declaring_team_civilization".to_owned(), json!("Rome")),
                ("target_team_civilization".to_owned(), json!("Egypt")),
            ]),
        ));
        let war_name = war_observation.historical_name().unwrap().clone();
        director.apply_historical_name(&war_name, "The Nile Iron War".to_owned());

        let peace_event = event(
            "peace_signed",
            BTreeMap::from([
                ("first_team_leader_player_id".to_owned(), json!(1)),
                ("second_team_leader_player_id".to_owned(), json!(0)),
                ("first_team_civilization".to_owned(), json!("Rome")),
                ("second_team_civilization".to_owned(), json!("Egypt")),
            ]),
        );
        let peace_observation = director.observe_event(&peace_event);
        let treaty_name = peace_observation.historical_name().unwrap().clone();

        assert_eq!(
            treaty_name.request().current_name.as_deref(),
            Some("The Nile Iron War")
        );

        director.apply_historical_name(&treaty_name, "The Memphis Settlement".to_owned());
        let enriched = director.enrich_event(&peace_event);

        assert_eq!(
            enriched
                .facts
                .get("named_conflict_title")
                .and_then(|value| value.as_str()),
            Some("The Nile Iron War")
        );
        assert_eq!(
            enriched
                .facts
                .get("named_treaty_title")
                .and_then(|value| value.as_str()),
            Some("The Memphis Settlement")
        );
    }

    #[test]
    fn civilization_arc_enriches_later_events_for_same_civilization() {
        let mut director = DirectorState::default();
        let observation = director.observe_event(&event(
            "wonder_built",
            BTreeMap::from([
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("building_name".to_owned(), json!("The Oracle")),
            ]),
        ));
        let civilization_arc = observation
            .historical_names()
            .iter()
            .find(|proposal| proposal.request().name_kind == "civilization_arc")
            .unwrap()
            .clone();

        director.apply_historical_name(&civilization_arc, "The Gold Road Ascendant".to_owned());

        let enriched = director.enrich_event(&event(
            "city_founded",
            BTreeMap::from([
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
            ]),
        ));

        assert_eq!(
            enriched
                .facts
                .get("civilization_arc_title")
                .and_then(|value| value.as_str()),
            Some("The Gold Road Ascendant")
        );
        assert!(enriched
            .facts
            .get("civilization_arcs")
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("Mali"));
    }

    #[test]
    fn diplomacy_context_includes_both_civilization_arcs() {
        let mut director = DirectorState::default();
        let observation = director.observe_event(&event(
            "war_declared",
            BTreeMap::from([
                ("declaring_team_leader_player_id".to_owned(), json!(1)),
                ("target_team_leader_player_id".to_owned(), json!(0)),
                ("declaring_team_civilization".to_owned(), json!("Rome")),
                ("target_team_civilization".to_owned(), json!("Egypt")),
            ]),
        ));

        for proposal in observation.historical_names() {
            let request = proposal.request();
            let title = if request.name_kind == "war" {
                "The Nile Iron War"
            } else if request.involved_civilizations.first().map(String::as_str) == Some("Rome") {
                "The Iron Mandate"
            } else {
                "The Nile Reckoning"
            };
            director.apply_historical_name(proposal, title.to_owned());
        }

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
        let context = enriched.world_arc.unwrap();

        assert!(context.contains("Active player civilization arc: The Nile Reckoning"));
        assert!(context.contains("Rival leader civilization arc: The Iron Mandate"));
    }
}
