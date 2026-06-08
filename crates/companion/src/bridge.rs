use std::collections::BTreeMap;

use ages_beyond_protocol::GameEvent;
use anyhow::Context;
use civ4::{BridgeCallbackMessage, BridgeClient, BridgeEvent, CityRef, InfoKind, Plot, TeamId};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::chronicle::ChronicleWriter;
use crate::director::DirectorState;
use crate::events;
use crate::llm::LlmClient;
use crate::memory::{
    MemoryWriter, QuestDecisionResponseReader, QuestJournalWriter, QuestLogWriter,
};
use crate::notifications::{NotificationWriter, QuestDecisionWriter, QuestRewardWriter};

pub async fn run_client<L>(
    llm: L,
    chronicle: Option<ChronicleWriter>,
    notifications: Option<NotificationWriter>,
    quest_notifications: Option<NotificationWriter>,
    quest_decisions: Option<QuestDecisionWriter>,
    quest_decision_responses: Option<QuestDecisionResponseReader>,
    quest_rewards: Option<QuestRewardWriter>,
    memory: Option<MemoryWriter>,
    quest_log: Option<QuestLogWriter>,
    quest_journal: Option<QuestJournalWriter>,
    initial_director: DirectorState,
) -> anyhow::Result<()>
where
    L: LlmClient,
{
    let (mut client, hello) = BridgeClient::connect_from_env_with_handshake()
        .context("failed to connect to CvGameCoreDLL bridge")?;
    let missing = hello.missing_capabilities(&["callbacks", "queries"]);
    if !missing.is_empty() {
        anyhow::bail!("bridge is missing capabilities: {}", missing.join(", "));
    }

    info!(
        protocol = hello.protocol,
        capabilities = ?hello.capabilities,
        "connected to CvGameCoreDLL bridge"
    );

    let director = Mutex::new(initial_director);

    loop {
        let callback = tokio::task::block_in_place(|| client.next_callback_message())
            .context("failed to read bridge callback")?;

        if let Some(request_id) = callback.request_id() {
            client
                .write_callback_success(request_id, &json!({ "consume": false }))
                .context("failed to write bridge callback reply")?;
            continue;
        }

        let callback_id = callback_id(&callback);
        let bridge_event = callback.event().clone();
        let Some(event) = tokio::task::block_in_place(|| {
            bridge_event_to_game_event(&mut client, &bridge_event, callback_id)
        })
        .context("failed to adapt bridge callback")?
        else {
            debug!(event = bridge_event.name(), "ignored bridge callback");
            continue;
        };

        let applied = events::apply_quest_decision_responses(
            quest_decision_responses.as_ref(),
            memory.as_ref(),
            quest_log.as_ref(),
            quest_journal.as_ref(),
            &director,
        )
        .await?;
        if !applied.is_empty() {
            debug!(
                count = applied.len(),
                "applied quest decision responses before bridge event"
            );
        }

        match events::process_game_event(
            &event,
            &llm,
            chronicle.as_ref(),
            notifications.as_ref(),
            quest_notifications.as_ref(),
            quest_decisions.as_ref(),
            quest_rewards.as_ref(),
            memory.as_ref(),
            quest_log.as_ref(),
            quest_journal.as_ref(),
            &director,
        )
        .await
        {
            Ok(_) => {}
            Err(err) => {
                warn!(
                    event_type = %event.event_type,
                    error = %err,
                    "failed to process bridge event"
                );
            }
        }
    }
}

fn callback_id(callback: &BridgeCallbackMessage) -> Value {
    match callback {
        BridgeCallbackMessage::Mirror(message) => json!(format!("bridge:{}", message.seq)),
        BridgeCallbackMessage::Request(request) => json!(format!("bridge-request:{}", request.id)),
    }
}

fn bridge_event_to_game_event(
    client: &mut BridgeClient,
    event: &BridgeEvent,
    event_id: Value,
) -> anyhow::Result<Option<GameEvent>> {
    let turn = client.get_game_turn().ok();
    let game_state = client.get_game_state().ok();
    let mut facts = BTreeMap::new();
    facts.insert("contract_version".to_owned(), json!(4));
    facts.insert("event_id".to_owned(), event_id);
    facts.insert("bridge_event".to_owned(), json!(event.name()));
    facts.insert("known_to_active_player".to_owned(), json!(true));
    facts.insert("location_known_to_active_player".to_owned(), json!(true));
    facts.insert("is_global_announcement".to_owned(), json!(true));
    facts.insert("involves_active_player".to_owned(), json!(true));
    facts.insert("involves_active_team".to_owned(), json!(true));
    facts.insert("plot_visibility".to_owned(), json!("known"));

    if let Some(state) = &game_state {
        facts.insert("active_player_id".to_owned(), json!(state.active_player));
        facts.insert("active_team_id".to_owned(), json!(state.active_team));
        facts.insert("current_era_id".to_owned(), json!(state.current_era));
        facts.insert("max_civ_players".to_owned(), json!(18));
        facts.insert("barbarian_team_id".to_owned(), json!(18));
    }

    let event = match event {
        BridgeEvent::GameStart => game_event("game_started", turn, None, facts),
        BridgeEvent::CityBuilt { city, plot } => {
            insert_city(&mut facts, *city);
            insert_plot(&mut facts, *plot);
            facts.insert("player_id".to_owned(), json!(city.player));
            facts.insert("city_name".to_owned(), json!(city_label(*city)));
            facts.insert(
                "dynamic_quest_seed".to_owned(),
                json!("settlement_identity"),
            );
            game_event(
                "city_founded",
                turn,
                Some(format!(
                    "Player {} founded {}.",
                    city.player,
                    city_label(*city)
                )),
                facts,
            )
        }
        BridgeEvent::CityRazed {
            city,
            razed_by,
            plot,
        } => {
            insert_city(&mut facts, *city);
            insert_plot(&mut facts, *plot);
            facts.insert("old_owner_id".to_owned(), json!(city.player));
            facts.insert("player_id".to_owned(), json!(razed_by.0));
            facts.insert("razing_player_id".to_owned(), json!(razed_by.0));
            facts.insert("city_name".to_owned(), json!(city_label(*city)));
            facts.insert("dynamic_quest_seed".to_owned(), json!("city_ruins_legacy"));
            game_event(
                "city_razed",
                turn,
                Some(format!("{} was razed.", city_label(*city))),
                facts,
            )
        }
        BridgeEvent::CityAcquired {
            old_player,
            city,
            conquest,
            trade,
            plot,
        } => {
            insert_city(&mut facts, *city);
            insert_plot(&mut facts, *plot);
            facts.insert("old_owner_id".to_owned(), json!(old_player.0));
            facts.insert("new_owner_id".to_owned(), json!(city.player));
            facts.insert("player_id".to_owned(), json!(city.player));
            facts.insert("city_name".to_owned(), json!(city_label(*city)));
            facts.insert("conquest".to_owned(), json!(*conquest));
            facts.insert("trade".to_owned(), json!(*trade));
            facts.insert(
                "dynamic_quest_seed".to_owned(),
                json!(if *conquest {
                    "occupation_aftermath"
                } else {
                    "city_transition"
                }),
            );
            game_event(
                if *conquest {
                    "city_captured"
                } else {
                    "city_acquired"
                },
                turn,
                Some(format!(
                    "Player {} acquired {} from player {}.",
                    city.player,
                    city_label(*city),
                    old_player.0
                )),
                facts,
            )
        }
        BridgeEvent::BuildingBuilt { city, building } => {
            insert_city(&mut facts, *city);
            facts.insert("player_id".to_owned(), json!(city.player));
            facts.insert("building_id".to_owned(), json!(*building));
            facts.insert(
                "building_name".to_owned(),
                json!(info_label(
                    client,
                    InfoKind::Building,
                    *building,
                    "Building"
                )),
            );
            facts.insert("dynamic_quest_seed".to_owned(), json!("wonder_legacy"));
            game_event("wonder_built", turn, None, facts)
        }
        BridgeEvent::ProjectBuilt { city, project } => {
            insert_city(&mut facts, *city);
            facts.insert("player_id".to_owned(), json!(city.player));
            facts.insert("project_id".to_owned(), json!(*project));
            facts.insert(
                "project_name".to_owned(),
                json!(info_label(client, InfoKind::Project, *project, "Project")),
            );
            facts.insert(
                "dynamic_quest_seed".to_owned(),
                json!("great_project_consequences"),
            );
            game_event("project_built", turn, None, facts)
        }
        BridgeEvent::TechAcquired {
            team, player, tech, ..
        } => {
            facts.insert("team_id".to_owned(), json!(team.0));
            facts.insert("player_id".to_owned(), json!(player.0));
            facts.insert("discoverer_id".to_owned(), json!(player.0));
            facts.insert("tech_id".to_owned(), json!(*tech));
            facts.insert(
                "tech_name".to_owned(),
                json!(info_label(client, InfoKind::Tech, *tech, "Tech")),
            );
            facts.insert("dynamic_quest_seed".to_owned(), json!("new_knowledge"));
            game_event("tech_discovered", turn, None, facts)
        }
        BridgeEvent::ReligionFounded { player, religion } => {
            facts.insert("player_id".to_owned(), json!(player.0));
            facts.insert("founder_id".to_owned(), json!(player.0));
            facts.insert("religion_id".to_owned(), json!(*religion));
            facts.insert(
                "religion_name".to_owned(),
                json!(info_label(
                    client,
                    InfoKind::Religion,
                    *religion,
                    "Religion"
                )),
            );
            facts.insert("dynamic_quest_seed".to_owned(), json!("holy_city"));
            game_event("religion_founded", turn, None, facts)
        }
        BridgeEvent::GoldenAge { player } => {
            facts.insert("player_id".to_owned(), json!(player.0));
            facts.insert("dynamic_quest_seed".to_owned(), json!("golden_age_mandate"));
            game_event("golden_age_started", turn, None, facts)
        }
        BridgeEvent::GreatPersonBorn {
            player,
            city,
            unit,
            unit_type,
            plot,
        } => {
            insert_city(&mut facts, *city);
            insert_plot(&mut facts, *plot);
            facts.insert("player_id".to_owned(), json!(player.0));
            facts.insert("unit_id".to_owned(), json!(unit.id));
            facts.insert("unit_type_id".to_owned(), json!(*unit_type));
            facts.insert(
                "great_person_name".to_owned(),
                json!(info_label(client, InfoKind::Unit, *unit_type, "Unit")),
            );
            facts.insert(
                "dynamic_quest_seed".to_owned(),
                json!("great_person_legacy"),
            );
            game_event("great_person_born", turn, None, facts)
        }
        BridgeEvent::ChangeWar {
            war,
            team,
            other_team,
        } => {
            insert_team_pair(client, &mut facts, *team, *other_team);
            game_event(
                if *war { "war_declared" } else { "peace_signed" },
                turn,
                None,
                facts,
            )
        }
        BridgeEvent::Victory { team, victory } => {
            facts.insert("team_id".to_owned(), json!(team.0));
            facts.insert("victory_id".to_owned(), json!(*victory));
            facts.insert(
                "victory_name".to_owned(),
                json!(info_label(client, InfoKind::Victory, *victory, "Victory")),
            );
            game_event("victory", turn, None, facts)
        }
        _ => return Ok(None),
    };

    Ok(Some(event))
}

fn game_event(
    event_type: impl Into<String>,
    turn: Option<i32>,
    summary: Option<String>,
    facts: BTreeMap<String, Value>,
) -> GameEvent {
    GameEvent {
        event_type: event_type.into(),
        turn,
        actors: Vec::new(),
        summary,
        facts,
    }
}

fn insert_city(facts: &mut BTreeMap<String, Value>, city: CityRef) {
    facts.insert("player_id".to_owned(), json!(city.player));
    facts.insert("city_id".to_owned(), json!(city.id));
}

fn insert_plot(facts: &mut BTreeMap<String, Value>, plot: Plot) {
    facts.insert("x".to_owned(), json!(plot.x));
    facts.insert("y".to_owned(), json!(plot.y));
}

fn insert_team_pair(
    client: &mut BridgeClient,
    facts: &mut BTreeMap<String, Value>,
    team: TeamId,
    other_team: TeamId,
) {
    facts.insert("team_id".to_owned(), json!(team.0));
    facts.insert("data1".to_owned(), json!(other_team.0));
    facts.insert("first_team_id".to_owned(), json!(team.0));
    facts.insert("second_team_id".to_owned(), json!(other_team.0));
    facts.insert("declaring_team_id".to_owned(), json!(team.0));
    facts.insert("target_team_id".to_owned(), json!(other_team.0));

    if let Ok(state) = client.get_team_state(team) {
        insert_team_leader(client, facts, "first_team", state.leader);
        insert_team_leader(client, facts, "declaring_team", state.leader);
    }
    if let Ok(state) = client.get_team_state(other_team) {
        insert_team_leader(client, facts, "second_team", state.leader);
        insert_team_leader(client, facts, "target_team", state.leader);
    }
}

fn insert_team_leader(
    client: &mut BridgeClient,
    facts: &mut BTreeMap<String, Value>,
    prefix: &str,
    player: i32,
) {
    facts.insert(format!("{prefix}_leader_player_id"), json!(player));
    facts.insert(
        format!("{prefix}_civilization"),
        json!(player_civilization_label(client, player)),
    );
}

fn city_label(city: CityRef) -> String {
    format!("City {}", city.id)
}

fn player_civilization_label(client: &mut BridgeClient, player: i32) -> String {
    client
        .get_player_state(player)
        .ok()
        .map(|state| info_label(client, InfoKind::Civilization, state.civilization, "Player"))
        .unwrap_or_else(|| format!("Player {player}"))
}

fn info_label(client: &mut BridgeClient, kind: InfoKind, id: i32, fallback_prefix: &str) -> String {
    client
        .get_info_type(kind, id)
        .ok()
        .map(|info| display_info_type(&info.type_name))
        .unwrap_or_else(|| format!("{fallback_prefix} {id}"))
}

fn display_info_type(type_name: &str) -> String {
    let without_prefix = type_name
        .split_once('_')
        .map(|(_, rest)| rest)
        .unwrap_or(type_name);
    without_prefix
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_ascii_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use civ4::PlayerId;

    #[test]
    fn adapts_city_acquired_to_capture_event() {
        let event = bridge_event_without_queries(
            BridgeEvent::CityAcquired {
                old_player: PlayerId(1),
                city: CityRef::new(0, 7),
                conquest: true,
                trade: false,
                plot: Plot::new(10, 12),
            },
            json!("bridge:1"),
        )
        .unwrap();

        assert_eq!(event.event_type, "city_captured");
        assert_eq!(event.facts.get("old_owner_id"), Some(&json!(1)));
        assert_eq!(event.facts.get("new_owner_id"), Some(&json!(0)));
        assert_eq!(
            event.facts.get("dynamic_quest_seed"),
            Some(&json!("occupation_aftermath"))
        );
    }

    #[test]
    fn adapts_tech_acquired_to_discovery_event() {
        let event = bridge_event_without_queries(
            BridgeEvent::TechAcquired {
                team: TeamId(2),
                player: PlayerId(3),
                tech: 11,
                announce: true,
            },
            json!("bridge:2"),
        )
        .unwrap();

        assert_eq!(event.event_type, "tech_discovered");
        assert_eq!(event.facts.get("discoverer_id"), Some(&json!(3)));
        assert_eq!(event.facts.get("tech_name"), Some(&json!("Tech 11")));
    }

    fn bridge_event_without_queries(event: BridgeEvent, event_id: Value) -> Option<GameEvent> {
        let mut facts = BTreeMap::new();
        facts.insert("contract_version".to_owned(), json!(4));
        facts.insert("event_id".to_owned(), event_id);
        facts.insert("bridge_event".to_owned(), json!(event.name()));
        match event {
            BridgeEvent::CityAcquired {
                old_player,
                city,
                conquest,
                trade,
                plot,
            } => {
                insert_city(&mut facts, city);
                insert_plot(&mut facts, plot);
                facts.insert("old_owner_id".to_owned(), json!(old_player.0));
                facts.insert("new_owner_id".to_owned(), json!(city.player));
                facts.insert("city_name".to_owned(), json!(city_label(city)));
                facts.insert("conquest".to_owned(), json!(conquest));
                facts.insert("trade".to_owned(), json!(trade));
                facts.insert(
                    "dynamic_quest_seed".to_owned(),
                    json!("occupation_aftermath"),
                );
                Some(game_event("city_captured", None, None, facts))
            }
            BridgeEvent::TechAcquired {
                team, player, tech, ..
            } => {
                facts.insert("team_id".to_owned(), json!(team.0));
                facts.insert("player_id".to_owned(), json!(player.0));
                facts.insert("discoverer_id".to_owned(), json!(player.0));
                facts.insert("tech_id".to_owned(), json!(tech));
                facts.insert("tech_name".to_owned(), json!(format!("Tech {tech}")));
                facts.insert("dynamic_quest_seed".to_owned(), json!("new_knowledge"));
                Some(game_event("tech_discovered", None, None, facts))
            }
            _ => None,
        }
    }
}
