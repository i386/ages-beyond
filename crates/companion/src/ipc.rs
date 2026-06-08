#[cfg(windows)]
use ages_beyond_protocol::{CompanionRequest, CompanionResponse};
#[cfg(windows)]
use anyhow::Context;
#[cfg(windows)]
use std::{collections::HashMap, sync::Arc};
#[cfg(windows)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(windows)]
use tokio::sync::Mutex;
#[cfg(windows)]
use tracing::{debug, error, info, warn};

use crate::chronicle::ChronicleWriter;
use crate::director::DirectorState;
#[cfg(windows)]
use crate::events;
use crate::llm::LlmClient;
use crate::memory::{
    MemoryWriter, QuestDecisionResponseReader, QuestJournalWriter, QuestLogWriter,
};
use crate::notifications::{NotificationWriter, QuestDecisionWriter, QuestRewardWriter};

#[cfg(windows)]
pub async fn run_server<L>(
    pipe_name: &str,
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
    use tokio::net::windows::named_pipe::ServerOptions;

    let pipe = ServerOptions::new()
        .first_pipe_instance(true)
        .create(pipe_name)
        .with_context(|| format!("failed to create named pipe {pipe_name}"))?;

    info!(pipe = %pipe_name, "waiting for DLL connection");
    pipe.connect()
        .await
        .with_context(|| format!("failed while waiting for pipe client {pipe_name}"))?;
    info!("DLL connected");

    let diplomacy_cache = Arc::new(Mutex::new(HashMap::new()));
    let director = Arc::new(Mutex::new(initial_director));
    handle_connection(
        pipe,
        llm,
        chronicle,
        notifications,
        quest_notifications,
        quest_decisions,
        quest_decision_responses,
        quest_rewards,
        memory,
        quest_log,
        quest_journal,
        diplomacy_cache,
        director,
    )
    .await
}

#[cfg(not(windows))]
pub async fn run_server<L>(
    _pipe_name: &str,
    _llm: L,
    _chronicle: Option<ChronicleWriter>,
    _notifications: Option<NotificationWriter>,
    _quest_notifications: Option<NotificationWriter>,
    _quest_decisions: Option<QuestDecisionWriter>,
    _quest_decision_responses: Option<QuestDecisionResponseReader>,
    _quest_rewards: Option<QuestRewardWriter>,
    _memory: Option<MemoryWriter>,
    _quest_log: Option<QuestLogWriter>,
    _quest_journal: Option<QuestJournalWriter>,
    _initial_director: DirectorState,
) -> anyhow::Result<()>
where
    L: LlmClient,
{
    anyhow::bail!("Windows named pipes are only available on Windows builds")
}

#[cfg(windows)]
async fn handle_connection<S, L>(
    stream: S,
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
    diplomacy_cache: Arc<Mutex<HashMap<String, String>>>,
    director: Arc<Mutex<DirectorState>>,
) -> anyhow::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    L: LlmClient,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines
        .next_line()
        .await
        .context("failed to read pipe line")?
    {
        let response = match serde_json::from_str::<CompanionRequest>(&line) {
            Ok(request) => {
                let id = request.id.clone();
                match handle_request(
                    &request.body,
                    &llm,
                    chronicle.as_ref(),
                    notifications.as_ref(),
                    quest_notifications.as_ref(),
                    quest_decisions.as_ref(),
                    quest_decision_responses.as_ref(),
                    quest_rewards.as_ref(),
                    memory.as_ref(),
                    quest_log.as_ref(),
                    quest_journal.as_ref(),
                    diplomacy_cache.clone(),
                    director.clone(),
                )
                .await
                {
                    Ok(text) => CompanionResponse::ok(id, text),
                    Err(err) => {
                        error!(request_id = %id, error = %err, "failed to handle request");
                        CompanionResponse::error(id, err.to_string())
                    }
                }
            }
            Err(err) => {
                error!(error = %err, "invalid request JSON");
                CompanionResponse::error("invalid-request", err.to_string())
            }
        };

        let mut output = serde_json::to_vec(&response).context("failed to serialize response")?;
        output.push(b'\n');
        writer
            .write_all(&output)
            .await
            .context("failed to write pipe response")?;
        writer
            .flush()
            .await
            .context("failed to flush pipe response")?;
    }

    info!("DLL disconnected");
    Ok(())
}

#[cfg(windows)]
async fn handle_request<L>(
    body: &ages_beyond_protocol::RequestBody,
    llm: &L,
    chronicle: Option<&ChronicleWriter>,
    notifications: Option<&NotificationWriter>,
    quest_notifications: Option<&NotificationWriter>,
    quest_decisions: Option<&QuestDecisionWriter>,
    quest_decision_responses: Option<&QuestDecisionResponseReader>,
    quest_rewards: Option<&QuestRewardWriter>,
    memory: Option<&MemoryWriter>,
    quest_log: Option<&QuestLogWriter>,
    quest_journal: Option<&QuestJournalWriter>,
    diplomacy_cache: Arc<Mutex<HashMap<String, String>>>,
    director: Arc<Mutex<DirectorState>>,
) -> anyhow::Result<String>
where
    L: LlmClient,
{
    if matches!(
        body,
        ages_beyond_protocol::RequestBody::GameEvent { .. }
            | ages_beyond_protocol::RequestBody::DiplomacyText { .. }
    ) {
        let applied = events::apply_quest_decision_responses(
            quest_decision_responses,
            memory,
            quest_log,
            quest_journal,
            &director,
        )
        .await?;
        if !applied.is_empty() {
            diplomacy_cache.lock().await.clear();
        }
    }

    match body {
        ages_beyond_protocol::RequestBody::Ping => llm.respond(body).await,
        ages_beyond_protocol::RequestBody::HistoricalName { .. } => llm.respond(body).await,
        ages_beyond_protocol::RequestBody::WorldArcTitle { .. } => llm.respond(body).await,
        ages_beyond_protocol::RequestBody::GameEvent { event } => {
            events::process_game_event(
                event,
                llm,
                chronicle,
                notifications,
                quest_notifications,
                quest_decisions,
                quest_rewards,
                memory,
                quest_log,
                quest_journal,
                &director,
            )
            .await
        }
        ages_beyond_protocol::RequestBody::DiplomacyText { request } => {
            let key = diplomacy_cache_key(request);
            if let Some(text) = diplomacy_cache.lock().await.get(&key).cloned() {
                debug!(comment_type = %request.comment_type, key = %key, "served cached diplomacy text");
                return Ok(text);
            }

            let llm = llm.clone();
            let enriched_request = {
                let director = director.lock().await;
                director.enrich_diplomacy_request(request)
            };
            let body = ages_beyond_protocol::RequestBody::DiplomacyText {
                request: enriched_request,
            };
            let cache = diplomacy_cache.clone();
            let key_for_task = key.clone();
            let comment_type = request.comment_type.clone();
            tokio::spawn(async move {
                match llm.respond(&body).await {
                    Ok(text) if !text.trim().is_empty() => {
                        cache.lock().await.insert(key_for_task, text);
                    }
                    Ok(_) => {
                        debug!(comment_type = %comment_type, "diplomacy generation returned empty text");
                    }
                    Err(err) => {
                        warn!(comment_type = %comment_type, error = %err, "failed to generate diplomacy text");
                    }
                }
            });

            Ok(String::new())
        }
    }
}

#[cfg(windows)]
fn diplomacy_cache_key(request: &ages_beyond_protocol::DiplomacyTextRequest) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        request.comment_type,
        request.active_player_id,
        request.leader_player_id,
        request.turn.unwrap_or(-1) / 10,
        request.attitude.as_deref().unwrap_or("unknown")
    )
}
