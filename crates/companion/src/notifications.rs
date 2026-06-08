#![cfg_attr(not(windows), allow(dead_code))]

use std::path::PathBuf;

use ages_beyond_protocol::GameEvent;
use anyhow::Context;
use serde_json::Value;
use tokio::io::AsyncWriteExt;

use crate::director::{QuestDecisionPrompt, QuestRewardCommand};

#[derive(Clone, Debug)]
pub struct NotificationWriter {
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct QuestRewardWriter {
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct QuestDecisionWriter {
    path: PathBuf,
}

impl NotificationWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn reset(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create notification directory {}",
                    parent.display()
                )
            })?;
        }

        tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .await
            .with_context(|| {
                format!("failed to reset notification file {}", self.path.display())
            })?;

        Ok(())
    }

    pub async fn append_event(&self, event: &GameEvent, text: &str) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create notification directory {}",
                    parent.display()
                )
            })?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .with_context(|| format!("failed to open notification file {}", self.path.display()))?;

        let line = format!(
            "{}\t{}\t{}\t{}\n",
            event_id_label(event),
            fact_i64(event, "x").unwrap_or(-1),
            fact_i64(event, "y").unwrap_or(-1),
            sanitize_text(text)
        );
        file.write_all(line.as_bytes())
            .await
            .context("failed to append notification")?;
        file.flush().await.context("failed to flush notification")?;

        Ok(())
    }
}

impl QuestRewardWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn reset(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create quest reward directory {}",
                    parent.display()
                )
            })?;
        }

        tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .await
            .with_context(|| {
                format!("failed to reset quest reward file {}", self.path.display())
            })?;

        Ok(())
    }

    pub async fn append_reward(&self, reward: &QuestRewardCommand) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create quest reward directory {}",
                    parent.display()
                )
            })?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .with_context(|| format!("failed to open quest reward file {}", self.path.display()))?;

        let line = format!(
            "{}\t{}\t{}\t{}\t{}\n",
            sanitize_text(&reward.id),
            reward.player_id,
            sanitize_text(&reward.reward_key),
            reward.amount,
            sanitize_text(&reward.text)
        );
        file.write_all(line.as_bytes())
            .await
            .context("failed to append quest reward")?;
        file.flush().await.context("failed to flush quest reward")?;

        Ok(())
    }
}

impl QuestDecisionWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn reset(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create quest decision directory {}",
                    parent.display()
                )
            })?;
        }

        tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .await
            .with_context(|| {
                format!(
                    "failed to reset quest decision file {}",
                    self.path.display()
                )
            })?;

        Ok(())
    }

    pub async fn append_decision(&self, decision: &QuestDecisionPrompt) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create quest decision directory {}",
                    parent.display()
                )
            })?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .with_context(|| {
                format!("failed to open quest decision file {}", self.path.display())
            })?;

        let mut choices = decision.choices.iter();
        let line = format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            sanitize_text(&decision.id),
            decision.player_id,
            sanitize_text(&decision.title),
            sanitize_text(&decision.body),
            sanitize_text(choices.next().map(String::as_str).unwrap_or("Accept")),
            sanitize_text(choices.next().map(String::as_str).unwrap_or("Defer")),
            sanitize_text(choices.next().map(String::as_str).unwrap_or("Ignore")),
        );
        file.write_all(line.as_bytes())
            .await
            .context("failed to append quest decision")?;
        file.flush()
            .await
            .context("failed to flush quest decision")?;

        Ok(())
    }
}

fn sanitize_text(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            '\r' | '\n' | '\t' => ' ',
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            _ if ch.is_ascii() => ch,
            _ => '?',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn event_id_label(event: &GameEvent) -> String {
    match event.facts.get("event_id") {
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => value.to_string(),
        None => "unsaved".to_owned(),
    }
}

fn fact_i64(event: &GameEvent, key: &str) -> Option<i64> {
    match event.facts.get(key) {
        Some(Value::Number(value)) => value.as_i64(),
        Some(Value::String(value)) => value.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reward_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ages-beyond-reward-test-{}-{label}.tsv",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn writes_quest_reward_command() {
        let path = reward_path("command");
        let _ = tokio::fs::remove_file(&path).await;
        let writer = QuestRewardWriter::new(path.clone());

        writer.reset().await.unwrap();
        writer
            .append_reward(&QuestRewardCommand {
                id: "reward:test".to_owned(),
                player_id: 0,
                reward_key: "gold".to_owned(),
                amount: 75,
                text: "Living Quest reward:\n+75 gold.".to_owned(),
            })
            .await
            .unwrap();

        let text = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(
            text,
            "reward:test\t0\tgold\t75\tLiving Quest reward: +75 gold.\n"
        );

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn writes_quest_decision_prompt() {
        let path = reward_path("decision");
        let _ = tokio::fs::remove_file(&path).await;
        let writer = QuestDecisionWriter::new(path.clone());

        writer.reset().await.unwrap();
        writer
            .append_decision(&QuestDecisionPrompt {
                id: "decision:test".to_owned(),
                player_id: 0,
                title: "Remember Memphis".to_owned(),
                body: "Choose a stance.\nNo coordinates.".to_owned(),
                choices: vec![
                    "Restore".to_owned(),
                    "Vengeance".to_owned(),
                    "Warning".to_owned(),
                ],
            })
            .await
            .unwrap();

        let text = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(
            text,
            "decision:test\t0\tRemember Memphis\tChoose a stance. No coordinates.\tRestore\tVengeance\tWarning\n"
        );

        let _ = tokio::fs::remove_file(&path).await;
    }
}
