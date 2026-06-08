#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::{HashMap, HashSet, VecDeque};

use ages_beyond_protocol::{
    DiplomacyTextRequest, GameEvent, HistoricalNameRequest, WorldArcRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const MAX_RELATIONSHIP_MEMORIES: usize = 8;
const MAX_CIVILIZATION_MEMORIES: usize = 10;
const MAX_ACTIVE_LIVING_QUESTS: usize = 16;
const MAX_QUEST_PROGRESS_NOTES: usize = 6;
const MAX_RECENT_WORLD_EVENTS: usize = 12;
const MAX_RECENT_CONFLICTS: usize = 8;
const MAX_ERA_MEMORIES: usize = 6;
const MEMORY_SNAPSHOT_VERSION: u16 = 1;

#[derive(Debug, Clone, Default)]
pub struct DirectorState {
    relationship_memories: HashMap<RelationshipKey, VecDeque<String>>,
    civilization_memories: HashMap<i32, VecDeque<String>>,
    recent_world_events: VecDeque<String>,
    active_conflicts: HashMap<RelationshipKey, NamedConflict>,
    recent_conflicts: VecDeque<NamedConflict>,
    civilization_arcs: HashMap<i32, CivilizationArc>,
    living_quests: VecDeque<LivingQuest>,
    player_eras: HashMap<i32, PlayerEraState>,
    era_memories: HashMap<i32, VecDeque<String>>,
    world_arc: Option<WorldArc>,
}

#[derive(Debug, Clone, Default)]
pub struct DirectorObservation {
    historical_names: Vec<HistoricalNameProposal>,
    memory_projections: Vec<String>,
    quest_projections: Vec<String>,
    quest_decisions: Vec<QuestDecisionPrompt>,
    quest_rewards: Vec<QuestRewardCommand>,
    era_transition: Option<GameEvent>,
    world_arc: Option<WorldArcRequest>,
}

#[derive(Debug, Clone)]
pub struct HistoricalNameProposal {
    request: HistoricalNameRequest,
    target: HistoricalNameTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestRewardCommand {
    pub id: String,
    pub player_id: i32,
    pub reward_key: String,
    pub amount: i32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDecisionPrompt {
    pub id: String,
    pub player_id: i32,
    pub title: String,
    pub body: String,
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDecisionResponse {
    pub id: String,
    pub player_id: i32,
    pub choice: String,
}

#[derive(Debug, Clone)]
struct LivingQuestUpdate {
    projection: String,
    reward: Option<QuestRewardCommand>,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone)]
struct PlayerEraState {
    era_id: i32,
    era_name: String,
}

#[derive(Debug, Clone)]
struct LivingQuest {
    id: String,
    kind: String,
    player_id: i32,
    civilization: String,
    title: String,
    prompt: String,
    objective: LivingQuestObjective,
    reward: String,
    consequence: String,
    decision: Option<LivingQuestDecision>,
    origin_event_type: String,
    origin_turn: Option<i32>,
    target: Option<String>,
    status: String,
    progress_notes: VecDeque<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivingQuestDecision {
    pub id: String,
    pub choice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivingQuestObjective {
    pub key: String,
    pub text: String,
    pub progress: i32,
    pub required: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorMemorySnapshot {
    pub version: u16,
    pub recent_world_events: Vec<String>,
    pub world_arc: Option<WorldArcSnapshot>,
    pub civilizations: Vec<CivilizationMemorySnapshot>,
    pub relationships: Vec<RelationshipMemorySnapshot>,
    #[serde(default)]
    pub living_quests: Vec<LivingQuestSnapshot>,
    pub active_conflicts: Vec<NamedConflictSnapshot>,
    pub recent_conflicts: Vec<NamedConflictSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilizationMemorySnapshot {
    pub player_id: i32,
    pub civilization: Option<String>,
    pub arc: Option<CivilizationArcSnapshot>,
    pub era_memories: Vec<String>,
    pub memories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMemorySnapshot {
    pub first_player: i32,
    pub second_player: i32,
    pub memories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldArcSnapshot {
    pub title: String,
    pub theme: String,
    pub pressure: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilizationArcSnapshot {
    pub title: String,
    pub theme: String,
    pub pressure: i32,
    pub updated_turn: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedConflictSnapshot {
    pub first_player: i32,
    pub second_player: i32,
    pub title: String,
    pub first_civilization: String,
    pub second_civilization: String,
    pub started_turn: Option<i32>,
    pub ended_turn: Option<i32>,
    pub treaty_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivingQuestSnapshot {
    pub id: String,
    pub kind: String,
    pub player_id: i32,
    pub civilization: String,
    pub title: String,
    pub prompt: String,
    #[serde(default = "default_living_quest_objective")]
    pub objective: LivingQuestObjective,
    #[serde(default)]
    pub reward: String,
    #[serde(default)]
    pub consequence: String,
    #[serde(default)]
    pub decision: Option<LivingQuestDecision>,
    pub origin_event_type: String,
    pub origin_turn: Option<i32>,
    pub target: Option<String>,
    pub status: String,
    pub progress_notes: Vec<String>,
}

impl Default for LivingQuestObjective {
    fn default() -> Self {
        default_living_quest_objective()
    }
}

fn default_living_quest_objective() -> LivingQuestObjective {
    LivingQuestObjective {
        key: "story".to_owned(),
        text: "Advance this living quest through future campaign events.".to_owned(),
        progress: 0,
        required: 1,
    }
}

impl DirectorState {
    pub fn from_memory_snapshot(snapshot: DirectorMemorySnapshot) -> Result<Self, String> {
        if snapshot.version != MEMORY_SNAPSHOT_VERSION {
            return Err(format!(
                "unsupported memory snapshot version {}",
                snapshot.version
            ));
        }

        let mut state = Self::default();

        for event in snapshot.recent_world_events {
            state.remember_world_event(event);
        }

        state.world_arc = snapshot.world_arc.map(WorldArc::from);

        for civilization in snapshot.civilizations {
            let player_id = civilization.player_id;
            for memory in civilization.memories {
                state.remember_civilization(player_id, memory);
            }
            for memory in civilization.era_memories {
                state.remember_era_memory(player_id, memory);
            }
            if let Some(arc) = civilization.arc {
                state.civilization_arcs.insert(
                    player_id,
                    CivilizationArc {
                        player_id,
                        civilization: civilization
                            .civilization
                            .unwrap_or_else(|| format!("Player {player_id}")),
                        title: arc.title,
                        theme: arc.theme,
                        pressure: arc.pressure,
                        updated_turn: arc.updated_turn,
                    },
                );
            }
        }

        for relationship in snapshot.relationships {
            for memory in relationship.memories {
                state.remember_relationship(
                    relationship.first_player,
                    relationship.second_player,
                    memory,
                );
            }
        }

        for quest in snapshot.living_quests {
            state.remember_living_quest(LivingQuest::from(quest));
        }

        for conflict in snapshot.active_conflicts {
            let conflict = NamedConflict::from(conflict);
            state.active_conflicts.insert(conflict.key, conflict);
        }

        for conflict in snapshot.recent_conflicts {
            state.remember_recent_conflict(NamedConflict::from(conflict));
        }

        Ok(state)
    }

    pub fn observe_event(&mut self, event: &GameEvent) -> DirectorObservation {
        if let Some(memory) = relationship_memory(event) {
            self.remember_relationship(memory.first_player, memory.second_player, memory.text);
        }

        if let Some(summary) = world_event_summary(event) {
            self.remember_world_event(summary);
        }

        let mut memory_projections = Vec::new();
        for memory in civilization_memories(event) {
            if self.remember_civilization(memory.player_id, memory.text.clone()) {
                memory_projections.push(format!("Memory: {}", memory.text));
            }
        }

        let quest_updates = self.update_living_quests(event);
        let mut quest_projections = quest_updates
            .iter()
            .map(|update| update.projection.clone())
            .collect::<Vec<_>>();
        let quest_rewards = quest_updates
            .into_iter()
            .filter_map(|update| update.reward)
            .collect::<Vec<_>>();
        let mut quest_decisions = Vec::new();
        for quest in living_quest_seeds(event) {
            if self.remember_living_quest(quest.clone()) {
                quest_projections.push(quest_projection(&quest));
                quest_decisions.push(quest_decision_prompt(&quest));
            }
        }

        let era_transition = self.observe_era_transition(event);
        if let Some(era_event) = &era_transition {
            if let Some(summary) = &era_event.summary {
                memory_projections.push(format!("Memory: {summary}"));
            }
        }

        DirectorObservation {
            historical_names: self.propose_historical_names(event),
            memory_projections,
            quest_projections,
            quest_decisions,
            quest_rewards,
            era_transition,
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

        let civilization_memory = self.event_civilization_memory(event);
        if !civilization_memory.is_empty() {
            enriched.facts.insert(
                "civilization_memory".to_owned(),
                json!(civilization_memory.join(" | ")),
            );
        }

        let living_quests = self.event_living_quests(event);
        if !living_quests.is_empty() {
            enriched
                .facts
                .insert("living_quests".to_owned(), json!(living_quests.join(" | ")));
        }

        if let Some(player_id) = fact_i32(event, "player_id") {
            if let Some(memory) = self.era_memory_summary(player_id) {
                enriched
                    .facts
                    .insert("civilization_era_memory".to_owned(), json!(memory));
            }
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

    pub fn apply_quest_decision_responses(
        &mut self,
        responses: &[QuestDecisionResponse],
    ) -> Vec<String> {
        let mut projections = Vec::new();
        let mut memories = Vec::new();

        for response in responses {
            if response.id.trim().is_empty() || response.choice.trim().is_empty() {
                continue;
            }

            let Some(quest) = self.living_quests.iter_mut().find(|quest| {
                quest.player_id == response.player_id
                    && quest_decision_id(quest) == response.id
                    && quest.decision.is_none()
            }) else {
                continue;
            };

            let choice = response.choice.trim().to_owned();
            quest.decision = Some(LivingQuestDecision {
                id: response.id.clone(),
                choice: choice.clone(),
            });

            let note = format!(
                "{} chose a stance for {}: {}.",
                quest.civilization, quest.title, choice
            );
            quest.progress_notes.push_back(note.clone());
            while quest.progress_notes.len() > MAX_QUEST_PROGRESS_NOTES {
                quest.progress_notes.pop_front();
            }

            projections.push(format!("Quest: {} stance chosen. {}", quest.title, note));
            memories.push((quest.player_id, note));
        }

        for (player_id, memory) in memories {
            let _ = self.remember_civilization(player_id, memory);
        }

        projections
    }

    pub fn memory_snapshot(&self) -> DirectorMemorySnapshot {
        let mut player_ids = HashSet::new();
        player_ids.extend(self.civilization_memories.keys().copied());
        player_ids.extend(self.civilization_arcs.keys().copied());
        player_ids.extend(self.era_memories.keys().copied());
        player_ids.extend(self.living_quests.iter().map(|quest| quest.player_id));

        let mut player_ids = player_ids.into_iter().collect::<Vec<_>>();
        player_ids.sort_unstable();

        let civilizations = player_ids
            .into_iter()
            .map(|player_id| CivilizationMemorySnapshot {
                player_id,
                civilization: self
                    .civilization_arcs
                    .get(&player_id)
                    .map(|arc| arc.civilization.clone()),
                arc: self
                    .civilization_arcs
                    .get(&player_id)
                    .map(CivilizationArcSnapshot::from),
                era_memories: self
                    .era_memories
                    .get(&player_id)
                    .map(vecdeque_strings)
                    .unwrap_or_default(),
                memories: self
                    .civilization_memories
                    .get(&player_id)
                    .map(vecdeque_strings)
                    .unwrap_or_default(),
            })
            .collect();

        let mut relationships = self
            .relationship_memories
            .iter()
            .map(|(key, memories)| RelationshipMemorySnapshot {
                first_player: key.first_player,
                second_player: key.second_player,
                memories: vecdeque_strings(memories),
            })
            .collect::<Vec<_>>();
        relationships.sort_by_key(|memory| (memory.first_player, memory.second_player));

        let mut active_conflicts = self
            .active_conflicts
            .values()
            .map(NamedConflictSnapshot::from)
            .collect::<Vec<_>>();
        active_conflicts.sort_by_key(|conflict| (conflict.first_player, conflict.second_player));

        DirectorMemorySnapshot {
            version: MEMORY_SNAPSHOT_VERSION,
            recent_world_events: self.recent_world_events.iter().cloned().collect(),
            world_arc: self.world_arc.as_ref().map(WorldArcSnapshot::from),
            civilizations,
            relationships,
            living_quests: self
                .living_quests
                .iter()
                .map(LivingQuestSnapshot::from)
                .collect(),
            active_conflicts,
            recent_conflicts: self
                .recent_conflicts
                .iter()
                .map(NamedConflictSnapshot::from)
                .collect(),
        }
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

    fn remember_civilization(&mut self, player_id: i32, text: String) -> bool {
        if player_id < 0 || text.trim().is_empty() {
            return false;
        }

        let memories = self.civilization_memories.entry(player_id).or_default();
        if memories.iter().any(|memory| memory == &text) {
            return false;
        }

        memories.push_back(text);
        while memories.len() > MAX_CIVILIZATION_MEMORIES {
            memories.pop_front();
        }
        true
    }

    fn remember_living_quest(&mut self, quest: LivingQuest) -> bool {
        if quest.player_id < 0 || quest.id.trim().is_empty() {
            return false;
        }

        if self
            .living_quests
            .iter()
            .any(|existing| existing.id == quest.id)
        {
            return false;
        }

        self.living_quests.push_back(quest);
        while self.living_quests.len() > MAX_ACTIVE_LIVING_QUESTS {
            self.living_quests.pop_front();
        }
        true
    }

    fn update_living_quests(&mut self, event: &GameEvent) -> Vec<LivingQuestUpdate> {
        let mut updates = Vec::new();

        for quest in &mut self.living_quests {
            if quest.status != "active" {
                continue;
            }

            let Some(note) = quest_completion_note(quest, event) else {
                continue;
            };

            quest.status = "completed".to_owned();
            quest.objective.progress = quest.objective.required.max(1);
            quest.progress_notes.push_back(note.clone());
            while quest.progress_notes.len() > MAX_QUEST_PROGRESS_NOTES {
                quest.progress_notes.pop_front();
            }
            updates.push(LivingQuestUpdate {
                projection: format!(
                    "Quest: {} completed. {}{} Reward: {}",
                    quest.title,
                    note,
                    quest_decision_completion_note(quest),
                    quest.reward
                ),
                reward: Some(quest_reward_command(quest)),
            });
        }

        updates
    }

    fn remember_recent_conflict(&mut self, conflict: NamedConflict) {
        self.recent_conflicts
            .retain(|existing| existing.key != conflict.key);
        self.recent_conflicts.push_back(conflict);
        while self.recent_conflicts.len() > MAX_RECENT_CONFLICTS {
            self.recent_conflicts.pop_front();
        }
    }

    fn remember_era_memory(&mut self, player_id: i32, text: String) {
        if player_id < 0 || text.trim().is_empty() {
            return;
        }

        let memories = self.era_memories.entry(player_id).or_default();
        if memories.iter().any(|memory| memory == &text) {
            return;
        }

        memories.push_back(text);
        while memories.len() > MAX_ERA_MEMORIES {
            memories.pop_front();
        }
    }

    fn observe_era_transition(&mut self, event: &GameEvent) -> Option<GameEvent> {
        if event.event_type != "tech_discovered" {
            return None;
        }

        let player_id =
            fact_i32(event, "discoverer_id").or_else(|| fact_i32(event, "player_id"))?;
        let new_era_id = fact_i32(event, "tech_era_id")?;
        if new_era_id < 0 {
            return None;
        }

        let current_era_id = fact_i32(event, "era_id");
        let current_era_name = fact_string(event, "era_name");
        let previous = self.player_eras.get(&player_id).cloned().or_else(|| {
            current_era_id.map(|era_id| PlayerEraState {
                era_id,
                era_name: current_era_name
                    .clone()
                    .unwrap_or_else(|| era_label(era_id)),
            })
        });

        let new_era_name = fact_string(event, "tech_era_name")
            .or_else(|| {
                (current_era_id == Some(new_era_id))
                    .then(|| current_era_name.clone())
                    .flatten()
            })
            .unwrap_or_else(|| era_label(new_era_id));

        self.player_eras.insert(
            player_id,
            PlayerEraState {
                era_id: new_era_id,
                era_name: new_era_name.clone(),
            },
        );

        let previous = previous?;
        if new_era_id <= previous.era_id {
            return None;
        }

        let civilization = fact_string(event, "discoverer_civilization")
            .or_else(|| fact_string(event, "player_civilization"))
            .unwrap_or_else(|| format!("Player {player_id}"));
        let tech_name = fact_string(event, "tech_name").unwrap_or_else(|| "a discovery".to_owned());
        let memory = format!(
            "{} entered the {} after {}.",
            civilization, new_era_name, tech_name
        );
        self.remember_era_memory(player_id, memory);

        let mut facts = event.facts.clone();
        facts.insert(
            "event_id".to_owned(),
            json!(format!(
                "era:{player_id}:{}:{new_era_id}:{}",
                previous.era_id,
                event.turn.unwrap_or(-1)
            )),
        );
        facts.insert("internal_event".to_owned(), json!(true));
        facts.insert("importance".to_owned(), json!("epochal"));
        facts.insert("chapter".to_owned(), json!("Era Transitions"));
        facts.insert("story_arc".to_owned(), json!("era_transition"));
        facts.insert("player_id".to_owned(), json!(player_id));
        facts.insert(
            "player_civilization".to_owned(),
            json!(civilization.clone()),
        );
        facts.insert("old_era_id".to_owned(), json!(previous.era_id));
        facts.insert("old_era_name".to_owned(), json!(previous.era_name.clone()));
        facts.insert("new_era_id".to_owned(), json!(new_era_id));
        facts.insert("new_era_name".to_owned(), json!(new_era_name.clone()));
        facts.insert("triggering_tech_name".to_owned(), json!(tech_name.clone()));
        facts.insert("location_known_to_active_player".to_owned(), json!(false));
        facts.insert("is_global_announcement".to_owned(), json!(false));

        Some(GameEvent {
            event_type: "era_transition".to_owned(),
            turn: event.turn,
            actors: event.actors.clone(),
            summary: Some(format!(
                "{} entered the {} from the {} after {}.",
                civilization, new_era_name, previous.era_name, tech_name
            )),
            facts,
        })
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

        if let Some(memory) = self.civilization_memory_summary(active_player_id) {
            parts.push(format!("Active player civilization memory: {memory}"));
        }

        if let Some(memory) = self.civilization_memory_summary(leader_player_id) {
            parts.push(format!("Rival leader civilization memory: {memory}"));
        }

        if let Some(quests) = self.living_quest_summary(active_player_id) {
            parts.push(format!("Active player living quests: {quests}"));
        }

        if let Some(quests) = self.living_quest_summary(leader_player_id) {
            parts.push(format!("Rival leader living quests: {quests}"));
        }

        if let Some(memory) = self.era_memory_summary(active_player_id) {
            parts.push(format!("Active player era memory: {memory}"));
        }

        if let Some(memory) = self.era_memory_summary(leader_player_id) {
            parts.push(format!("Rival leader era memory: {memory}"));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" | "))
        }
    }

    fn living_quest_summary(&self, player_id: i32) -> Option<String> {
        let quests = self
            .living_quests
            .iter()
            .filter(|quest| quest.player_id == player_id && quest.status == "active")
            .map(living_quest_summary)
            .collect::<Vec<_>>();

        if quests.is_empty() {
            None
        } else {
            Some(quests.join(" | "))
        }
    }

    fn civilization_memory_summary(&self, player_id: i32) -> Option<String> {
        let memories = self.civilization_memories.get(&player_id)?;
        if memories.is_empty() {
            None
        } else {
            Some(memories.iter().cloned().collect::<Vec<_>>().join(" | "))
        }
    }

    fn era_memory_summary(&self, player_id: i32) -> Option<String> {
        let memories = self.era_memories.get(&player_id)?;
        if memories.is_empty() {
            None
        } else {
            Some(memories.iter().cloned().collect::<Vec<_>>().join(" | "))
        }
    }

    fn event_civilization_arcs(&self, event: &GameEvent) -> Vec<&CivilizationArc> {
        civilization_arc_player_ids(event)
            .into_iter()
            .filter_map(|player_id| self.civilization_arcs.get(&player_id))
            .collect()
    }

    fn event_civilization_memory(&self, event: &GameEvent) -> Vec<String> {
        civilization_arc_player_ids(event)
            .into_iter()
            .filter_map(|player_id| {
                self.civilization_memory_summary(player_id)
                    .map(|memory| format!("Player {player_id}: {memory}"))
            })
            .collect()
    }

    fn event_living_quests(&self, event: &GameEvent) -> Vec<String> {
        civilization_arc_player_ids(event)
            .into_iter()
            .filter_map(|player_id| {
                self.living_quest_summary(player_id)
                    .map(|quests| format!("Player {player_id}: {quests}"))
            })
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

    pub fn memory_projections(&self) -> &[String] {
        &self.memory_projections
    }

    pub fn quest_projections(&self) -> &[String] {
        &self.quest_projections
    }

    pub fn quest_decisions(&self) -> &[QuestDecisionPrompt] {
        &self.quest_decisions
    }

    pub fn quest_rewards(&self) -> &[QuestRewardCommand] {
        &self.quest_rewards
    }

    pub fn world_arc(&self) -> Option<&WorldArcRequest> {
        self.world_arc.as_ref()
    }

    pub fn era_transition(&self) -> Option<&GameEvent> {
        self.era_transition.as_ref()
    }
}

impl From<&WorldArc> for WorldArcSnapshot {
    fn from(arc: &WorldArc) -> Self {
        Self {
            title: arc.title.clone(),
            theme: arc.theme.clone(),
            pressure: arc.pressure,
        }
    }
}

impl From<WorldArcSnapshot> for WorldArc {
    fn from(snapshot: WorldArcSnapshot) -> Self {
        Self {
            title: snapshot.title,
            theme: snapshot.theme,
            pressure: snapshot.pressure,
        }
    }
}

impl From<&CivilizationArc> for CivilizationArcSnapshot {
    fn from(arc: &CivilizationArc) -> Self {
        Self {
            title: arc.title.clone(),
            theme: arc.theme.clone(),
            pressure: arc.pressure,
            updated_turn: arc.updated_turn,
        }
    }
}

impl From<&NamedConflict> for NamedConflictSnapshot {
    fn from(conflict: &NamedConflict) -> Self {
        Self {
            first_player: conflict.key.first_player,
            second_player: conflict.key.second_player,
            title: conflict.title.clone(),
            first_civilization: conflict.first_civilization.clone(),
            second_civilization: conflict.second_civilization.clone(),
            started_turn: conflict.started_turn,
            ended_turn: conflict.ended_turn,
            treaty_title: conflict.treaty_title.clone(),
        }
    }
}

impl From<NamedConflictSnapshot> for NamedConflict {
    fn from(snapshot: NamedConflictSnapshot) -> Self {
        Self {
            key: RelationshipKey::new(snapshot.first_player, snapshot.second_player),
            title: snapshot.title,
            first_civilization: snapshot.first_civilization,
            second_civilization: snapshot.second_civilization,
            started_turn: snapshot.started_turn,
            ended_turn: snapshot.ended_turn,
            treaty_title: snapshot.treaty_title,
        }
    }
}

impl From<&LivingQuest> for LivingQuestSnapshot {
    fn from(quest: &LivingQuest) -> Self {
        Self {
            id: quest.id.clone(),
            kind: quest.kind.clone(),
            player_id: quest.player_id,
            civilization: quest.civilization.clone(),
            title: quest.title.clone(),
            prompt: quest.prompt.clone(),
            objective: quest.objective.clone(),
            reward: quest.reward.clone(),
            consequence: quest.consequence.clone(),
            decision: quest.decision.clone(),
            origin_event_type: quest.origin_event_type.clone(),
            origin_turn: quest.origin_turn,
            target: quest.target.clone(),
            status: quest.status.clone(),
            progress_notes: quest.progress_notes.iter().cloned().collect(),
        }
    }
}

impl From<LivingQuestSnapshot> for LivingQuest {
    fn from(snapshot: LivingQuestSnapshot) -> Self {
        let objective = if snapshot.objective.text.trim().is_empty()
            || snapshot.objective.key == default_living_quest_objective().key
        {
            quest_objective(&snapshot.kind, snapshot.target.as_deref())
        } else {
            snapshot.objective
        };
        let reward = if snapshot.reward.trim().is_empty() {
            quest_reward(&snapshot.kind)
        } else {
            snapshot.reward
        };
        let consequence = if snapshot.consequence.trim().is_empty() {
            quest_consequence(&snapshot.kind)
        } else {
            snapshot.consequence
        };

        Self {
            id: snapshot.id,
            kind: snapshot.kind,
            player_id: snapshot.player_id,
            civilization: snapshot.civilization,
            title: snapshot.title,
            prompt: snapshot.prompt,
            objective,
            reward,
            consequence,
            decision: snapshot.decision,
            origin_event_type: snapshot.origin_event_type,
            origin_turn: snapshot.origin_turn,
            target: snapshot.target,
            status: snapshot.status,
            progress_notes: snapshot.progress_notes.into_iter().collect(),
        }
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

struct CivilizationMemory {
    player_id: i32,
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

fn civilization_memories(event: &GameEvent) -> Vec<CivilizationMemory> {
    match event.event_type.as_str() {
        "city_founded" => vec![player_memory(
            fact_i32(event, "player_id").or_else(|| fact_i32(event, "owner_id")),
            format!(
                "{} founded {}.",
                civilization_name(event, "player_civilization", "player_id"),
                fact_string(event, "city_name").unwrap_or_else(|| "a new city".to_owned())
            ),
        )],
        "city_captured" | "city_acquired" => {
            let city = fact_string(event, "city_name").unwrap_or_else(|| "a city".to_owned());
            let old_owner_id = fact_i32(event, "old_owner_id").or_else(|| fact_i32(event, "data1"));
            let new_owner_id =
                fact_i32(event, "new_owner_id").or_else(|| fact_i32(event, "player_id"));
            vec![
                player_memory(
                    new_owner_id,
                    format!(
                        "{} took control of {}.",
                        civilization_name(event, "new_owner_civilization", "new_owner_id"),
                        city
                    ),
                ),
                player_memory(
                    old_owner_id,
                    format!(
                        "{} lost {}.",
                        civilization_name(event, "old_owner_civilization", "old_owner_id"),
                        city
                    ),
                ),
            ]
        }
        "city_razed" => {
            let city = fact_string(event, "city_name").unwrap_or_else(|| "a city".to_owned());
            let old_owner_id = fact_i32(event, "old_owner_id").or_else(|| fact_i32(event, "data1"));
            let razing_player_id =
                fact_i32(event, "razing_player_id").or_else(|| fact_i32(event, "player_id"));
            vec![
                player_memory(
                    razing_player_id,
                    format!(
                        "{} razed {}.",
                        civilization_name(event, "razing_player_civilization", "razing_player_id"),
                        city
                    ),
                ),
                player_memory(
                    old_owner_id,
                    format!(
                        "{} remembers the destruction of {}.",
                        civilization_name(event, "old_owner_civilization", "old_owner_id"),
                        city
                    ),
                ),
            ]
        }
        "religion_founded" => vec![player_memory(
            fact_i32(event, "player_id").or_else(|| fact_i32(event, "founder_id")),
            format!(
                "{} became associated with the founding of {}.",
                civilization_name(event, "player_civilization", "player_id"),
                fact_string(event, "religion_name").unwrap_or_else(|| "a faith".to_owned())
            ),
        )],
        "wonder_built" => vec![player_memory(
            fact_i32(event, "player_id").or_else(|| fact_i32(event, "owner_id")),
            format!(
                "{} completed {}.",
                civilization_name(event, "player_civilization", "player_id"),
                fact_string(event, "building_name").unwrap_or_else(|| "a wonder".to_owned())
            ),
        )],
        "project_built" => vec![player_memory(
            fact_i32(event, "player_id").or_else(|| fact_i32(event, "owner_id")),
            format!(
                "{} completed {}.",
                civilization_name(event, "player_civilization", "player_id"),
                fact_string(event, "project_name").unwrap_or_else(|| "a great project".to_owned())
            ),
        )],
        "tech_discovered" => vec![player_memory(
            fact_i32(event, "discoverer_id").or_else(|| fact_i32(event, "player_id")),
            format!(
                "{} discovered {}.",
                civilization_name(event, "discoverer_civilization", "discoverer_id"),
                fact_string(event, "tech_name").unwrap_or_else(|| "new knowledge".to_owned())
            ),
        )],
        "golden_age_started" => vec![player_memory(
            fact_i32(event, "player_id"),
            format!(
                "{} entered a golden age.",
                civilization_name(event, "player_civilization", "player_id")
            ),
        )],
        "great_person_born" => vec![player_memory(
            fact_i32(event, "player_id").or_else(|| fact_i32(event, "owner_id")),
            format!(
                "{} gave rise to {}.",
                civilization_name(event, "player_civilization", "player_id"),
                fact_string(event, "great_person_name")
                    .or_else(|| fact_string(event, "unit_name"))
                    .unwrap_or_else(|| "a great person".to_owned())
            ),
        )],
        "war_declared" => vec![
            player_memory(
                fact_i32(event, "declaring_team_leader_player_id"),
                format!(
                    "{} opened war against {}.",
                    civilization_name(
                        event,
                        "declaring_team_civilization",
                        "declaring_team_leader_player_id"
                    ),
                    fact_string(event, "target_team_civilization")
                        .or_else(|| fact_string(event, "target_team_name"))
                        .unwrap_or_else(|| "a rival".to_owned())
                ),
            ),
            player_memory(
                fact_i32(event, "target_team_leader_player_id"),
                format!(
                    "{} was drawn into war with {}.",
                    civilization_name(
                        event,
                        "target_team_civilization",
                        "target_team_leader_player_id"
                    ),
                    fact_string(event, "declaring_team_civilization")
                        .or_else(|| fact_string(event, "declaring_team_name"))
                        .unwrap_or_else(|| "a rival".to_owned())
                ),
            ),
        ],
        "peace_signed" => vec![
            player_memory(
                team_leader_from_prefix(event, "first_team"),
                format!(
                    "{} accepted peace with {}.",
                    civilization_name(
                        event,
                        "first_team_civilization",
                        "first_team_leader_player_id"
                    ),
                    fact_string(event, "second_team_civilization")
                        .or_else(|| fact_string(event, "second_team_name"))
                        .unwrap_or_else(|| "a rival".to_owned())
                ),
            ),
            player_memory(
                team_leader_from_prefix(event, "second_team"),
                format!(
                    "{} accepted peace with {}.",
                    civilization_name(
                        event,
                        "second_team_civilization",
                        "second_team_leader_player_id"
                    ),
                    fact_string(event, "first_team_civilization")
                        .or_else(|| fact_string(event, "first_team_name"))
                        .unwrap_or_else(|| "a rival".to_owned())
                ),
            ),
        ],
        _ => Vec::new(),
    }
    .into_iter()
    .flatten()
    .collect()
}

fn living_quest_seeds(event: &GameEvent) -> Vec<LivingQuest> {
    let Some(seed) = fact_string(event, "dynamic_quest_seed") else {
        return Vec::new();
    };

    match seed.as_str() {
        "city_ruins_legacy" => {
            let player_id = fact_i32(event, "old_owner_id").or_else(|| fact_i32(event, "data1"));
            let city = fact_string(event, "city_name").unwrap_or_else(|| "the lost city".to_owned());
            let civilization = civilization_name(event, "old_owner_civilization", "old_owner_id");
            vec![living_quest(
                event,
                "restoration",
                player_id,
                civilization.clone(),
                format!("Remember {city}"),
                format!(
                    "{} must decide whether the memory of {} becomes mourning, vengeance, or restoration.",
                    civilization, city
                ),
                Some(city),
            )]
        }
        "occupation_aftermath" | "city_transition" => {
            let city = fact_string(event, "city_name").unwrap_or_else(|| "the city".to_owned());
            let old_owner_id =
                fact_i32(event, "old_owner_id").or_else(|| fact_i32(event, "data1"));
            let new_owner_id =
                fact_i32(event, "new_owner_id").or_else(|| fact_i32(event, "player_id"));
            let old_civ = civilization_name(event, "old_owner_civilization", "old_owner_id");
            let new_civ = civilization_name(event, "new_owner_civilization", "new_owner_id");
            vec![
                living_quest(
                    event,
                    "restoration",
                    old_owner_id,
                    old_civ.clone(),
                    format!("The Claim on {city}"),
                    format!(
                        "{} must decide whether {} remains a wound, a bargaining chip, or a future restoration.",
                        old_civ, city
                    ),
                    Some(city.clone()),
                ),
                living_quest(
                    event,
                    "legitimacy",
                    new_owner_id,
                    new_civ.clone(),
                    format!("Settle {city}"),
                    format!(
                        "{} must make {} feel governed, not merely taken.",
                        new_civ, city
                    ),
                    Some(city),
                ),
            ]
        }
        "wonder_legacy" => single_player_quest(
            event,
            "legacy",
            format!(
                "Make {} Matter",
                fact_string(event, "building_name").unwrap_or_else(|| "the wonder".to_owned())
            ),
            format!(
                "{} must give {} a deed worthy of its monument.",
                event_player_civilization(event),
                fact_string(event, "building_name").unwrap_or_else(|| "the wonder".to_owned())
            ),
            fact_string(event, "building_name"),
        ),
        "holy_city" => single_player_quest(
            event,
            "faith",
            format!(
                "Carry {}",
                fact_string(event, "religion_name").unwrap_or_else(|| "the faith".to_owned())
            ),
            format!(
                "{} must decide whether {} becomes private devotion, civic law, or a bridge to other peoples.",
                event_player_civilization(event),
                fact_string(event, "religion_name").unwrap_or_else(|| "the faith".to_owned())
            ),
            fact_string(event, "religion_name"),
        ),
        "new_knowledge" => single_player_quest(
            event,
            "breakthrough",
            format!(
                "Apply {}",
                fact_string(event, "tech_name").unwrap_or_else(|| "the discovery".to_owned())
            ),
            format!(
                "{} must turn {} from knowledge into a visible change in the realm.",
                event_player_civilization(event),
                fact_string(event, "tech_name").unwrap_or_else(|| "the discovery".to_owned())
            ),
            fact_string(event, "tech_name"),
        ),
        "golden_age_mandate" => single_player_quest(
            event,
            "mandate",
            format!("Spend the Bright Years"),
            format!(
                "{} must choose what should survive after the golden age fades.",
                event_player_civilization(event)
            ),
            None,
        ),
        "great_person_legacy" => single_player_quest(
            event,
            "legacy",
            format!(
                "The Work of {}",
                fact_string(event, "great_person_name")
                    .or_else(|| fact_string(event, "unit_name"))
                    .unwrap_or_else(|| "the great person".to_owned())
            ),
            format!(
                "{} must decide what institution, rivalry, or work will carry this life into memory.",
                event_player_civilization(event)
            ),
            fact_string(event, "great_person_name").or_else(|| fact_string(event, "unit_name")),
        ),
        "war_aims" => {
            let declaring_player = fact_i32(event, "declaring_team_leader_player_id");
            let target_player = fact_i32(event, "target_team_leader_player_id");
            let declaring_civ =
                civilization_name(event, "declaring_team_civilization", "declaring_team_leader_player_id");
            let target_civ =
                civilization_name(event, "target_team_civilization", "target_team_leader_player_id");
            vec![
                living_quest(
                    event,
                    "war_aim",
                    declaring_player,
                    declaring_civ.clone(),
                    "Name the War Aim".to_owned(),
                    format!(
                        "{} must decide what victory over {} is supposed to prove.",
                        declaring_civ, target_civ
                    ),
                    Some(target_civ.clone()),
                ),
                living_quest(
                    event,
                    "survival",
                    target_player,
                    target_civ.clone(),
                    "Answer the War".to_owned(),
                    format!(
                        "{} must decide whether this war becomes endurance, reprisal, or reconciliation.",
                        target_civ
                    ),
                    Some(declaring_civ),
                ),
            ]
        }
        "peace_terms" => {
            let first_player = team_leader_from_prefix(event, "first_team");
            let second_player = team_leader_from_prefix(event, "second_team");
            let first_civ = civilization_name(event, "first_team_civilization", "first_team_leader_player_id");
            let second_civ = civilization_name(event, "second_team_civilization", "second_team_leader_player_id");
            vec![
                living_quest(
                    event,
                    "settlement",
                    first_player,
                    first_civ.clone(),
                    "Keep the Peace".to_owned(),
                    format!(
                        "{} must decide what concession, memorial, or boundary keeps peace with {} from becoming only a pause.",
                        first_civ, second_civ
                    ),
                    Some(second_civ.clone()),
                ),
                living_quest(
                    event,
                    "settlement",
                    second_player,
                    second_civ.clone(),
                    "Keep the Peace".to_owned(),
                    format!(
                        "{} must decide what concession, memorial, or boundary keeps peace with {} from becoming only a pause.",
                        second_civ, first_civ
                    ),
                    Some(first_civ),
                ),
            ]
        }
        "settlement_identity" => single_player_quest(
            event,
            "settlement",
            format!(
                "Define {}",
                fact_string(event, "city_name").unwrap_or_else(|| "the new city".to_owned())
            ),
            format!(
                "{} must decide whether {} is remembered for safety, ambition, exile, or faith.",
                event_player_civilization(event),
                fact_string(event, "city_name").unwrap_or_else(|| "the new city".to_owned())
            ),
            fact_string(event, "city_name"),
        ),
        "great_project_consequences" => single_player_quest(
            event,
            "project",
            format!(
                "Bear {}",
                fact_string(event, "project_name").unwrap_or_else(|| "the great project".to_owned())
            ),
            format!(
                "{} must decide what burden {} was built to carry.",
                event_player_civilization(event),
                fact_string(event, "project_name").unwrap_or_else(|| "the great project".to_owned())
            ),
            fact_string(event, "project_name"),
        ),
        _ => Vec::new(),
    }
    .into_iter()
    .flatten()
    .collect()
}

fn single_player_quest(
    event: &GameEvent,
    kind: &str,
    title: String,
    prompt: String,
    target: Option<String>,
) -> Vec<Option<LivingQuest>> {
    vec![living_quest(
        event,
        kind,
        event_player_id(event),
        event_player_civilization(event),
        title,
        prompt,
        target,
    )]
}

fn living_quest(
    event: &GameEvent,
    kind: &str,
    player_id: Option<i32>,
    civilization: String,
    title: String,
    prompt: String,
    target: Option<String>,
) -> Option<LivingQuest> {
    let player_id = player_id?;
    if player_id < 0 {
        return None;
    }

    let id = format!(
        "{}:{player_id}:{kind}:{}",
        event_id_token(event),
        target
            .as_deref()
            .unwrap_or(&title)
            .replace(' ', "_")
            .to_lowercase()
    );

    Some(LivingQuest {
        id,
        kind: kind.to_owned(),
        player_id,
        civilization,
        title,
        prompt,
        objective: quest_objective(kind, target.as_deref()),
        reward: quest_reward(kind),
        consequence: quest_consequence(kind),
        decision: None,
        origin_event_type: event.event_type.clone(),
        origin_turn: event.turn,
        target,
        status: "active".to_owned(),
        progress_notes: VecDeque::new(),
    })
}

fn quest_projection(quest: &LivingQuest) -> String {
    format!(
        "Quest: {} - {} Objective: {} Reward: {} ({})",
        quest.title, quest.prompt, quest.objective.text, quest.reward, quest.civilization
    )
}

fn quest_decision_prompt(quest: &LivingQuest) -> QuestDecisionPrompt {
    QuestDecisionPrompt {
        id: quest_decision_id(quest),
        player_id: quest.player_id,
        title: quest.title.clone(),
        body: format!(
            "{}\n\nObjective: {}\n\nChoose how this quest should be remembered.",
            quest.prompt, quest.objective.text
        ),
        choices: quest_decision_choices(&quest.kind),
    }
}

fn quest_decision_id(quest: &LivingQuest) -> String {
    format!("decision:{}", quest.id)
}

fn quest_decision_choices(kind: &str) -> Vec<String> {
    match kind {
        "restoration" => vec![
            "Restore what was lost".to_owned(),
            "Swear vengeance".to_owned(),
            "Let memory become warning".to_owned(),
        ],
        "legitimacy" => vec![
            "Govern with mercy".to_owned(),
            "Rule with order".to_owned(),
            "Make an example".to_owned(),
        ],
        "faith" => vec![
            "Make it devotion".to_owned(),
            "Make it law".to_owned(),
            "Make it diplomacy".to_owned(),
        ],
        "war_aim" => vec![
            "Seek decisive victory".to_owned(),
            "Seek tribute".to_owned(),
            "Seek recognition".to_owned(),
        ],
        "survival" => vec![
            "Endure".to_owned(),
            "Prepare reprisal".to_owned(),
            "Seek reconciliation".to_owned(),
        ],
        "settlement" => vec![
            "Honor the peace".to_owned(),
            "Secure the border".to_owned(),
            "Prepare for betrayal".to_owned(),
        ],
        _ => vec![
            "Build a legacy".to_owned(),
            "Use it for power".to_owned(),
            "Let it change the people".to_owned(),
        ],
    }
}

fn living_quest_summary(quest: &LivingQuest) -> String {
    let target = quest
        .target
        .as_deref()
        .map(|target| format!("; target {target}"))
        .unwrap_or_default();
    let decision = quest
        .decision
        .as_ref()
        .map(|decision| format!(" Stance: {}", decision.choice))
        .unwrap_or_default();
    format!(
        "{} [{}]: {} Objective: {} Progress: {}/{}{}{}",
        quest.title,
        quest.kind,
        quest.prompt,
        quest.objective.text,
        quest.objective.progress,
        quest.objective.required,
        target,
        decision
    )
}

fn quest_objective(kind: &str, target: Option<&str>) -> LivingQuestObjective {
    let key = match kind {
        "restoration" => "regain_city",
        "legitimacy" => "stabilize_city",
        "legacy" => "create_legacy",
        "faith" => "shape_faith",
        "breakthrough" => "apply_discovery",
        "mandate" => "spend_golden_age",
        "war_aim" => "prove_war_aim",
        "survival" => "survive_war",
        "settlement" => "settle_terms",
        "project" => "bear_project",
        _ => "story",
    };
    let target = target.unwrap_or("the realm");
    let text = match kind {
        "restoration" => format!("Regain or meaningfully answer the loss of {target}."),
        "legitimacy" => format!("Stabilize {target} so conquest becomes accepted rule."),
        "legacy" => format!("Create a lasting deed worthy of {target}."),
        "faith" => format!("Shape {target} into devotion, law, or diplomacy."),
        "breakthrough" => {
            format!("Turn {target} into a visible civic, military, or economic change.")
        }
        "mandate" => "Complete a lasting achievement before the golden age fades.".to_owned(),
        "war_aim" => format!("Win a meaningful victory or settlement against {target}."),
        "survival" => format!("Survive pressure from {target} and define the answer."),
        "settlement" => format!("Turn peace or settlement with {target} into stable memory."),
        "project" => format!("Absorb the cost of {target} and turn it into long-term leverage."),
        _ => "Advance this living quest through future campaign events.".to_owned(),
    };

    LivingQuestObjective {
        key: key.to_owned(),
        text,
        progress: 0,
        required: 1,
    }
}

fn quest_reward(kind: &str) -> String {
    match kind {
        "restoration" => {
            "Future narration and diplomacy remember the restoration as a legitimacy claim."
                .to_owned()
        }
        "legitimacy" => {
            "Future narration treats the captured city as integrated rather than merely occupied."
                .to_owned()
        }
        "legacy" => {
            "Future arcs can treat the achievement as a cultural or strategic mandate.".to_owned()
        }
        "faith" => {
            "Future diplomacy and chronicle entries can treat the faith as an active identity."
                .to_owned()
        }
        "breakthrough" => {
            "Future arcs can treat the discovery as applied power, not just knowledge.".to_owned()
        }
        "mandate" => "Future memory records what the golden age was spent to preserve.".to_owned(),
        "war_aim" => "Future diplomacy can cite the war aim as fulfilled or betrayed.".to_owned(),
        "survival" => {
            "Future diplomacy can cite survival as endurance, reprisal, or reconciliation."
                .to_owned()
        }
        "settlement" => {
            "Future relationship memory can cite the settlement as a durable peace.".to_owned()
        }
        "project" => {
            "Future arcs can treat the project as a burden carried into lasting power.".to_owned()
        }
        _ => "Future narration remembers how this quest resolved.".to_owned(),
    }
}

fn quest_reward_command(quest: &LivingQuest) -> QuestRewardCommand {
    let amount = quest_reward_gold(quest);
    QuestRewardCommand {
        id: format!("reward:{}", quest.id),
        player_id: quest.player_id,
        reward_key: "gold".to_owned(),
        amount,
        text: format!(
            "Living Quest reward: {} completed.{} +{} gold.",
            quest.title,
            quest_decision_reward_note(quest),
            amount
        ),
    }
}

fn quest_reward_gold(quest: &LivingQuest) -> i32 {
    let base = match quest.kind.as_str() {
        "restoration" => 75,
        "legitimacy" => 50,
        "legacy" => 50,
        "faith" => 45,
        "breakthrough" => 45,
        "mandate" => 60,
        "war_aim" => 75,
        "survival" => 75,
        "settlement" => 50,
        "project" => 60,
        _ => 40,
    };
    base + quest_decision_reward_bonus(quest)
}

fn quest_decision_reward_bonus(quest: &LivingQuest) -> i32 {
    let Some(decision) = &quest.decision else {
        return 0;
    };

    match decision.choice.as_str() {
        "Swear vengeance"
        | "Make an example"
        | "Seek decisive victory"
        | "Prepare reprisal"
        | "Use it for power" => 25,
        "Let memory become warning"
        | "Govern with mercy"
        | "Make it diplomacy"
        | "Seek reconciliation"
        | "Honor the peace" => 10,
        _ => 15,
    }
}

fn quest_decision_completion_note(quest: &LivingQuest) -> String {
    quest
        .decision
        .as_ref()
        .map(|decision| format!(" Stance honored: {}.", decision.choice))
        .unwrap_or_default()
}

fn quest_decision_reward_note(quest: &LivingQuest) -> String {
    quest
        .decision
        .as_ref()
        .map(|decision| format!(" Stance: {}.", decision.choice))
        .unwrap_or_default()
}

fn quest_consequence(kind: &str) -> String {
    match kind {
        "restoration" => "If ignored, future narration may frame the loss as an unresolved grievance.".to_owned(),
        "legitimacy" => "If ignored, future narration may frame the city as occupation and instability.".to_owned(),
        "legacy" => "If ignored, future narration may frame the achievement as prestige without purpose.".to_owned(),
        "faith" => "If ignored, future narration may frame the faith as private belief rather than public force.".to_owned(),
        "breakthrough" => "If ignored, future narration may frame the discovery as unused potential.".to_owned(),
        "mandate" => "If ignored, future narration may frame the golden age as a squandered moment.".to_owned(),
        "war_aim" => "If ignored, future diplomacy may frame the war as violence without a declared purpose.".to_owned(),
        "survival" => "If ignored, future diplomacy may frame the war as trauma without answer.".to_owned(),
        "settlement" => "If ignored, future diplomacy may frame the peace as only a pause.".to_owned(),
        "project" => "If ignored, future narration may frame the project as cost without settlement.".to_owned(),
        _ => "If ignored, future narration may remember this as unresolved pressure.".to_owned(),
    }
}

fn quest_completion_note(quest: &LivingQuest, event: &GameEvent) -> Option<String> {
    match quest.kind.as_str() {
        "restoration" => restoration_completion_note(quest, event),
        "legitimacy" => legitimacy_completion_note(quest, event),
        "legacy" => player_milestone_completion_note(
            quest,
            event,
            &[
                "wonder_built",
                "project_built",
                "tech_discovered",
                "golden_age_started",
                "great_person_born",
                "city_founded",
                "city_captured",
                "city_acquired",
                "peace_signed",
                "war_declared",
                "victory",
            ],
            "gave the quest a new deed through",
        ),
        "faith" => player_milestone_completion_note(
            quest,
            event,
            &[
                "peace_signed",
                "war_declared",
                "wonder_built",
                "project_built",
                "golden_age_started",
                "tech_discovered",
                "city_founded",
            ],
            "made the faith public through",
        ),
        "breakthrough" => player_milestone_completion_note(
            quest,
            event,
            &[
                "project_built",
                "wonder_built",
                "city_founded",
                "city_captured",
                "city_acquired",
                "golden_age_started",
                "war_declared",
            ],
            "turned the discovery into visible power through",
        ),
        "mandate" => player_milestone_completion_note(
            quest,
            event,
            &[
                "wonder_built",
                "project_built",
                "great_person_born",
                "city_founded",
                "city_captured",
                "city_acquired",
                "religion_founded",
                "tech_discovered",
                "peace_signed",
                "war_declared",
                "victory",
            ],
            "spent the mandate through",
        ),
        "war_aim" => war_aim_completion_note(quest, event),
        "survival" => survival_completion_note(quest, event),
        "settlement" => settlement_completion_note(quest, event),
        "project" => player_milestone_completion_note(
            quest,
            event,
            &[
                "golden_age_started",
                "tech_discovered",
                "wonder_built",
                "great_person_born",
                "war_declared",
                "peace_signed",
            ],
            "turned the project into lasting leverage through",
        ),
        _ => None,
    }
}

fn restoration_completion_note(quest: &LivingQuest, event: &GameEvent) -> Option<String> {
    if !matches!(event.event_type.as_str(), "city_captured" | "city_acquired")
        || !quest_city_target_matches(quest, event)
        || event_new_owner_id(event) != Some(quest.player_id)
    {
        return None;
    }

    Some(format!(
        "{} regained {}.",
        quest.civilization,
        quest_target_label(quest)
    ))
}

fn legitimacy_completion_note(quest: &LivingQuest, event: &GameEvent) -> Option<String> {
    if !quest_player_is_primary(event, quest.player_id)
        || !matches!(
            event.event_type.as_str(),
            "wonder_built"
                | "project_built"
                | "religion_founded"
                | "great_person_born"
                | "golden_age_started"
                | "tech_discovered"
        )
    {
        return None;
    }

    let city_scoped = quest_city_target_matches(quest, event);
    let realm_scoped = fact_string(event, "city_name").is_none()
        && matches!(
            event.event_type.as_str(),
            "religion_founded" | "golden_age_started" | "tech_discovered"
        );
    if !city_scoped && !realm_scoped {
        return None;
    }

    Some(format!(
        "{} made {} accepted through {}.",
        quest.civilization,
        quest_target_label(quest),
        event_deed_label(event)
    ))
}

fn war_aim_completion_note(quest: &LivingQuest, event: &GameEvent) -> Option<String> {
    let target_matches = quest_target_civilization_matches(quest, event);
    let completed = match event.event_type.as_str() {
        "peace_signed" => peace_involves_player(event, quest.player_id) && target_matches,
        "city_captured" | "city_acquired" => {
            event_new_owner_id(event) == Some(quest.player_id) && target_matches
        }
        _ => false,
    };
    completed.then(|| {
        format!(
            "{} proved its war aim against {} through {}.",
            quest.civilization,
            quest_target_label(quest),
            event_deed_label(event)
        )
    })
}

fn survival_completion_note(quest: &LivingQuest, event: &GameEvent) -> Option<String> {
    let target_matches = quest_target_civilization_matches(quest, event);
    let completed = match event.event_type.as_str() {
        "peace_signed" => peace_involves_player(event, quest.player_id) && target_matches,
        "city_captured" | "city_acquired" => {
            event_new_owner_id(event) == Some(quest.player_id) && target_matches
        }
        _ => false,
    };
    completed.then(|| {
        format!(
            "{} answered pressure from {} through {}.",
            quest.civilization,
            quest_target_label(quest),
            event_deed_label(event)
        )
    })
}

fn settlement_completion_note(quest: &LivingQuest, event: &GameEvent) -> Option<String> {
    let completed = match event.event_type.as_str() {
        "peace_signed" => {
            peace_involves_player(event, quest.player_id)
                && quest_target_civilization_matches(quest, event)
        }
        "city_founded" | "wonder_built" | "project_built" | "golden_age_started"
        | "tech_discovered" => quest_player_is_primary(event, quest.player_id),
        _ => false,
    };
    completed.then(|| {
        format!(
            "{} turned settlement with {} into stable memory through {}.",
            quest.civilization,
            quest_target_label(quest),
            event_deed_label(event)
        )
    })
}

fn player_milestone_completion_note(
    quest: &LivingQuest,
    event: &GameEvent,
    event_types: &[&str],
    verb: &str,
) -> Option<String> {
    if !event_types.contains(&event.event_type.as_str())
        || !quest_player_is_primary(event, quest.player_id)
    {
        return None;
    }

    Some(format!(
        "{} {} {}.",
        quest.civilization,
        verb,
        event_deed_label(event)
    ))
}

fn quest_player_is_primary(event: &GameEvent, player_id: i32) -> bool {
    match event.event_type.as_str() {
        "city_captured" | "city_acquired" => event_new_owner_id(event) == Some(player_id),
        "city_razed" => {
            fact_i32(event, "razing_player_id").or_else(|| fact_i32(event, "player_id"))
                == Some(player_id)
        }
        "war_declared" => war_involves_player(event, player_id),
        "peace_signed" => peace_involves_player(event, player_id),
        _ => event_player_id(event) == Some(player_id),
    }
}

fn event_new_owner_id(event: &GameEvent) -> Option<i32> {
    fact_i32(event, "new_owner_id").or_else(|| fact_i32(event, "player_id"))
}

fn peace_involves_player(event: &GameEvent, player_id: i32) -> bool {
    team_leader_from_prefix(event, "first_team") == Some(player_id)
        || team_leader_from_prefix(event, "second_team") == Some(player_id)
        || fact_i32(event, "player_id") == Some(player_id)
        || fact_i32(event, "data1") == Some(player_id)
}

fn war_involves_player(event: &GameEvent, player_id: i32) -> bool {
    fact_i32(event, "declaring_team_leader_player_id") == Some(player_id)
        || fact_i32(event, "target_team_leader_player_id") == Some(player_id)
        || fact_i32(event, "player_id") == Some(player_id)
        || fact_i32(event, "data1") == Some(player_id)
}

fn quest_city_target_matches(quest: &LivingQuest, event: &GameEvent) -> bool {
    let Some(target) = quest.target.as_deref() else {
        return false;
    };
    fact_string(event, "city_name")
        .map(|city| normalized_token(&city) == normalized_token(target))
        .unwrap_or(false)
}

fn quest_target_civilization_matches(quest: &LivingQuest, event: &GameEvent) -> bool {
    let Some(target) = quest.target.as_deref() else {
        return true;
    };
    let target = normalized_token(target);
    involved_civilizations(event)
        .iter()
        .any(|civilization| normalized_token(civilization) == target)
}

fn quest_target_label(quest: &LivingQuest) -> String {
    quest
        .target
        .clone()
        .unwrap_or_else(|| "the realm".to_owned())
}

fn event_deed_label(event: &GameEvent) -> String {
    fact_string(event, "building_name")
        .or_else(|| fact_string(event, "project_name"))
        .or_else(|| fact_string(event, "tech_name"))
        .or_else(|| fact_string(event, "religion_name"))
        .or_else(|| fact_string(event, "great_person_name"))
        .or_else(|| fact_string(event, "unit_name"))
        .or_else(|| fact_string(event, "city_name"))
        .or_else(|| fact_string(event, "victory_name"))
        .unwrap_or_else(|| event.event_type.replace('_', " "))
}

fn normalized_token(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn event_player_id(event: &GameEvent) -> Option<i32> {
    fact_i32(event, "player_id")
        .or_else(|| fact_i32(event, "owner_id"))
        .or_else(|| fact_i32(event, "discoverer_id"))
        .or_else(|| fact_i32(event, "founder_id"))
}

fn event_player_civilization(event: &GameEvent) -> String {
    fact_string(event, "player_civilization")
        .or_else(|| fact_string(event, "owner_civilization"))
        .or_else(|| fact_string(event, "discoverer_civilization"))
        .or_else(|| fact_string(event, "founder_civilization"))
        .or_else(|| event_player_id(event).map(|player_id| format!("Player {player_id}")))
        .unwrap_or_else(|| "A civilization".to_owned())
}

fn event_id_token(event: &GameEvent) -> String {
    match event.facts.get("event_id") {
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => value.to_string(),
        None => format!("{}:{}", event.event_type, event.turn.unwrap_or(-1)),
    }
}

fn player_memory(player_id: Option<i32>, text: String) -> Option<CivilizationMemory> {
    let player_id = player_id?;
    (player_id >= 0).then_some(CivilizationMemory { player_id, text })
}

fn civilization_name(event: &GameEvent, civilization_key: &str, player_id_key: &str) -> String {
    fact_string(event, civilization_key)
        .or_else(|| fact_string(event, "player_civilization"))
        .or_else(|| fact_i32(event, player_id_key).map(|player_id| format!("Player {player_id}")))
        .unwrap_or_else(|| "A civilization".to_owned())
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
        | "era_transition" | "golden_age_started" | "great_person_born" | "tech_discovered" => {
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

fn era_label(era_id: i32) -> String {
    if era_id >= 0 {
        format!("Era {era_id}")
    } else {
        "an unknown era".to_owned()
    }
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

fn vecdeque_strings(values: &VecDeque<String>) -> Vec<String> {
    values.iter().cloned().collect()
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

    use super::{DirectorMemorySnapshot, DirectorState, QuestDecisionResponse};

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

    #[test]
    fn civilization_memory_enriches_later_events_for_same_civilization() {
        let mut director = DirectorState::default();
        director.observe_event(&event(
            "wonder_built",
            BTreeMap::from([
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("building_name".to_owned(), json!("The Oracle")),
            ]),
        ));

        let enriched = director.enrich_event(&event(
            "tech_discovered",
            BTreeMap::from([
                ("discoverer_id".to_owned(), json!(2)),
                ("discoverer_civilization".to_owned(), json!("Mali")),
                ("tech_name".to_owned(), json!("Writing")),
            ]),
        ));

        assert!(enriched
            .facts
            .get("civilization_memory")
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("Mali completed The Oracle."));
    }

    #[test]
    fn memory_snapshot_includes_civilization_memory() {
        let mut director = DirectorState::default();
        director.observe_event(&event(
            "wonder_built",
            BTreeMap::from([
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("building_name".to_owned(), json!("The Oracle")),
            ]),
        ));

        let snapshot = director.memory_snapshot();

        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.civilizations.len(), 1);
        assert_eq!(snapshot.civilizations[0].player_id, 2);
        assert!(snapshot.civilizations[0]
            .memories
            .iter()
            .any(|memory| memory == "Mali completed The Oracle."));
    }

    #[test]
    fn city_razing_creates_restoration_quest() {
        let mut director = DirectorState::default();
        let observation = director.observe_event(&event(
            "city_razed",
            BTreeMap::from([
                ("event_id".to_owned(), json!(77)),
                ("old_owner_id".to_owned(), json!(0)),
                ("old_owner_civilization".to_owned(), json!("Egypt")),
                ("razing_player_id".to_owned(), json!(1)),
                ("razing_player_civilization".to_owned(), json!("Rome")),
                ("city_name".to_owned(), json!("Memphis")),
                ("dynamic_quest_seed".to_owned(), json!("city_ruins_legacy")),
            ]),
        ));

        assert!(observation
            .quest_projections()
            .iter()
            .any(|quest| quest.contains("Quest: Remember Memphis")));
        assert_eq!(observation.quest_decisions().len(), 1);
        assert_eq!(observation.quest_decisions()[0].player_id, 0);
        assert_eq!(observation.quest_decisions()[0].choices.len(), 3);
        assert!(observation.quest_decisions()[0]
            .choices
            .contains(&"Restore what was lost".to_owned()));

        let snapshot = director.memory_snapshot();
        assert_eq!(snapshot.living_quests.len(), 1);
        assert_eq!(snapshot.living_quests[0].kind, "restoration");
        assert_eq!(snapshot.living_quests[0].target.as_deref(), Some("Memphis"));
        assert_eq!(snapshot.living_quests[0].status, "active");
        assert_eq!(snapshot.living_quests[0].objective.key, "regain_city");
        assert!(snapshot.living_quests[0].objective.text.contains("Memphis"));
        assert!(snapshot.living_quests[0].reward.contains("restoration"));
        assert!(snapshot.living_quests[0]
            .consequence
            .contains("unresolved grievance"));
    }

    #[test]
    fn restoration_quest_completes_when_city_is_retaken() {
        let mut director = DirectorState::default();
        director.observe_event(&event(
            "city_captured",
            BTreeMap::from([
                ("event_id".to_owned(), json!(77)),
                ("old_owner_id".to_owned(), json!(0)),
                ("old_owner_civilization".to_owned(), json!("Egypt")),
                ("new_owner_id".to_owned(), json!(1)),
                ("new_owner_civilization".to_owned(), json!("Rome")),
                ("city_name".to_owned(), json!("Memphis")),
                (
                    "dynamic_quest_seed".to_owned(),
                    json!("occupation_aftermath"),
                ),
            ]),
        ));

        let observation = director.observe_event(&event(
            "city_captured",
            BTreeMap::from([
                ("event_id".to_owned(), json!(88)),
                ("old_owner_id".to_owned(), json!(1)),
                ("old_owner_civilization".to_owned(), json!("Rome")),
                ("new_owner_id".to_owned(), json!(0)),
                ("new_owner_civilization".to_owned(), json!("Egypt")),
                ("city_name".to_owned(), json!("Memphis")),
            ]),
        ));

        assert!(observation
            .quest_projections()
            .iter()
            .any(|quest| quest.contains("completed")));
        assert_eq!(observation.quest_rewards().len(), 1);
        assert_eq!(observation.quest_rewards()[0].player_id, 0);
        assert_eq!(observation.quest_rewards()[0].reward_key, "gold");
        assert_eq!(observation.quest_rewards()[0].amount, 75);
        let snapshot = director.memory_snapshot();
        assert!(snapshot
            .living_quests
            .iter()
            .any(|quest| quest.kind == "restoration"
                && quest.status == "completed"
                && quest.objective.progress == quest.objective.required));
    }

    #[test]
    fn restoration_quest_reward_reflects_chosen_stance() {
        let mut director = DirectorState::default();
        let observation = director.observe_event(&event(
            "city_captured",
            BTreeMap::from([
                ("event_id".to_owned(), json!(77)),
                ("old_owner_id".to_owned(), json!(0)),
                ("old_owner_civilization".to_owned(), json!("Egypt")),
                ("new_owner_id".to_owned(), json!(1)),
                ("new_owner_civilization".to_owned(), json!("Rome")),
                ("city_name".to_owned(), json!("Memphis")),
                (
                    "dynamic_quest_seed".to_owned(),
                    json!("occupation_aftermath"),
                ),
            ]),
        ));
        let decision = observation.quest_decisions()[0].clone();
        director.apply_quest_decision_responses(&[QuestDecisionResponse {
            id: decision.id,
            player_id: 0,
            choice: "Swear vengeance".to_owned(),
        }]);

        let observation = director.observe_event(&event(
            "city_captured",
            BTreeMap::from([
                ("event_id".to_owned(), json!(88)),
                ("old_owner_id".to_owned(), json!(1)),
                ("old_owner_civilization".to_owned(), json!("Rome")),
                ("new_owner_id".to_owned(), json!(0)),
                ("new_owner_civilization".to_owned(), json!("Egypt")),
                ("city_name".to_owned(), json!("Memphis")),
            ]),
        ));

        assert!(observation
            .quest_projections()
            .iter()
            .any(|quest| quest.contains("Stance honored: Swear vengeance")));
        assert_eq!(observation.quest_rewards().len(), 1);
        assert_eq!(observation.quest_rewards()[0].amount, 100);
        assert!(observation.quest_rewards()[0]
            .text
            .contains("Stance: Swear vengeance"));
    }

    #[test]
    fn legacy_quest_completes_on_later_discovery() {
        let mut director = DirectorState::default();
        director.observe_event(&event(
            "wonder_built",
            BTreeMap::from([
                ("event_id".to_owned(), json!(12)),
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("building_name".to_owned(), json!("The Oracle")),
                ("dynamic_quest_seed".to_owned(), json!("wonder_legacy")),
            ]),
        ));

        let observation = director.observe_event(&event(
            "tech_discovered",
            BTreeMap::from([
                ("event_id".to_owned(), json!(13)),
                ("discoverer_id".to_owned(), json!(2)),
                ("discoverer_civilization".to_owned(), json!("Mali")),
                ("tech_name".to_owned(), json!("Writing")),
            ]),
        ));

        assert!(observation
            .quest_projections()
            .iter()
            .any(|quest| quest.contains("Quest: Make The Oracle Matter completed")));
        assert_eq!(observation.quest_rewards().len(), 1);
        assert_eq!(observation.quest_rewards()[0].player_id, 2);
        assert_eq!(observation.quest_rewards()[0].reward_key, "gold");

        let snapshot = director.memory_snapshot();
        assert!(snapshot
            .living_quests
            .iter()
            .any(|quest| quest.kind == "legacy" && quest.status == "completed"));
    }

    #[test]
    fn breakthrough_quest_completes_on_later_project() {
        let mut director = DirectorState::default();
        director.observe_event(&event(
            "tech_discovered",
            BTreeMap::from([
                ("event_id".to_owned(), json!(31)),
                ("discoverer_id".to_owned(), json!(2)),
                ("discoverer_civilization".to_owned(), json!("Mali")),
                ("tech_name".to_owned(), json!("Writing")),
                ("dynamic_quest_seed".to_owned(), json!("new_knowledge")),
            ]),
        ));

        let observation = director.observe_event(&event(
            "project_built",
            BTreeMap::from([
                ("event_id".to_owned(), json!(32)),
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("project_name".to_owned(), json!("Apollo Program")),
            ]),
        ));

        assert!(observation
            .quest_projections()
            .iter()
            .any(|quest| quest.contains("Quest: Apply Writing completed")));
        assert_eq!(observation.quest_rewards().len(), 1);
        assert_eq!(observation.quest_rewards()[0].player_id, 2);

        let snapshot = director.memory_snapshot();
        assert!(snapshot.living_quests.iter().any(|quest| {
            quest.kind == "breakthrough"
                && quest.status == "completed"
                && quest.objective.progress == quest.objective.required
        }));
    }

    #[test]
    fn war_aim_quest_completes_on_peace_with_target() {
        let mut director = DirectorState::default();
        director.observe_event(&event(
            "war_declared",
            BTreeMap::from([
                ("event_id".to_owned(), json!(41)),
                ("declaring_team_leader_player_id".to_owned(), json!(0)),
                ("target_team_leader_player_id".to_owned(), json!(1)),
                ("declaring_team_civilization".to_owned(), json!("Egypt")),
                ("target_team_civilization".to_owned(), json!("Rome")),
                ("dynamic_quest_seed".to_owned(), json!("war_aims")),
            ]),
        ));

        let observation = director.observe_event(&event(
            "peace_signed",
            BTreeMap::from([
                ("event_id".to_owned(), json!(42)),
                ("first_team_leader_player_id".to_owned(), json!(0)),
                ("second_team_leader_player_id".to_owned(), json!(1)),
                ("first_team_civilization".to_owned(), json!("Egypt")),
                ("second_team_civilization".to_owned(), json!("Rome")),
            ]),
        ));

        assert!(observation
            .quest_projections()
            .iter()
            .any(|quest| quest.contains("Quest: Name the War Aim completed")));
        assert!(observation
            .quest_rewards()
            .iter()
            .any(|reward| reward.player_id == 0
                && reward.reward_key == "gold"
                && reward.text.contains("Name the War Aim")));

        let snapshot = director.memory_snapshot();
        assert!(snapshot.living_quests.iter().any(|quest| {
            quest.kind == "war_aim" && quest.player_id == 0 && quest.status == "completed"
        }));
    }

    #[test]
    fn memory_snapshot_restores_living_quests() {
        let mut director = DirectorState::default();
        director.observe_event(&event(
            "wonder_built",
            BTreeMap::from([
                ("event_id".to_owned(), json!(12)),
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("building_name".to_owned(), json!("The Oracle")),
                ("dynamic_quest_seed".to_owned(), json!("wonder_legacy")),
            ]),
        ));

        let restored = DirectorState::from_memory_snapshot(director.memory_snapshot()).unwrap();
        let enriched = restored.enrich_event(&event(
            "tech_discovered",
            BTreeMap::from([
                ("discoverer_id".to_owned(), json!(2)),
                ("discoverer_civilization".to_owned(), json!("Mali")),
                ("tech_name".to_owned(), json!("Writing")),
            ]),
        ));

        assert!(enriched
            .facts
            .get("living_quests")
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("Make The Oracle Matter"));
    }

    #[test]
    fn quest_decision_response_updates_living_quest_memory() {
        let mut director = DirectorState::default();
        let observation = director.observe_event(&event(
            "city_razed",
            BTreeMap::from([
                ("event_id".to_owned(), json!(77)),
                ("old_owner_id".to_owned(), json!(0)),
                ("old_owner_civilization".to_owned(), json!("Egypt")),
                ("razing_player_id".to_owned(), json!(1)),
                ("razing_player_civilization".to_owned(), json!("Rome")),
                ("city_name".to_owned(), json!("Memphis")),
                ("dynamic_quest_seed".to_owned(), json!("city_ruins_legacy")),
            ]),
        ));
        let decision = observation.quest_decisions()[0].clone();

        let projections = director.apply_quest_decision_responses(&[QuestDecisionResponse {
            id: decision.id,
            player_id: 0,
            choice: "Swear vengeance".to_owned(),
        }]);

        assert_eq!(projections.len(), 1);
        let snapshot = director.memory_snapshot();
        assert_eq!(
            snapshot.living_quests[0]
                .decision
                .as_ref()
                .map(|decision| decision.choice.as_str()),
            Some("Swear vengeance")
        );
        assert!(snapshot.living_quests[0]
            .progress_notes
            .iter()
            .any(|note| note.contains("Swear vengeance")));

        let enriched = director.enrich_event(&event(
            "city_captured",
            BTreeMap::from([
                ("new_owner_id".to_owned(), json!(0)),
                ("new_owner_civilization".to_owned(), json!("Egypt")),
                ("city_name".to_owned(), json!("Memphis")),
            ]),
        ));
        assert!(enriched
            .facts
            .get("living_quests")
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("Stance: Swear vengeance"));
    }

    #[test]
    fn old_memory_snapshot_restores_living_quest_objective_defaults() {
        let snapshot: DirectorMemorySnapshot = serde_json::from_value(json!({
            "version": 1,
            "recent_world_events": [],
            "world_arc": null,
            "civilizations": [],
            "relationships": [],
            "living_quests": [{
                "id": "old:0:restoration:memphis",
                "kind": "restoration",
                "player_id": 0,
                "civilization": "Egypt",
                "title": "Remember Memphis",
                "prompt": "Egypt must decide what Memphis means.",
                "origin_event_type": "city_razed",
                "origin_turn": 12,
                "target": "Memphis",
                "status": "active",
                "progress_notes": []
            }],
            "active_conflicts": [],
            "recent_conflicts": []
        }))
        .unwrap();

        let restored = DirectorState::from_memory_snapshot(snapshot).unwrap();
        let quest = &restored.memory_snapshot().living_quests[0];

        assert_eq!(quest.objective.key, "regain_city");
        assert!(quest.objective.text.contains("Memphis"));
        assert!(quest.reward.contains("restoration"));
        assert!(quest.consequence.contains("unresolved grievance"));
    }

    #[test]
    fn memory_snapshot_restores_director_context() {
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
            let title = if proposal.request().name_kind == "war" {
                "The Nile Iron War"
            } else {
                "The Iron Mandate"
            };
            director.apply_historical_name(proposal, title.to_owned());
        }

        let restored = DirectorState::from_memory_snapshot(director.memory_snapshot()).unwrap();
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
        let enriched = restored.enrich_diplomacy_request(&request);

        assert!(enriched
            .diplomacy_memory
            .unwrap()
            .contains("Active named conflict: The Nile Iron War"));
        assert!(enriched.world_arc.unwrap().contains("The Iron Mandate"));
    }

    #[test]
    fn memory_snapshot_rejects_unknown_versions() {
        let director = DirectorState::default();
        let mut snapshot = director.memory_snapshot();
        snapshot.version = 99;

        assert!(DirectorState::from_memory_snapshot(snapshot).is_err());
    }

    #[test]
    fn diplomacy_context_includes_civilization_memory() {
        let mut director = DirectorState::default();
        director.observe_event(&event(
            "city_razed",
            BTreeMap::from([
                ("old_owner_id".to_owned(), json!(0)),
                ("old_owner_civilization".to_owned(), json!("Egypt")),
                ("razing_player_id".to_owned(), json!(1)),
                ("razing_player_civilization".to_owned(), json!("Rome")),
                ("city_name".to_owned(), json!("Memphis")),
            ]),
        ));

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

        let context = director
            .enrich_diplomacy_request(&request)
            .world_arc
            .unwrap();

        assert!(context.contains("Active player civilization memory"));
        assert!(context.contains("Egypt remembers the destruction of Memphis."));
        assert!(context.contains("Rival leader civilization memory"));
        assert!(context.contains("Rome razed Memphis."));
    }

    #[test]
    fn tech_discovery_can_emit_era_transition_event() {
        let mut director = DirectorState::default();
        assert!(director
            .observe_event(&event(
                "tech_discovered",
                BTreeMap::from([
                    ("discoverer_id".to_owned(), json!(2)),
                    ("discoverer_civilization".to_owned(), json!("Mali")),
                    ("tech_name".to_owned(), json!("Pottery")),
                    ("tech_era_id".to_owned(), json!(0)),
                    ("tech_era_name".to_owned(), json!("Ancient Era")),
                    ("era_id".to_owned(), json!(0)),
                    ("era_name".to_owned(), json!("Ancient Era")),
                ]),
            ))
            .era_transition()
            .is_none());

        let observation = director.observe_event(&event(
            "tech_discovered",
            BTreeMap::from([
                ("discoverer_id".to_owned(), json!(2)),
                ("discoverer_civilization".to_owned(), json!("Mali")),
                ("tech_name".to_owned(), json!("Writing")),
                ("tech_era_id".to_owned(), json!(1)),
                ("tech_era_name".to_owned(), json!("Classical Era")),
                ("era_id".to_owned(), json!(0)),
                ("era_name".to_owned(), json!("Ancient Era")),
            ]),
        ));
        let era_event = observation.era_transition().unwrap();

        assert_eq!(era_event.event_type, "era_transition");
        assert_eq!(
            era_event
                .facts
                .get("old_era_name")
                .and_then(|value| value.as_str()),
            Some("Ancient Era")
        );
        assert_eq!(
            era_event
                .facts
                .get("new_era_name")
                .and_then(|value| value.as_str()),
            Some("Classical Era")
        );

        let enriched = director.enrich_event(era_event);
        assert!(enriched
            .facts
            .get("civilization_era_memory")
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("Mali entered the Classical Era after Writing."));
    }
}
