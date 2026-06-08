#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::{BTreeMap, BTreeSet};

use ages_beyond_protocol::GameEvent;
use anyhow::Context;
use civ4::BridgeClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::director::{
    DirectorMemorySnapshot, DirectorState, QuestDecisionPrompt, QuestDecisionResponse,
    QuestRewardCommand,
};

const SAVE_STATE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgesBeyondSaveState {
    pub schema_version: u16,
    pub director: DirectorMemorySnapshot,
    #[serde(default)]
    pub seen_event_ids: BTreeSet<String>,
    #[serde(default)]
    pub pending_decisions: BTreeMap<String, QuestDecisionPrompt>,
    #[serde(default)]
    pub chosen_decisions: BTreeMap<String, QuestDecisionResponse>,
    #[serde(default)]
    pub pending_rewards: BTreeMap<String, QuestRewardCommand>,
    #[serde(default)]
    pub applied_reward_ids: BTreeSet<String>,
}

impl Default for AgesBeyondSaveState {
    fn default() -> Self {
        Self {
            schema_version: SAVE_STATE_SCHEMA_VERSION,
            director: DirectorState::default().memory_snapshot(),
            seen_event_ids: BTreeSet::new(),
            pending_decisions: BTreeMap::new(),
            chosen_decisions: BTreeMap::new(),
            pending_rewards: BTreeMap::new(),
            applied_reward_ids: BTreeSet::new(),
        }
    }
}

impl AgesBeyondSaveState {
    pub fn load_from_bridge(client: &mut BridgeClient) -> anyhow::Result<Option<Self>> {
        let Some(state) = client
            .load_mod_state::<AgesBeyondSaveState>()
            .context("failed to load Ages Beyond save state from bridge")?
        else {
            return Ok(None);
        };

        state.validate()?;
        Ok(Some(state))
    }

    pub fn save_to_bridge(&self, client: &mut BridgeClient) -> anyhow::Result<usize> {
        self.validate()?;
        client
            .save_mod_state(self)
            .context("failed to save Ages Beyond state through bridge")
    }

    pub fn restore_director(&self) -> anyhow::Result<DirectorState> {
        DirectorState::from_memory_snapshot(self.director.clone())
            .map_err(anyhow::Error::msg)
            .context("failed to restore director from Ages Beyond save state")
    }

    pub fn refresh_director(&mut self, director: &DirectorState) {
        self.director = director.memory_snapshot();
    }

    pub fn is_event_seen(&self, event: &GameEvent) -> bool {
        event_key(event)
            .as_ref()
            .is_some_and(|key| self.seen_event_ids.contains(key))
    }

    pub fn mark_event_seen(&mut self, event: &GameEvent) -> bool {
        let Some(key) = event_key(event) else {
            return false;
        };
        self.seen_event_ids.insert(key)
    }

    pub fn record_pending_decisions(&mut self, decisions: &[QuestDecisionPrompt]) -> bool {
        let mut changed = false;
        for decision in decisions {
            if decision.id.trim().is_empty() || self.chosen_decisions.contains_key(&decision.id) {
                continue;
            }

            changed |= self
                .pending_decisions
                .insert(decision.id.clone(), decision.clone())
                .is_none();
        }
        changed
    }

    pub fn apply_decision_responses(
        &mut self,
        responses: &[QuestDecisionResponse],
    ) -> (Vec<QuestDecisionResponse>, bool) {
        let mut new_responses = Vec::new();
        let mut changed = false;

        for response in responses {
            if response.id.trim().is_empty() || response.choice.trim().is_empty() {
                continue;
            }
            if self.chosen_decisions.contains_key(&response.id) {
                continue;
            }

            self.pending_decisions.remove(&response.id);
            self.chosen_decisions
                .insert(response.id.clone(), response.clone());
            new_responses.push(response.clone());
            changed = true;
        }

        (new_responses, changed)
    }

    pub fn record_pending_rewards(&mut self, rewards: &[QuestRewardCommand]) -> bool {
        let mut changed = false;
        for reward in rewards {
            if reward.id.trim().is_empty() || self.applied_reward_ids.contains(&reward.id) {
                continue;
            }

            changed |= self
                .pending_rewards
                .insert(reward.id.clone(), reward.clone())
                .is_none();
        }
        changed
    }

    pub fn pending_decisions(&self) -> impl Iterator<Item = &QuestDecisionPrompt> {
        self.pending_decisions.values()
    }

    pub fn pending_rewards_to_apply(&self) -> Vec<QuestRewardCommand> {
        self.pending_rewards.values().cloned().collect()
    }

    pub fn mark_reward_applied(&mut self, reward_id: &str) -> bool {
        let reward_id = reward_id.trim();
        if reward_id.is_empty() {
            return false;
        }

        self.pending_rewards.remove(reward_id);
        self.applied_reward_ids.insert(reward_id.to_owned())
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.schema_version != SAVE_STATE_SCHEMA_VERSION {
            anyhow::bail!(
                "unsupported Ages Beyond save state schema {}",
                self.schema_version
            );
        }
        Ok(())
    }
}

pub fn event_key(event: &GameEvent) -> Option<String> {
    match event.facts.get("event_id") {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.trim().to_owned()),
        Some(Value::Number(value)) => Some(value.to_string()),
        Some(value) => Some(value.to_string()),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;

    fn event(id: Value) -> GameEvent {
        GameEvent {
            event_type: "city_founded".to_owned(),
            turn: Some(1),
            actors: Vec::new(),
            summary: None,
            facts: BTreeMap::from([("event_id".to_owned(), id)]),
        }
    }

    #[test]
    fn event_seen_state_uses_save_event_id() {
        let mut state = AgesBeyondSaveState::default();
        let event = event(json!("bridge:7"));

        assert!(!state.is_event_seen(&event));
        assert!(state.mark_event_seen(&event));
        assert!(state.is_event_seen(&event));
        assert!(!state.mark_event_seen(&event));
    }

    #[test]
    fn decision_responses_are_applied_once() {
        let mut state = AgesBeyondSaveState::default();
        let response = QuestDecisionResponse {
            id: "quest:1".to_owned(),
            player_id: 0,
            choice: "Restore it".to_owned(),
        };

        let (new_responses, changed) = state.apply_decision_responses(&[response.clone()]);
        assert!(changed);
        assert_eq!(new_responses.len(), 1);
        assert!(state.pending_decisions.is_empty());

        let (new_responses, changed) = state.apply_decision_responses(&[response]);
        assert!(!changed);
        assert!(new_responses.is_empty());
    }

    #[test]
    fn applied_rewards_clear_pending_reward() {
        let mut state = AgesBeyondSaveState::default();
        let reward = QuestRewardCommand {
            id: "reward:1".to_owned(),
            player_id: 0,
            reward_key: "gold".to_owned(),
            amount: 50,
            text: "Reward".to_owned(),
        };

        assert!(state.record_pending_rewards(&[reward]));
        assert!(state.pending_rewards.contains_key("reward:1"));
        assert!(state.mark_reward_applied("reward:1"));
        assert!(!state.pending_rewards.contains_key("reward:1"));
        assert!(state.applied_reward_ids.contains("reward:1"));
    }
}
