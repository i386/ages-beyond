#![cfg_attr(not(windows), allow(dead_code))]

use std::{
    fmt::Write as _,
    io::{ErrorKind, SeekFrom},
    path::PathBuf,
    sync::Arc,
};

use anyhow::Context;
use tokio::{
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader},
    sync::Mutex,
};

use crate::director::{DirectorMemorySnapshot, LivingQuestSnapshot, QuestDecisionResponse};

#[derive(Clone, Debug)]
pub struct MemoryWriter {
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct QuestLogWriter {
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct QuestJournalWriter {
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct QuestDecisionResponseReader {
    path: PathBuf,
    offset: Arc<Mutex<u64>>,
}

impl MemoryWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn write_snapshot(&self, snapshot: &DirectorMemorySnapshot) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create memory directory {}", parent.display())
            })?;
        }

        let mut bytes =
            serde_json::to_vec_pretty(snapshot).context("failed to serialize memory snapshot")?;
        bytes.push(b'\n');

        tokio::fs::write(&self.path, bytes)
            .await
            .with_context(|| format!("failed to write memory snapshot {}", self.path.display()))?;

        Ok(())
    }
}

impl QuestDecisionResponseReader {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            offset: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn read_new(&self) -> anyhow::Result<Vec<QuestDecisionResponse>> {
        let metadata = match tokio::fs::metadata(&self.path).await {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(err).with_context(|| {
                    format!(
                        "failed to inspect quest decision response file {}",
                        self.path.display()
                    )
                });
            }
        };

        let mut offset = self.offset.lock().await;
        if metadata.len() < *offset {
            *offset = 0;
        }

        let mut file = tokio::fs::File::open(&self.path).await.with_context(|| {
            format!(
                "failed to open quest decision response file {}",
                self.path.display()
            )
        })?;
        file.seek(SeekFrom::Start(*offset)).await.with_context(|| {
            format!(
                "failed to seek quest decision response file {}",
                self.path.display()
            )
        })?;

        let mut reader = BufReader::new(file);
        let mut next_offset = *offset;
        let mut responses = Vec::new();
        loop {
            let mut line = String::new();
            let bytes = reader.read_line(&mut line).await.with_context(|| {
                format!(
                    "failed to read quest decision response file {}",
                    self.path.display()
                )
            })?;
            if bytes == 0 {
                break;
            }
            next_offset += bytes as u64;
            if let Some(response) = parse_quest_decision_response(&line) {
                responses.push(response);
            }
        }
        *offset = next_offset;

        Ok(responses)
    }
}

impl QuestLogWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn write_snapshot(&self, snapshot: &DirectorMemorySnapshot) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create quest log directory {}", parent.display())
            })?;
        }

        let text = render_quest_log(snapshot);
        tokio::fs::write(&self.path, text)
            .await
            .with_context(|| format!("failed to write quest log {}", self.path.display()))?;

        Ok(())
    }
}

impl QuestJournalWriter {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn write_snapshot(&self, snapshot: &DirectorMemorySnapshot) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create quest journal directory {}",
                    parent.display()
                )
            })?;
        }

        let text = render_quest_journal(snapshot);
        tokio::fs::write(&self.path, text)
            .await
            .with_context(|| format!("failed to write quest journal {}", self.path.display()))?;

        Ok(())
    }
}

fn parse_quest_decision_response(line: &str) -> Option<QuestDecisionResponse> {
    let parts = line.trim().splitn(3, '\t').collect::<Vec<_>>();
    if parts.len() != 3 {
        return None;
    }

    let id = parts[0].trim();
    let player_id = parts[1].trim().parse::<i32>().ok()?;
    let choice = parts[2].trim();
    if id.is_empty() || choice.is_empty() {
        return None;
    }

    Some(QuestDecisionResponse {
        id: id.to_owned(),
        player_id,
        choice: choice.to_owned(),
    })
}

fn render_quest_log(snapshot: &DirectorMemorySnapshot) -> String {
    let mut output = String::new();
    output.push_str("# Ages Beyond Living Quests\n\n");
    output.push_str("This file is rewritten by mod.exe from the Rust director state.\n\n");

    let active = snapshot
        .living_quests
        .iter()
        .filter(|quest| quest.status == "active")
        .collect::<Vec<_>>();
    let completed = snapshot
        .living_quests
        .iter()
        .filter(|quest| quest.status == "completed")
        .collect::<Vec<_>>();
    let other = snapshot
        .living_quests
        .iter()
        .filter(|quest| quest.status != "active" && quest.status != "completed")
        .collect::<Vec<_>>();

    render_quest_section(&mut output, "Active", &active);
    render_quest_section(&mut output, "Completed", &completed);
    if !other.is_empty() {
        render_quest_section(&mut output, "Other", &other);
    }

    output
}

fn render_quest_journal(snapshot: &DirectorMemorySnapshot) -> String {
    let active = snapshot
        .living_quests
        .iter()
        .filter(|quest| quest.status == "active")
        .collect::<Vec<_>>();
    let completed = snapshot
        .living_quests
        .iter()
        .filter(|quest| quest.status == "completed")
        .collect::<Vec<_>>();
    let headline = active
        .iter()
        .take(2)
        .filter_map(|quest| clean_markdown_line(&quest.title))
        .collect::<Vec<_>>()
        .join("; ");
    let headline = if headline.is_empty() {
        "No active living quests".to_owned()
    } else {
        format!("Active: {headline}")
    };

    let mut output = String::new();
    let _ = writeln!(
        output,
        "summary\t{}\t{}\t{}",
        active.len(),
        completed.len(),
        sanitize_tsv_cell(&headline)
    );

    for quest in active {
        render_quest_journal_row(&mut output, "active", quest);
    }
    for quest in completed {
        render_quest_journal_row(&mut output, "completed", quest);
    }

    output
}

fn render_quest_journal_row(output: &mut String, status: &str, quest: &LivingQuestSnapshot) {
    let latest_progress = quest
        .progress_notes
        .last()
        .map(String::as_str)
        .unwrap_or("");
    let _ = writeln!(
        output,
        "{}\t{}\t{}\t{}\t{}\t{}",
        status,
        sanitize_tsv_cell(&quest.id),
        sanitize_tsv_cell(&quest.civilization),
        sanitize_tsv_cell(&quest.title),
        sanitize_tsv_cell(&format!(
            "{} Progress: {}/{}",
            quest.objective.text, quest.objective.progress, quest.objective.required
        )),
        sanitize_tsv_cell(&quest_journal_note(quest, latest_progress))
    );
}

fn quest_journal_note(quest: &LivingQuestSnapshot, latest_progress: &str) -> String {
    let stance = quest
        .decision
        .as_ref()
        .map(|decision| format!("Stance: {}", decision.choice))
        .unwrap_or_default();

    match (stance.is_empty(), latest_progress.trim().is_empty()) {
        (true, true) => String::new(),
        (false, true) => stance,
        (true, false) => latest_progress.to_owned(),
        (false, false) => format!("{stance}. {latest_progress}"),
    }
}

fn render_quest_section(output: &mut String, heading: &str, quests: &[&LivingQuestSnapshot]) {
    let _ = writeln!(output, "## {heading}\n");

    if quests.is_empty() {
        output.push_str("No living quests in this state.\n\n");
        return;
    }

    for quest in quests {
        render_quest(output, quest);
    }
}

fn render_quest(output: &mut String, quest: &LivingQuestSnapshot) {
    let _ = writeln!(
        output,
        "### {}\n",
        clean_markdown_line(&quest.title).unwrap_or_else(|| "Untitled quest".to_owned())
    );
    let _ = writeln!(
        output,
        "- Civilization: {}",
        clean_markdown_line(&quest.civilization).unwrap_or_else(|| "Unknown".to_owned())
    );
    let _ = writeln!(output, "- Status: {}", display_value(&quest.status));
    let _ = writeln!(output, "- Kind: {}", display_value(&quest.kind));
    let _ = writeln!(
        output,
        "- Origin: {}, {}",
        display_value(&quest.origin_event_type),
        display_turn(quest.origin_turn)
    );

    if let Some(target) = clean_optional_line(quest.target.as_deref()) {
        let _ = writeln!(output, "- Target: {target}");
    }

    if let Some(prompt) = clean_markdown_line(&quest.prompt) {
        let _ = writeln!(output, "- Prompt: {prompt}");
    }
    if let Some(objective) = clean_markdown_line(&quest.objective.text) {
        let _ = writeln!(
            output,
            "- Objective: {} ({}/{})",
            objective, quest.objective.progress, quest.objective.required
        );
    }
    if let Some(reward) = clean_markdown_line(&quest.reward) {
        let _ = writeln!(output, "- Reward: {reward}");
    }
    if let Some(consequence) = clean_markdown_line(&quest.consequence) {
        let _ = writeln!(output, "- Consequence: {consequence}");
    }
    if let Some(decision) = &quest.decision {
        if let Some(choice) = clean_markdown_line(&decision.choice) {
            let _ = writeln!(output, "- Stance: {choice}");
        }
    }

    if !quest.progress_notes.is_empty() {
        output.push_str("- Progress:\n");
        for note in &quest.progress_notes {
            if let Some(note) = clean_markdown_line(note) {
                let _ = writeln!(output, "  - {note}");
            }
        }
    }

    output.push('\n');
}

fn clean_optional_line(value: Option<&str>) -> Option<String> {
    value.and_then(clean_markdown_line)
}

fn clean_markdown_line(value: &str) -> Option<String> {
    let cleaned = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn display_value(value: &str) -> String {
    clean_markdown_line(&value.replace('_', " ")).unwrap_or_else(|| "unknown".to_owned())
}

fn display_turn(turn: Option<i32>) -> String {
    match turn {
        Some(turn) => format!("turn {turn}"),
        None => "turn unknown".to_owned(),
    }
}

fn sanitize_tsv_cell(text: &str) -> String {
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ages_beyond_protocol::GameEvent;
    use serde_json::json;

    use crate::director::DirectorState;

    use super::*;

    fn memory_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ages-beyond-memory-test-{}-{label}.json",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn writes_director_snapshot_projection() {
        let path = memory_path("projection");
        let _ = tokio::fs::remove_file(&path).await;
        let writer = MemoryWriter::new(path.clone());
        let mut director = DirectorState::default();
        director.observe_event(&GameEvent {
            event_type: "wonder_built".to_owned(),
            turn: Some(12),
            actors: Vec::new(),
            summary: None,
            facts: BTreeMap::from([
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("building_name".to_owned(), json!("The Oracle")),
                ("dynamic_quest_seed".to_owned(), json!("wonder_legacy")),
            ]),
        });

        writer
            .write_snapshot(&director.memory_snapshot())
            .await
            .unwrap();
        let text = tokio::fs::read_to_string(&path).await.unwrap();

        assert!(text.contains("Mali completed The Oracle."));
        assert!(text.contains("The Oracle"));

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn writes_living_quest_log() {
        let path = memory_path("quest-log").with_extension("md");
        let _ = tokio::fs::remove_file(&path).await;
        let writer = QuestLogWriter::new(path.clone());
        let mut director = DirectorState::default();
        director.observe_event(&GameEvent {
            event_type: "wonder_built".to_owned(),
            turn: Some(12),
            actors: Vec::new(),
            summary: None,
            facts: BTreeMap::from([
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("building_name".to_owned(), json!("The Oracle")),
                ("dynamic_quest_seed".to_owned(), json!("wonder_legacy")),
            ]),
        });

        writer
            .write_snapshot(&director.memory_snapshot())
            .await
            .unwrap();
        let text = tokio::fs::read_to_string(&path).await.unwrap();

        assert!(text.contains("# Ages Beyond Living Quests"));
        assert!(text.contains("## Active"));
        assert!(text.contains("Mali"));
        assert!(text.contains("The Oracle"));
        assert!(text.contains("- Origin: wonder built, turn 12"));
        assert!(text.contains("- Objective: Create a lasting deed worthy of The Oracle. (0/1)"));
        assert!(text.contains("- Reward: Future arcs can treat the achievement"));
        assert!(text.contains("- Consequence: If ignored, future narration"));
        assert!(text.contains("## Completed"));

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn reads_quest_decision_responses_incrementally() {
        let path = memory_path("quest-decision-responses").with_extension("tsv");
        let _ = tokio::fs::remove_file(&path).await;
        let reader = QuestDecisionResponseReader::new(path.clone());

        tokio::fs::write(
            &path,
            "decision:77:0:restoration:memphis\t0\tSwear vengeance\nbad line\n",
        )
        .await
        .unwrap();

        let responses = reader.read_new().await.unwrap();
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].id, "decision:77:0:restoration:memphis");
        assert_eq!(responses[0].player_id, 0);
        assert_eq!(responses[0].choice, "Swear vengeance");
        assert!(reader.read_new().await.unwrap().is_empty());

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn writes_living_quest_journal_summary() {
        let path = memory_path("quest-journal").with_extension("tsv");
        let _ = tokio::fs::remove_file(&path).await;
        let writer = QuestJournalWriter::new(path.clone());
        let mut director = DirectorState::default();
        director.observe_event(&GameEvent {
            event_type: "wonder_built".to_owned(),
            turn: Some(12),
            actors: Vec::new(),
            summary: None,
            facts: BTreeMap::from([
                ("player_id".to_owned(), json!(2)),
                ("player_civilization".to_owned(), json!("Mali")),
                ("building_name".to_owned(), json!("The Oracle")),
                ("dynamic_quest_seed".to_owned(), json!("wonder_legacy")),
            ]),
        });

        writer
            .write_snapshot(&director.memory_snapshot())
            .await
            .unwrap();
        let text = tokio::fs::read_to_string(&path).await.unwrap();

        assert!(text.contains("summary\t1\t0\tActive: Make The Oracle Matter"));
        assert!(text.contains("active\t"));
        assert!(text.contains("\tMali\t"));
        assert!(text.contains("\tMake The Oracle Matter\t"));
        assert!(text.contains("Create a lasting deed worthy of The Oracle. Progress: 0/1"));
        assert!(!text.contains('\r'));

        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn writes_empty_living_quest_log() {
        let path = memory_path("empty-quest-log").with_extension("md");
        let _ = tokio::fs::remove_file(&path).await;
        let writer = QuestLogWriter::new(path.clone());

        writer
            .write_snapshot(&DirectorState::default().memory_snapshot())
            .await
            .unwrap();
        let text = tokio::fs::read_to_string(&path).await.unwrap();

        assert!(text.contains("# Ages Beyond Living Quests"));
        assert!(text.contains("No living quests in this state."));

        let _ = tokio::fs::remove_file(&path).await;
    }
}
