#![cfg_attr(not(windows), allow(dead_code))]

use std::time::Duration;

use ages_beyond_protocol::{GameEvent, RequestBody};
use anyhow::{anyhow, Context};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

pub trait LlmClient: Clone + Send + Sync + 'static {
    async fn respond(&self, body: &RequestBody) -> anyhow::Result<String>;
}

#[derive(Clone)]
pub struct OllamaClient {
    client: reqwest::Client,
    generate_url: Url,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: String, model: String) -> anyhow::Result<Self> {
        let mut base = Url::parse(&base_url).context("invalid Ollama base URL")?;
        base.set_path("/api/generate");
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .context("failed to build HTTP client")?,
            generate_url: base,
            model,
        })
    }

    async fn generate(&self, prompt: String) -> anyhow::Result<String> {
        let request = OllamaGenerateRequest {
            model: &self.model,
            prompt: &prompt,
            stream: false,
            system: "You write concise, grounded Civilization IV narrative text. Use only the supplied game facts. Do not invent game-mechanical consequences. Use plain ASCII punctuation.",
            options: OllamaOptions {
                temperature: 0.7,
                num_predict: 120,
            },
        };

        let response = self
            .client
            .post(self.generate_url.clone())
            .json(&request)
            .send()
            .await
            .context("Ollama request failed")?;

        if !response.status().is_success() {
            return Err(anyhow!("Ollama returned HTTP {}", response.status()));
        }

        let body: OllamaGenerateResponse = response
            .json()
            .await
            .context("failed to parse Ollama response")?;

        let text = body.response.trim();
        if text.is_empty() {
            return Err(anyhow!("Ollama returned an empty response"));
        }

        Ok(text.to_owned())
    }
}

impl LlmClient for OllamaClient {
    async fn respond(&self, body: &RequestBody) -> anyhow::Result<String> {
        match body {
            RequestBody::Ping => Ok("Ages Beyond Companion ready.".to_owned()),
            RequestBody::GameEvent { event } => match self.generate(game_event_prompt(event)).await
            {
                Ok(text) => Ok(sanitize_player_text(&text)),
                Err(err) => {
                    warn!(event_type = %event.event_type, error = %err, "using fallback chronicle text");
                    Ok(fallback_game_event_text(event))
                }
            },
        }
    }
}

fn game_event_prompt(event: &GameEvent) -> String {
    let facts = serde_json::to_string_pretty(&player_visible_facts(event))
        .unwrap_or_else(|_| "{}".to_owned());
    let actors = serde_json::to_string_pretty(&event.actors).unwrap_or_else(|_| "[]".to_owned());
    let summary = event
        .summary
        .as_deref()
        .map(sanitize_player_text)
        .unwrap_or_default();

    format!(
        "Write one short in-world chronicle entry for this Civilization IV game event.\n\
         Constraints:\n\
         - 1 sentence, maximum 35 words.\n\
         - Historical tone, not modern UI language.\n\
         - Mention only facts provided below.\n\
         - Match tone to facts.importance: minor is restrained, major is consequential, epochal is chapter-defining.\n\
         - If facts.dynamic_quest_seed exists, include a subtle unresolved hook without claiming game effects were applied.\n\
         - Treat facts.data1 as target_team_id for war_declared/peace_signed, tech_id for tech_discovered, religion_id for religion_founded, and building_id for wonder_built.\n\
         - Treat facts.data1/data2 according to named facts when a clearer *_id field is present.\n\
         - Never mention map coordinates, tile coordinates, plot positions, x/y values, or coordinate pairs.\n\
         - Use plain ASCII punctuation.\n\
         - No bullet points.\n\n\
         Event type: {event_type}\n\
         Turn: {turn}\n\
         Summary: {summary}\n\
         Actors: {actors}\n\
         Facts: {facts}",
        event_type = event.event_type,
        turn = event
            .turn
            .map(|turn| turn.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        summary = summary,
        actors = actors,
        facts = facts,
    )
}

fn fallback_game_event_text(event: &GameEvent) -> String {
    match event.summary.as_deref() {
        Some(summary) if !summary.trim().is_empty() => sanitize_player_text(summary),
        _ => format!(
            "A {} event was recorded.",
            event.event_type.replace('_', " ")
        ),
    }
}

fn player_visible_facts(event: &GameEvent) -> serde_json::Map<String, Value> {
    event
        .facts
        .iter()
        .filter(|(key, _)| is_player_visible_fact(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn is_player_visible_fact(key: &str) -> bool {
    !matches!(
        key,
        "x" | "y" | "plot_x" | "plot_y" | "city_x" | "city_y" | "map_x" | "map_y"
    )
}

fn sanitize_player_text(text: &str) -> String {
    let without_coordinate_pairs = remove_coordinate_pairs(text);
    let mut cleaned = without_coordinate_pairs
        .replace(" at the coordinates .", ".")
        .replace(" at coordinates .", ".")
        .replace(" at coordinate .", ".")
        .replace(" at .", ".")
        .replace(" coordinates .", ".")
        .replace(" coordinate .", ".")
        .replace("  ", " ");

    while cleaned.contains("  ") {
        cleaned = cleaned.replace("  ", " ");
    }

    cleaned.trim().to_owned()
}

fn remove_coordinate_pairs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::new();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '(' {
            if let Some(end) = coordinate_pair_end(&chars, index) {
                index = end + 1;
                continue;
            }
        }

        output.push(chars[index]);
        index += 1;
    }

    output
}

fn coordinate_pair_end(chars: &[char], start: usize) -> Option<usize> {
    let mut index = start + 1;
    index = skip_spaces(chars, index);
    index = consume_digits(chars, index)?;
    index = skip_spaces(chars, index);
    if chars.get(index) != Some(&',') {
        return None;
    }
    index += 1;
    index = skip_spaces(chars, index);
    index = consume_digits(chars, index)?;
    index = skip_spaces(chars, index);
    if chars.get(index) == Some(&')') {
        Some(index)
    } else {
        None
    }
}

fn skip_spaces(chars: &[char], mut index: usize) -> usize {
    while chars.get(index).is_some_and(|ch| ch.is_ascii_whitespace()) {
        index += 1;
    }
    index
}

fn consume_digits(chars: &[char], mut index: usize) -> Option<usize> {
    let start = index;
    while chars.get(index).is_some_and(|ch| ch.is_ascii_digit()) {
        index += 1;
    }
    if index > start {
        Some(index)
    } else {
        None
    }
}

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    system: &'a str,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u16,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ages_beyond_protocol::GameEvent;
    use serde_json::json;

    use super::{fallback_game_event_text, game_event_prompt, sanitize_player_text};

    fn event_with_coordinates() -> GameEvent {
        GameEvent {
            event_type: "city_founded".to_owned(),
            turn: Some(0),
            actors: Vec::new(),
            summary: Some("Zulu Empire founded Ulundi at (12,23).".to_owned()),
            facts: BTreeMap::from([
                ("event_id".to_owned(), json!(2)),
                ("x".to_owned(), json!(12)),
                ("y".to_owned(), json!(23)),
                ("city_name".to_owned(), json!("Ulundi")),
                ("importance".to_owned(), json!("epochal")),
            ]),
        }
    }

    #[test]
    fn prompt_excludes_map_coordinates() {
        let prompt = game_event_prompt(&event_with_coordinates());

        assert!(!prompt.contains("\"x\""));
        assert!(!prompt.contains("\"y\""));
        assert!(!prompt.contains("(12,23)"));
        assert!(prompt.contains("\"city_name\""));
        assert!(prompt.contains("Never mention map coordinates"));
    }

    #[test]
    fn fallback_text_excludes_coordinate_pairs() {
        assert_eq!(
            fallback_game_event_text(&event_with_coordinates()),
            "Zulu Empire founded Ulundi."
        );
    }

    #[test]
    fn generated_text_sanitizer_removes_coordinate_pairs() {
        assert_eq!(
            sanitize_player_text("Ulundi rose at the coordinates (12, 23)."),
            "Ulundi rose."
        );
    }
}
