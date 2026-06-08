mod bridge;
mod chronicle;
mod director;
mod events;
mod llm;
mod memory;
mod notifications;
mod save_state;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use tracing::info;

use crate::chronicle::ChronicleWriter;
use crate::llm::OllamaClient;
use crate::memory::{
    MemoryWriter, QuestDecisionResponseReader, QuestJournalWriter, QuestLogWriter,
    QuestRewardResponseReader,
};
use crate::notifications::{NotificationWriter, QuestDecisionWriter, QuestRewardWriter};

#[derive(Debug, Parser)]
#[command(name = "mod")]
#[command(about = "LLM companion process for Civilization IV: Ages Beyond")]
struct Args {
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    #[arg(long, default_value = "llama3.1")]
    model: String,

    #[arg(long)]
    chronicle: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ages_beyond_companion=info,info".into()),
        )
        .init();

    let args = Args::parse();
    let llm = OllamaClient::new(args.ollama_url, args.model)
        .context("failed to initialize Ollama client")?;

    let chronicle_path = resolve_chronicle_path(args.chronicle);
    let chronicle = chronicle_path.clone().map(ChronicleWriter::new);
    let notifications = chronicle_path
        .clone()
        .map(|path| NotificationWriter::new(path.with_file_name("AgesBeyondNotifications.tsv")));
    let quest_notifications = chronicle_path.clone().map(|path| {
        NotificationWriter::new(path.with_file_name("AgesBeyondQuestNotifications.tsv"))
    });
    let quest_decisions = chronicle_path
        .clone()
        .map(|path| QuestDecisionWriter::new(path.with_file_name("AgesBeyondQuestDecisions.tsv")));
    let quest_decision_responses = chronicle_path.clone().map(|path| {
        QuestDecisionResponseReader::new(
            path.with_file_name("AgesBeyondQuestDecisionResponses.tsv"),
        )
    });
    let quest_rewards = chronicle_path
        .clone()
        .map(|path| QuestRewardWriter::new(path.with_file_name("AgesBeyondQuestRewards.tsv")));
    let quest_reward_responses = chronicle_path.clone().map(|path| {
        QuestRewardResponseReader::new(path.with_file_name("AgesBeyondQuestRewardResponses.tsv"))
    });
    let memory = chronicle_path
        .clone()
        .map(|path| MemoryWriter::new(path.with_file_name("AgesBeyondMemory.json")));
    let quest_log = chronicle_path
        .clone()
        .map(|path| QuestLogWriter::new(path.with_file_name("AgesBeyondQuestLog.md")));
    let quest_journal = chronicle_path
        .map(|path| QuestJournalWriter::new(path.with_file_name("AgesBeyondQuestJournal.tsv")));
    if let Some(writer) = &notifications {
        writer.reset().await?;
    }
    if let Some(writer) = &quest_notifications {
        writer.reset().await?;
    }
    if let Some(writer) = &quest_decisions {
        writer.reset().await?;
    }
    if let Some(writer) = &quest_rewards {
        writer.reset().await?;
    }

    info!("starting Ages Beyond companion bridge client");
    bridge::run_client(
        llm,
        chronicle,
        notifications,
        quest_notifications,
        quest_decisions,
        quest_decision_responses,
        quest_rewards,
        quest_reward_responses,
        memory,
        quest_log,
        quest_journal,
    )
    .await
}

fn resolve_chronicle_path(path: Option<PathBuf>) -> Option<PathBuf> {
    if path.is_some() {
        return path;
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from));
    let current_dir = std::env::current_dir().ok();

    let candidates = [
        exe_dir.as_ref().map(|dir| {
            dir.join("..")
                .join("Chronicle")
                .join("AgesBeyondChronicle.md")
        }),
        exe_dir
            .as_ref()
            .map(|dir| dir.join("Chronicle").join("AgesBeyondChronicle.md")),
        current_dir
            .as_ref()
            .map(|dir| dir.join("Chronicle").join("AgesBeyondChronicle.md")),
    ];

    candidates
        .iter()
        .flatten()
        .find(|path| path.parent().is_some_and(|parent| parent.exists()))
        .cloned()
        .or_else(|| candidates.into_iter().flatten().next())
}
