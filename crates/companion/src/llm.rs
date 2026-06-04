#![cfg_attr(not(windows), allow(dead_code))]

use std::time::Duration;

use ages_beyond_protocol::{GameEvent, RequestBody};
use anyhow::{anyhow, Context};
use reqwest::Url;
use serde::{Deserialize, Serialize};
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
            system: "You write concise, grounded Civilization IV narrative text. Use only the supplied game facts. Do not invent game-mechanical consequences.",
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
                Ok(text) => Ok(text),
                Err(err) => {
                    warn!(event_type = %event.event_type, error = %err, "using fallback chronicle text");
                    Ok(fallback_game_event_text(event))
                }
            },
        }
    }
}

fn game_event_prompt(event: &GameEvent) -> String {
    let facts = serde_json::to_string_pretty(&event.facts).unwrap_or_else(|_| "{}".to_owned());
    let actors = serde_json::to_string_pretty(&event.actors).unwrap_or_else(|_| "[]".to_owned());
    let summary = event.summary.as_deref().unwrap_or("");

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
        Some(summary) if !summary.trim().is_empty() => summary.trim().to_owned(),
        _ => format!(
            "A {} event was recorded.",
            event.event_type.replace('_', " ")
        ),
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
