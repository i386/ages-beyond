#![cfg_attr(not(windows), allow(dead_code))]

use ages_beyond_protocol::{GameEvent, RequestBody};
use anyhow::{anyhow, Context};
use reqwest::Url;
use serde::{Deserialize, Serialize};

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
            client: reqwest::Client::new(),
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
            RequestBody::GameEvent { event } => self.generate(game_event_prompt(event)).await,
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
