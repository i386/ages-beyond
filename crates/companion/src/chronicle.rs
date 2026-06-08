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

    pub async fn append_memory_projection(
        &self,
        event: &GameEvent,
        lines: &[String],
    ) -> anyhow::Result<()> {
        self.append_projection(event, "Memories", "Memory", lines)
            .await
    }

    pub async fn append_quest_projection(
        &self,
        event: &GameEvent,
        lines: &[String],
    ) -> anyhow::Result<()> {
        self.append_projection(event, "Living Quests", "Quest", lines)
            .await
    }

    async fn append_projection(
        &self,
        event: &GameEvent,
        chapter: &str,
        label: &str,
        lines: &[String],
    ) -> anyhow::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create chronicle directory {}", parent.display())
            })?;
        }

        let exists = tokio::fs::metadata(&self.path).await.is_ok();
        let event_id = event_id_label(event);
        let projection_id = format!("{label} {event_id}");
        if exists && event_id != "unsaved" {
            let existing = tokio::fs::read_to_string(&self.path)
                .await
                .with_context(|| format!("failed to read chronicle {}", self.path.display()))?;
            if existing.contains(&format!("### {projection_id} - ")) {
                return Ok(());
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
                .with_context(|| format!("failed to append {chapter} chapter"))?;
        }

        let turn = turn_label(event);
        let body = lines.join("\n");
        let entry = format!("### {projection_id} - Turn {turn}\n\n{body}\n\n");

        file.write_all(entry.as_bytes())
            .await
            .with_context(|| format!("failed to append {label} projection"))?;
        file.flush().await.context("failed to flush chronicle")?;

        Ok(())
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;

    fn test_event() -> GameEvent {
        GameEvent {
            event_type: "wonder_built".to_owned(),
            turn: Some(12),
            actors: Vec::new(),
            summary: None,
            facts: BTreeMap::from([("event_id".to_owned(), json!(42))]),
        }
    }

    #[tokio::test]
    async fn appends_memory_projection_without_notification_format() {
        let path = std::env::temp_dir().join(format!(
            "ages-beyond-chronicle-test-{}-{}.md",
            std::process::id(),
            "memory"
        ));
        let _ = tokio::fs::remove_file(&path).await;

        let writer = ChronicleWriter::new(path.clone());
        writer
            .append_memory_projection(
                &test_event(),
                &["Memory: Mali completed The Oracle.".to_owned()],
            )
            .await
            .unwrap();

        let text = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(text.contains("## Memories"));
        assert!(text.contains("### Memory 42 - Turn 12"));
        assert!(text.contains("Memory: Mali completed The Oracle."));

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn appends_quest_projection_to_living_quests_chapter() {
        let path = std::env::temp_dir().join(format!(
            "ages-beyond-chronicle-test-{}-{}.md",
            std::process::id(),
            "quest"
        ));
        let _ = tokio::fs::remove_file(&path).await;

        let writer = ChronicleWriter::new(path.clone());
        writer
            .append_quest_projection(
                &test_event(),
                &["Quest: Remember Memphis - Restore its place in memory.".to_owned()],
            )
            .await
            .unwrap();

        let text = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(text.contains("## Living Quests"));
        assert!(text.contains("### Quest 42 - Turn 12"));
        assert!(text.contains("Quest: Remember Memphis"));

        let _ = tokio::fs::remove_file(&path).await;
    }
}
