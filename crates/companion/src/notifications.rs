#![cfg_attr(not(windows), allow(dead_code))]

use std::path::PathBuf;

use ages_beyond_protocol::GameEvent;
use anyhow::Context;
use serde_json::Value;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct NotificationWriter {
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
