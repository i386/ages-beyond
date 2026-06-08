use std::{collections::BTreeMap, sync::Arc, thread, time::Duration};

use ages_beyond_protocol::{DiplomacyTextRequest, GameEvent, RequestBody};
use anyhow::Context;
use civ4::{
    BridgeCallbackMessage, BridgeCallbackReader, BridgeClient, BridgeEvent, CityRef, InfoKind,
    Plot, TeamId,
};
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::chronicle::ChronicleWriter;
use crate::director::DirectorState;
use crate::events;
use crate::llm::LlmClient;
use crate::memory::{
    MemoryWriter, QuestDecisionResponseReader, QuestJournalWriter, QuestLogWriter,
};
use crate::notifications::{NotificationWriter, QuestDecisionWriter};
use crate::save_state::AgesBeyondSaveState;

const UI_TEXT_LLM_TIMEOUT: Duration = Duration::from_millis(1600);

pub async fn run_client<L>(
    llm: L,
    chronicle: Option<ChronicleWriter>,
    notifications: Option<NotificationWriter>,
    quest_notifications: Option<NotificationWriter>,
    quest_decisions: Option<QuestDecisionWriter>,
    quest_decision_responses: Option<QuestDecisionResponseReader>,
    memory: Option<MemoryWriter>,
    quest_log: Option<QuestLogWriter>,
    quest_journal: Option<QuestJournalWriter>,
) -> anyhow::Result<()>
where
    L: LlmClient,
{
    let (mut client, hello) = BridgeClient::connect_default_with_handshake()
        .context("failed to connect to CvGameCoreDLL bridge")?;
    let missing = hello.missing_capabilities(&["callbacks", "queries", "mod_state"]);
    if !missing.is_empty() {
        anyhow::bail!("bridge is missing capabilities: {}", missing.join(", "));
    }

    info!(
        protocol = hello.protocol,
        capabilities = ?hello.capabilities,
        "connected to CvGameCoreDLL bridge"
    );

    let save_state =
        match tokio::task::block_in_place(|| AgesBeyondSaveState::load_from_bridge(&mut client)) {
            Ok(Some(state)) => {
                info!("restored Ages Beyond state from Civ4 save");
                state
            }
            Ok(None) => AgesBeyondSaveState::default(),
            Err(err) => {
                warn!(error = %err, "failed to restore Ages Beyond save state; starting clean");
                AgesBeyondSaveState::default()
            }
        };

    let initial_director = match save_state.restore_director() {
        Ok(director) => director,
        Err(err) => {
            warn!(error = %err, "failed to restore director from save state; starting clean");
            DirectorState::default()
        }
    };
    let save_state = Arc::new(Mutex::new(save_state));
    let director = Arc::new(Mutex::new(initial_director));
    let (startup_decisions, startup_decisions_changed) = {
        let mut save_state = save_state.lock().await;
        events::apply_quest_decision_responses(
            quest_decision_responses.as_ref(),
            memory.as_ref(),
            quest_log.as_ref(),
            quest_journal.as_ref(),
            &director,
            &mut save_state,
        )
        .await?
    };
    if !startup_decisions.is_empty() {
        debug!(
            count = startup_decisions.len(),
            "applied saved quest decision responses at startup"
        );
    }
    let startup_rewards_changed = apply_pending_rewards(&mut client, &save_state).await?;
    events::write_director_outputs(
        memory.as_ref(),
        quest_log.as_ref(),
        quest_journal.as_ref(),
        &director,
    )
    .await?;
    write_pending_outputs(quest_decisions.as_ref(), &save_state).await?;
    if startup_decisions_changed || startup_rewards_changed {
        debug!("persisting startup quest response state");
    }
    persist_save_state(&mut client, &save_state, &director).await?;

    let callback_reader = client
        .try_clone_callback_reader()
        .context("failed to clone bridge callback reader")?;
    let mut callback_rx = spawn_callback_reader(callback_reader);
    let (event_tx, mut event_result_rx) = spawn_event_processor(
        llm.clone(),
        chronicle.clone(),
        notifications.clone(),
        quest_notifications.clone(),
        quest_decisions.clone(),
        memory.clone(),
        quest_log.clone(),
        quest_journal.clone(),
        Arc::clone(&director),
        Arc::clone(&save_state),
    );

    for pending in save_state.lock().await.pending_event_jobs_to_process() {
        event_tx
            .send(pending.event)
            .await
            .context("failed to enqueue restored Ages Beyond event job")?;
    }

    loop {
        tokio::select! {
            Some(callback_result) = callback_rx.recv() => {
                let callback = callback_result.context("failed to read bridge callback")?;
                handle_callback(
                    callback,
                    &mut client,
                    &event_tx,
                    quest_decision_responses.as_ref(),
                    quest_decisions.as_ref(),
                    memory.as_ref(),
                    quest_log.as_ref(),
                    quest_journal.as_ref(),
                    &llm,
                    &director,
                    &save_state,
                )
                .await?;
            }
            Some(result) = event_result_rx.recv() => {
                if result.changed {
                    apply_pending_rewards(&mut client, &save_state).await?;
                    persist_save_state(&mut client, &save_state, &director).await?;
                }
            }
            else => anyhow::bail!("bridge callback and event worker channels closed"),
        }
    }
}

struct EventProcessingResult {
    changed: bool,
}

fn spawn_callback_reader(
    mut callback_reader: BridgeCallbackReader,
) -> mpsc::UnboundedReceiver<anyhow::Result<BridgeCallbackMessage>> {
    let (tx, rx) = mpsc::unbounded_channel();
    thread::spawn(move || loop {
        let result = callback_reader
            .next_callback_message()
            .map_err(anyhow::Error::from);
        let should_stop = result.is_err();
        if tx.send(result).is_err() || should_stop {
            break;
        }
    });
    rx
}

fn spawn_event_processor<L>(
    llm: L,
    chronicle: Option<ChronicleWriter>,
    notifications: Option<NotificationWriter>,
    quest_notifications: Option<NotificationWriter>,
    quest_decisions: Option<QuestDecisionWriter>,
    memory: Option<MemoryWriter>,
    quest_log: Option<QuestLogWriter>,
    quest_journal: Option<QuestJournalWriter>,
    director: Arc<Mutex<DirectorState>>,
    save_state: Arc<Mutex<AgesBeyondSaveState>>,
) -> (
    mpsc::Sender<GameEvent>,
    mpsc::Receiver<EventProcessingResult>,
)
where
    L: LlmClient,
{
    let (event_tx, mut event_rx) = mpsc::channel::<GameEvent>(64);
    let (result_tx, result_rx) = mpsc::channel::<EventProcessingResult>(64);

    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match events::process_game_event(
                &event,
                &llm,
                chronicle.as_ref(),
                notifications.as_ref(),
                quest_notifications.as_ref(),
                quest_decisions.as_ref(),
                memory.as_ref(),
                quest_log.as_ref(),
                quest_journal.as_ref(),
                &director,
                &save_state,
            )
            .await
            {
                Ok(changed) => {
                    if result_tx
                        .send(EventProcessingResult { changed })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(err) => {
                    warn!(
                        event_type = %event.event_type,
                        error = %err,
                        "failed to process queued bridge event"
                    );
                }
            }
        }
    });

    (event_tx, result_rx)
}

async fn handle_callback(
    callback: BridgeCallbackMessage,
    client: &mut BridgeClient,
    event_tx: &mpsc::Sender<GameEvent>,
    quest_decision_responses: Option<&QuestDecisionResponseReader>,
    quest_decisions: Option<&QuestDecisionWriter>,
    memory: Option<&MemoryWriter>,
    quest_log: Option<&QuestLogWriter>,
    quest_journal: Option<&QuestJournalWriter>,
    llm: &impl LlmClient,
    director: &Arc<Mutex<DirectorState>>,
    save_state: &Arc<Mutex<AgesBeyondSaveState>>,
) -> anyhow::Result<()> {
    if callback.event().name() == "pre_save" {
        flush_companion_state(
            client,
            quest_decision_responses,
            memory,
            quest_log,
            quest_journal,
            quest_decisions,
            director,
            save_state,
        )
        .await
        .context("failed to flush Ages Beyond state before save")?;

        if let Some(request_id) = callback.request_id() {
            client
                .write_callback_success(request_id, &json!({ "consume": false }))
                .context("failed to write pre_save bridge callback reply")?;
        }
        return Ok(());
    }

    if callback.event().name() == "ui_text" {
        if let Some(request_id) = callback.request_id() {
            handle_ui_text_callback(request_id, callback.event(), client, llm, director).await?;
        }
        return Ok(());
    }

    if let Some(request_id) = callback.request_id() {
        client
            .write_callback_success(request_id, &json!({ "consume": false }))
            .context("failed to write bridge callback reply")?;
        return Ok(());
    }

    let callback_id = callback_id(&callback);
    let bridge_event = callback.event().clone();
    let Some(event) = tokio::task::block_in_place(|| {
        bridge_event_to_game_event(client, &bridge_event, callback_id)
    })
    .context("failed to adapt bridge callback")?
    else {
        debug!(event = bridge_event.name(), "ignored bridge callback");
        return Ok(());
    };

    flush_companion_state(
        client,
        quest_decision_responses,
        memory,
        quest_log,
        quest_journal,
        quest_decisions,
        director,
        save_state,
    )
    .await?;

    let queued = {
        let mut save_state = save_state.lock().await;
        save_state.enqueue_event_job(event.clone())
    };
    if queued {
        persist_save_state(client, save_state, director).await?;
        event_tx
            .send(event)
            .await
            .context("failed to enqueue Ages Beyond event job")?;
    } else {
        debug!(
            event = bridge_event.name(),
            "skipped duplicate or already pending bridge event"
        );
    }

    Ok(())
}

async fn handle_ui_text_callback(
    request_id: u64,
    event: &BridgeEvent,
    client: &mut BridgeClient,
    llm: &impl LlmClient,
    director: &Arc<Mutex<DirectorState>>,
) -> anyhow::Result<()> {
    let fallback = ui_text_fallback(event).unwrap_or_default();
    let text = match ui_text_response(event, llm, director).await {
        Ok(text) if !text.trim().is_empty() => text,
        Ok(_) => fallback.clone(),
        Err(err) => {
            warn!(
                event = event.name(),
                error = %err,
                "using fallback bridge UI text"
            );
            fallback.clone()
        }
    };

    client
        .write_callback_success(request_id, &json!({ "text": text }))
        .context("failed to write bridge UI text callback reply")?;
    Ok(())
}

async fn ui_text_response(
    event: &BridgeEvent,
    llm: &impl LlmClient,
    director: &Arc<Mutex<DirectorState>>,
) -> anyhow::Result<String> {
    let args = ui_text_args(event).context("missing ui_text payload")?;
    let surface = json_string(args, "surface").unwrap_or_default();
    if surface != "diplomacy_comment" {
        return Ok(json_string(args, "fallback_text").unwrap_or_default());
    }

    let request = diplomacy_text_request(args)?;
    let enriched = director.lock().await.enrich_diplomacy_request(&request);
    let body = RequestBody::DiplomacyText { request: enriched };
    match timeout(UI_TEXT_LLM_TIMEOUT, llm.respond(&body)).await {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(err)) => Err(err),
        Err(_) => Ok(json_string(args, "fallback_text").unwrap_or_default()),
    }
}

fn ui_text_args(event: &BridgeEvent) -> Option<&Value> {
    match event {
        BridgeEvent::UiText { args } => Some(args),
        BridgeEvent::Unknown { name, args } if name == "ui_text" => Some(args),
        _ => None,
    }
}

fn ui_text_fallback(event: &BridgeEvent) -> Option<String> {
    ui_text_args(event).and_then(|args| json_string(args, "fallback_text"))
}

fn diplomacy_text_request(args: &Value) -> anyhow::Result<DiplomacyTextRequest> {
    Ok(DiplomacyTextRequest {
        comment_type: json_string(args, "comment_type").context("missing comment_type")?,
        active_player_id: json_i32(args, "active_player_id").context("missing active_player_id")?,
        leader_player_id: json_i32(args, "leader_player_id").context("missing leader_player_id")?,
        turn: json_i32(args, "turn"),
        active_player_name: json_string(args, "active_player_name"),
        active_civilization: json_string(args, "active_civilization"),
        leader_name: json_string(args, "leader_name"),
        leader_civilization: json_string(args, "leader_civilization"),
        attitude: json_string(args, "attitude"),
        at_war: json_bool(args, "at_war").unwrap_or(false),
        power_relation: json_string(args, "power_relation"),
        fallback_text: json_string(args, "fallback_text"),
        diplomacy_memory: None,
        world_arc: None,
    })
}

fn json_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn json_i32(args: &Value, key: &str) -> Option<i32> {
    args.get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn json_bool(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(Value::as_bool)
}

async fn write_pending_outputs(
    quest_decisions: Option<&QuestDecisionWriter>,
    save_state: &Arc<Mutex<AgesBeyondSaveState>>,
) -> anyhow::Result<()> {
    if let Some(writer) = quest_decisions {
        let decisions = {
            let save_state = save_state.lock().await;
            save_state.pending_decisions().cloned().collect::<Vec<_>>()
        };
        for decision in &decisions {
            writer.append_decision(decision).await?;
        }
    }

    Ok(())
}

async fn flush_companion_state(
    client: &mut BridgeClient,
    quest_decision_responses: Option<&QuestDecisionResponseReader>,
    memory: Option<&MemoryWriter>,
    quest_log: Option<&QuestLogWriter>,
    quest_journal: Option<&QuestJournalWriter>,
    quest_decisions: Option<&QuestDecisionWriter>,
    director: &Arc<Mutex<DirectorState>>,
    save_state: &Arc<Mutex<AgesBeyondSaveState>>,
) -> anyhow::Result<bool> {
    let (applied, decisions_changed) = {
        let mut save_state = save_state.lock().await;
        events::apply_quest_decision_responses(
            quest_decision_responses,
            memory,
            quest_log,
            quest_journal,
            director,
            &mut save_state,
        )
        .await?
    };
    if !applied.is_empty() {
        debug!(
            count = applied.len(),
            "applied quest decision responses before bridge event"
        );
    }
    if decisions_changed {
        write_pending_outputs(quest_decisions, save_state).await?;
    }

    let rewards_changed = apply_pending_rewards(client, save_state).await?;
    if decisions_changed || rewards_changed {
        persist_save_state(client, save_state, director).await?;
        return Ok(true);
    }

    Ok(false)
}

async fn apply_pending_rewards(
    client: &mut BridgeClient,
    save_state: &Arc<Mutex<AgesBeyondSaveState>>,
) -> anyhow::Result<bool> {
    let rewards = save_state.lock().await.pending_rewards_to_apply();
    let mut changed = false;

    for reward in rewards {
        if reward.reward_key == "gold" && reward.amount > 0 {
            let result = tokio::task::block_in_place(|| {
                client.change_player_gold(reward.player_id, reward.amount)
            });
            match result {
                Ok(_) => {
                    changed |= save_state.lock().await.mark_reward_applied(&reward.id);
                    info!(
                        reward_id = %reward.id,
                        player_id = reward.player_id,
                        amount = reward.amount,
                        "applied quest gold reward through bridge"
                    );
                }
                Err(err) => {
                    warn!(
                        reward_id = %reward.id,
                        player_id = reward.player_id,
                        amount = reward.amount,
                        error = %err,
                        "failed to apply quest gold reward; leaving pending"
                    );
                }
            }
        } else {
            warn!(
                reward_id = %reward.id,
                reward_key = %reward.reward_key,
                amount = reward.amount,
                "skipping unsupported quest reward"
            );
            changed |= save_state.lock().await.mark_reward_applied(&reward.id);
        }
    }

    Ok(changed)
}

async fn persist_save_state(
    client: &mut BridgeClient,
    save_state: &Arc<Mutex<AgesBeyondSaveState>>,
    director: &Arc<Mutex<DirectorState>>,
) -> anyhow::Result<()> {
    let director_snapshot = director.lock().await.memory_snapshot();
    let snapshot = {
        let mut save_state = save_state.lock().await;
        save_state.director = director_snapshot;
        save_state.clone()
    };

    tokio::task::block_in_place(|| snapshot.save_to_bridge(client))?;
    Ok(())
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
