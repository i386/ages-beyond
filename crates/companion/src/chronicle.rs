#![cfg_attr(not(windows), allow(dead_code))]

use std::path::PathBuf;

use ages_beyond_protocol::GameEvent;
use anyhow::Context;
use serde_json::Value;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct ChronicleWriter {
    path: PathBuf,
}

impl ChronicleWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn append_event(&self, event: &GameEvent, text: &str) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create chronicle directory {}", parent.display())
            })?;
        }

        let exists = tokio::fs::metadata(&self.path).await.is_ok();
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

        let turn = event
            .turn
            .map(|turn| turn.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let event_id = match event.facts.get("event_id") {
            Some(Value::Number(value)) => value.to_string(),
            Some(value) => value.to_string(),
            None => "unsaved".to_owned(),
        };
        let heading = event.event_type.replace('_', " ");
        let entry = format!("## Event {event_id} - Turn {turn}: {heading}\n\n{text}\n\n");

        file.write_all(entry.as_bytes())
            .await
            .context("failed to append chronicle entry")?;
        file.flush().await.context("failed to flush chronicle")?;

        Ok(())
    }
}
