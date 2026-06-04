#![cfg_attr(not(windows), allow(dead_code))]

use std::path::PathBuf;

use ages_beyond_protocol::GameEvent;
use anyhow::Context;
use serde_json::Value;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChronicleWrite {
    Appended,
    DuplicateSkipped,
}

#[derive(Clone, Debug)]
pub struct ChronicleWriter {
    path: PathBuf,
}

impl ChronicleWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn append_event(
        &self,
        event: &GameEvent,
        heading: &str,
        text: &str,
    ) -> anyhow::Result<ChronicleWrite> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create chronicle directory {}", parent.display())
            })?;
        }

        let exists = tokio::fs::metadata(&self.path).await.is_ok();
        let event_id = event_id_label(event);
        if exists && event_id != "unsaved" {
            let existing = tokio::fs::read_to_string(&self.path)
                .await
                .with_context(|| format!("failed to read chronicle {}", self.path.display()))?;
            if existing.contains(&format!("Event {event_id} - ")) {
                return Ok(ChronicleWrite::DuplicateSkipped);
            }
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .with_context(|| format!("failed to open chronicle {}", self.path.display()))?;

        if !exists {
            file.write_all(b"# Civilization IV: Ages Beyond Chronicle\n\n")
                .await
                .context("failed to write chronicle header")?;
        }

        if let Some(chapter) = chapter_label(event) {
            let existing = if exists {
                tokio::fs::read_to_string(&self.path)
                    .await
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let chapter_heading = format!("## {chapter}");
            if !existing.contains(&chapter_heading) {
                file.write_all(format!("{chapter_heading}\n\n").as_bytes())
                    .await
                    .context("failed to append chronicle chapter")?;
            }
        }

        let turn = turn_label(event);
        let entry = format!("### Event {event_id} - Turn {turn}: {heading}\n\n{text}\n\n");

        file.write_all(entry.as_bytes())
            .await
            .context("failed to append chronicle entry")?;
        file.flush().await.context("failed to flush chronicle")?;

        Ok(ChronicleWrite::Appended)
    }
}

fn turn_label(event: &GameEvent) -> String {
    event
        .turn
        .map(|turn| turn.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn event_id_label(event: &GameEvent) -> String {
    match event.facts.get("event_id") {
        Some(Value::Number(value)) => value.to_string(),
        Some(value) => value.to_string(),
        None => "unsaved".to_owned(),
    }
}

fn chapter_label(event: &GameEvent) -> Option<String> {
    match event.facts.get("chapter") {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.trim().to_owned()),
        _ => None,
    }
}
