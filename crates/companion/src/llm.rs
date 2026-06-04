#![cfg_attr(not(windows), allow(dead_code))]

use std::time::Duration;

use ages_beyond_protocol::{
    DiplomacyTextRequest, GameEvent, HistoricalNameRequest, RequestBody, WorldArcRequest,
};
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
            RequestBody::DiplomacyText { request } => {
                match self.generate(diplomacy_text_prompt(request)).await {
                    Ok(text) => Ok(sanitize_diplomacy_text(&text)),
                    Err(err) => {
                        warn!(comment_type = %request.comment_type, error = %err, "using fallback diplomacy text");
                        Ok(fallback_diplomacy_text(request))
                    }
                }
            }
            RequestBody::HistoricalName { request } => {
                match self.generate(historical_name_prompt(request)).await {
                    Ok(text) => Ok(sanitize_title(&text, &request.fallback_title)),
                    Err(err) => {
                        warn!(
                            name_kind = %request.name_kind,
                            trigger_event_type = %request.trigger_event_type,
                            error = %err,
                            "using fallback historical name"
                        );
                        Ok(sanitize_title("", &request.fallback_title))
                    }
                }
            }
            RequestBody::WorldArcTitle { request } => {
                match self.generate(world_arc_title_prompt(request)).await {
                    Ok(text) => Ok(sanitize_title(&text, &request.fallback_title)),
                    Err(err) => {
                        warn!(
                            trigger_event_type = %request.trigger_event_type,
                            error = %err,
                            "using fallback world arc title"
                        );
                        Ok(sanitize_title("", &request.fallback_title))
                    }
                }
            }
        }
    }
}

fn historical_name_prompt(request: &HistoricalNameRequest) -> String {
    let involved_civilizations = request.involved_civilizations.join(", ");
    let notable_places = request.notable_places.join(", ");
    let notable_terms = request.notable_terms.join(", ");
    let recent_events = request.recent_events.join(" | ");

    format!(
        "Name this Civilization IV historical subject.\n\
         Use the supplied facts only.\n\
         Be creative and specific to the civilizations, cities, faiths, wonders, people, or conflict in play.\n\
         For name_kind war, return a war name. For name_kind treaty, return a treaty or peace name. For name_kind civilization_arc, return a civilization-specific historical arc name.\n\
         Do not invent hidden map knowledge or mechanical consequences.\n\
         Do not mention map coordinates, tile coordinates, x/y values, or coordinate pairs.\n\
         Return only the name, one line, maximum 8 words.\n\
         Use plain ASCII punctuation.\n\n\
         Name kind: {name_kind}\n\
         Trigger event: {trigger_event_type}\n\
         Turn: {turn}\n\
         Subject: {subject}\n\
         Theme: {theme}\n\
         Involved civilizations: {involved_civilizations}\n\
         Notable places: {notable_places}\n\
         Notable terms: {notable_terms}\n\
         Recent world events: {recent_events}\n\
         Current name: {current_name}\n\
         Fallback name: {fallback_title}",
        name_kind = request.name_kind,
        trigger_event_type = request.trigger_event_type,
        turn = request
            .turn
            .map(|turn| turn.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        subject = request.subject,
        theme = request.theme,
        involved_civilizations = if involved_civilizations.is_empty() {
            "unknown"
        } else {
            &involved_civilizations
        },
        notable_places = if notable_places.is_empty() {
            "unknown"
        } else {
            &notable_places
        },
        notable_terms = if notable_terms.is_empty() {
            "unknown"
        } else {
            &notable_terms
        },
        recent_events = if recent_events.is_empty() {
            "none"
        } else {
            &recent_events
        },
        current_name = request.current_name.as_deref().unwrap_or("none"),
        fallback_title = request.fallback_title,
    )
}

fn world_arc_title_prompt(request: &WorldArcRequest) -> String {
    let involved_civilizations = request.involved_civilizations.join(", ");
    let notable_places = request.notable_places.join(", ");
    let notable_terms = request.notable_terms.join(", ");
    let recent_events = request.recent_events.join(" | ");

    format!(
        "Name the current historical arc in this Civilization IV game.\n\
         Use the supplied facts only.\n\
         Be creative and specific to the civilizations, cities, religions, wonders, people, or conflicts in play.\n\
         Avoid generic placeholder titles when the facts give you something sharper.\n\
         Do not invent hidden map knowledge or mechanical consequences.\n\
         Do not mention map coordinates, tile coordinates, x/y values, or coordinate pairs.\n\
         Return only the title, one line, maximum 8 words.\n\
         Use plain ASCII punctuation.\n\n\
         Trigger event: {trigger_event_type}\n\
         Turn: {turn}\n\
         Theme: {theme}\n\
         Pressure: {pressure}\n\
         Involved civilizations: {involved_civilizations}\n\
         Notable places: {notable_places}\n\
         Notable terms: {notable_terms}\n\
         Recent world events: {recent_events}\n\
         Current title: {current_title}\n\
         Fallback title: {fallback_title}",
        trigger_event_type = request.trigger_event_type,
        turn = request
            .turn
            .map(|turn| turn.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        theme = request.theme,
        pressure = request.pressure,
        involved_civilizations = if involved_civilizations.is_empty() {
            "unknown"
        } else {
            &involved_civilizations
        },
        notable_places = if notable_places.is_empty() {
            "unknown"
        } else {
            &notable_places
        },
        notable_terms = if notable_terms.is_empty() {
            "unknown"
        } else {
            &notable_terms
        },
        recent_events = if recent_events.is_empty() {
            "none"
        } else {
            &recent_events
        },
        current_title = request.current_title.as_deref().unwrap_or("none"),
        fallback_title = request.fallback_title,
    )
}

fn diplomacy_text_prompt(request: &DiplomacyTextRequest) -> String {
    format!(
        "Write one Civilization IV diplomacy line spoken by the rival leader.\n\
         Constraints:\n\
         - 1 sentence, maximum 28 words.\n\
         - The line must fit a diplomacy screen.\n\
         - Historical ruler tone, not modern UI language.\n\
         - Speak directly to the active player.\n\
         - Match the diplomacy comment type and attitude.\n\
         - Do not invent trades, treaties, wars, cities, religions, or hidden map knowledge.\n\
         - Do not include labels, quotation marks, markdown, or bullet points.\n\
         - Use plain ASCII punctuation.\n\n\
         Comment type: {comment_type}\n\
         Turn: {turn}\n\
         Speaker leader: {leader_name}\n\
         Speaker civilization: {leader_civilization}\n\
         Active player: {active_player_name}\n\
         Active civilization: {active_civilization}\n\
         Attitude: {attitude}\n\
         At war: {at_war}\n\
         Relative power: {power_relation}\n\
         Diplomacy memory: {diplomacy_memory}\n\
         Current world arc: {world_arc}\n\
         Vanilla fallback line: {fallback_text}",
        comment_type = request.comment_type,
        turn = request
            .turn
            .map(|turn| turn.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
        leader_name = optional_text(&request.leader_name),
        leader_civilization = optional_text(&request.leader_civilization),
        active_player_name = optional_text(&request.active_player_name),
        active_civilization = optional_text(&request.active_civilization),
        attitude = optional_text(&request.attitude),
        at_war = request.at_war,
        power_relation = optional_text(&request.power_relation),
        diplomacy_memory = optional_text(&request.diplomacy_memory),
        world_arc = optional_text(&request.world_arc),
        fallback_text = optional_text(&request.fallback_text),
    )
}

fn optional_text(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("unknown")
}

fn fallback_diplomacy_text(request: &DiplomacyTextRequest) -> String {
    request
        .fallback_text
        .as_deref()
        .map(sanitize_diplomacy_text)
        .unwrap_or_default()
}

fn sanitize_diplomacy_text(text: &str) -> String {
    let without_labels = text
        .trim()
        .trim_matches('"')
        .strip_prefix("Diplomacy:")
        .unwrap_or_else(|| text.trim().trim_matches('"'))
        .trim();

    sanitize_player_text(without_labels)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(220)
        .collect()
}

fn sanitize_title(text: &str, fallback: &str) -> String {
    let candidate = clean_title(text);
    if !candidate.is_empty() {
        return candidate;
    }

    let fallback = clean_title(fallback);
    if fallback.is_empty() {
        "The Turning Age".to_owned()
    } else {
        fallback
    }
}

fn sanitize_world_arc_title(text: &str, fallback: &str) -> String {
    sanitize_title(text, fallback)
}

fn clean_title(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    let without_label = line
        .strip_prefix("World Arc:")
        .or_else(|| line.strip_prefix("Historical Name:"))
        .or_else(|| line.strip_prefix("Name:"))
        .or_else(|| line.strip_prefix("Title:"))
        .unwrap_or(line)
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim();

    let no_coordinates = sanitize_player_text(without_label);
    let normalized = no_coordinates
        .replace(['\u{2018}', '\u{2019}'], "'")
        .replace(['\u{201c}', '\u{201d}'], "\"")
        .replace(['\u{2013}', '\u{2014}'], "-");

    let mut compact = String::new();
    let mut last_was_space = false;
    for ch in normalized.chars().filter(|ch| !ch.is_control()) {
        if ch.is_whitespace() {
            if !last_was_space {
                compact.push(' ');
                last_was_space = true;
            }
        } else {
            compact.push(ch);
            last_was_space = false;
        }
    }

    compact
        .trim()
        .trim_end_matches(['.', ',', ';', ':'])
        .chars()
        .take(80)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn game_event_prompt(event: &GameEvent) -> String {
    let facts =
        serde_json::to_string_pretty(&prompt_facts(event)).unwrap_or_else(|_| "{}".to_owned());
    let actors = serde_json::to_string_pretty(&event.actors).unwrap_or_else(|_| "[]".to_owned());
    let summary = prompt_summary(event);

    format!(
        "Write one short in-world chronicle entry for this Civilization IV game event.\n\
         You may also add lightweight experimental narrative gameplay hooks.\n\
         Constraints:\n\
         - Return 1 to 4 labeled lines.\n\
         - First line must begin with Chronicle: and contain 1 sentence, maximum 35 words.\n\
         - Optionally add Council: with one roleplay suggestion for the active player, not a command and not a claim that mechanics changed.\n\
         - Optionally add Quest Hook: when facts.dynamic_quest_seed exists; make it an unresolved narrative prompt, not a new enforced objective.\n\
         - Optionally add World Arc: for epochal events; give the emerging age, conflict, or legacy a short title.\n\
         - Historical tone, not modern UI language.\n\
         - Mention only facts provided below.\n\
         - Match tone to facts.importance: minor is restrained, major is consequential, epochal is chapter-defining.\n\
         - If facts.world_arc_title, facts.civilization_arcs, facts.named_conflict_title, facts.named_treaty_title, or facts.recent_world_events are present, use them only as continuity context.\n\
         - If facts.diplomacy_memory is present, you may echo the historical memory without adding new accusations.\n\
         - Treat facts.data1 as target_team_id for war_declared/peace_signed, tech_id for tech_discovered, religion_id for religion_founded, and building_id for wonder_built.\n\
         - Treat facts.data1/data2 according to named facts when a clearer *_id field is present.\n\
         - Coordinates may appear in facts only as private grounding context; never mention map coordinates, tile coordinates, plot positions, x/y values, or coordinate pairs in the prose.\n\
         - If facts.location_known_to_active_player is false, do not mention city names, terrain locations, directions, or local geography.\n\
         - Use plain ASCII punctuation.\n\
         - No bullet points or markdown.\n\n\
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
    let summary = prompt_summary(event);

    let chronicle = if !summary.trim().is_empty() {
        sanitize_player_text(&summary)
    } else {
        format!(
            "A {} event was recorded.",
            event.event_type.replace('_', " ")
        )
    };

    let mut lines = vec![format!("Chronicle: {chronicle}")];

    if let Some(council) = fallback_council_text(event) {
        lines.push(format!("Council: {council}"));
    }

    if let Some(hook) = fallback_quest_hook_text(event) {
        lines.push(format!("Quest Hook: {hook}"));
    }

    if let Some(arc) = fallback_world_arc_text(event) {
        lines.push(format!("World Arc: {arc}"));
    }

    lines.join("\n")
}

fn prompt_facts(event: &GameEvent) -> serde_json::Map<String, Value> {
    let location_known = fact_bool(event, "location_known_to_active_player").unwrap_or(true);

    event
        .facts
        .iter()
        .filter(|(key, _)| is_prompt_fact(key, location_known))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn is_prompt_fact(key: &str, location_known: bool) -> bool {
    location_known || !is_location_fact(key)
}

fn is_location_fact(key: &str) -> bool {
    matches!(
        key,
        "x" | "y"
            | "plot_x"
            | "plot_y"
            | "city_x"
            | "city_y"
            | "map_x"
            | "map_y"
            | "city_id"
            | "city_name"
            | "city_population"
            | "city_highest_population"
            | "city_religion_count"
            | "city_world_wonder_count"
            | "city_is_capital"
            | "city_is_holy_city"
            | "city_is_coastal"
            | "city_original_owner_id"
            | "city_previous_owner_id"
    )
}

fn prompt_summary(event: &GameEvent) -> String {
    if fact_bool(event, "location_known_to_active_player").unwrap_or(true) {
        return event
            .summary
            .as_deref()
            .map(sanitize_player_text)
            .unwrap_or_default();
    }

    match event.event_type.as_str() {
        "religion_founded" => fact_string(event, "religion_name")
            .map(|religion| format!("{religion} was founded."))
            .unwrap_or_default(),
        "wonder_built" => fact_string(event, "building_name")
            .map(|building| format!("{building} was completed."))
            .unwrap_or_default(),
        "project_built" => fact_string(event, "project_name")
            .map(|project| format!("{project} was completed."))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn fact_bool(event: &GameEvent, key: &str) -> Option<bool> {
    match event.facts.get(key) {
        Some(Value::Bool(value)) => Some(*value),
        Some(Value::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn fact_string(event: &GameEvent, key: &str) -> Option<String> {
    match event.facts.get(key) {
        Some(Value::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn fallback_council_text(event: &GameEvent) -> Option<String> {
    if fact_bool(event, "involves_active_player") != Some(true)
        && fact_bool(event, "involves_active_team") != Some(true)
    {
        return None;
    }

    match event.event_type.as_str() {
        "city_founded" => {
            Some("Consider what this settlement should be remembered for.".to_owned())
        }
        "city_captured" => {
            Some("Decide whether conquest should become mercy, order, or warning.".to_owned())
        }
        "tech_discovered" => {
            Some("Let the new knowledge shape a visible ambition before rivals answer.".to_owned())
        }
        "war_declared" => Some("Name the purpose of the war before the war names you.".to_owned()),
        "peace_signed" => {
            Some("Use the quiet after the treaty to choose what must not be repeated.".to_owned())
        }
        "golden_age_started" => {
            Some("Spend the bright years on a legacy that will outlast them.".to_owned())
        }
        "great_person_born" => {
            Some("Give this life a patron, a rival, or an unfinished work.".to_owned())
        }
        "quest_started" | "event_triggered" => Some(
            "Treat this choice as a story promise, even if the empire pays no cost yet.".to_owned(),
        ),
        _ => None,
    }
}

fn fallback_quest_hook_text(event: &GameEvent) -> Option<String> {
    if fact_string(event, "dynamic_quest_seed").is_none() {
        return None;
    }

    match event.event_type.as_str() {
        "city_founded" => Some("The first generation will ask whether this place was founded for safety, glory, or exile.".to_owned()),
        "city_captured" => Some("The conquered streets wait to see whether old loyalties become rebellion or renewal.".to_owned()),
        "religion_founded" => Some("Pilgrims will soon test whether faith can bind distant ambitions.".to_owned()),
        "wonder_built" => Some("The monument demands a deed worthy of its shadow.".to_owned()),
        "project_built" => Some("A great work now asks what burden it was built to carry.".to_owned()),
        "war_declared" => Some("The first victory may decide whether this war is remembered as justice or hunger.".to_owned()),
        "peace_signed" => Some("The treaty leaves one grievance unnamed and waiting.".to_owned()),
        "tech_discovered" => Some("The discovery opens a door that custom may resist walking through.".to_owned()),
        "golden_age_started" => Some("The age will be judged by what survives after its brightness fades.".to_owned()),
        "great_person_born" => Some("A patron, rival, or impossible work could turn talent into legend.".to_owned()),
        _ => Some("A small choice now could become the seed of a later age.".to_owned()),
    }
}

fn fallback_world_arc_text(event: &GameEvent) -> Option<String> {
    if fact_string(event, "importance").as_deref() != Some("epochal") {
        return None;
    }

    let title = match event.event_type.as_str() {
        "game_started" => "The First Turning",
        "city_founded" => "The Age of Foundations",
        "city_razed" => "The Ashen Reckoning",
        "religion_founded" => "The Covenant Age",
        "wonder_built" => "The Age of Monuments",
        "project_built" => "The Great Work",
        "golden_age_started" => "The Bright Mandate",
        "victory" => "The Final Age",
        _ => "The Turning Age",
    };

    Some(title.to_owned())
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

    use super::{
        fallback_game_event_text, game_event_prompt, sanitize_player_text, sanitize_world_arc_title,
    };

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
                ("location_known_to_active_player".to_owned(), json!(true)),
            ]),
        }
    }

    #[test]
    fn prompt_keeps_private_map_coordinates_for_grounding() {
        let prompt = game_event_prompt(&event_with_coordinates());

        assert!(prompt.contains("\"x\""));
        assert!(prompt.contains("\"y\""));
        assert!(!prompt.contains("(12,23)"));
        assert!(prompt.contains("\"city_name\""));
        assert!(prompt.contains("private grounding context"));
    }

    #[test]
    fn prompt_redacts_hidden_location_facts() {
        let mut event = event_with_coordinates();
        event
            .facts
            .insert("location_known_to_active_player".to_owned(), json!(false));

        let prompt = game_event_prompt(&event);

        assert!(!prompt.contains("\"x\""));
        assert!(!prompt.contains("\"y\""));
        assert!(!prompt.contains("\"city_name\""));
        assert!(!prompt.contains("Ulundi"));
    }

    #[test]
    fn fallback_text_excludes_coordinate_pairs() {
        assert_eq!(
            fallback_game_event_text(&event_with_coordinates()),
            "Chronicle: Zulu Empire founded Ulundi.\nWorld Arc: The Age of Foundations"
        );
    }

    #[test]
    fn fallback_text_can_add_experimental_hooks() {
        let mut event = event_with_coordinates();
        event
            .facts
            .insert("involves_active_player".to_owned(), json!(true));
        event.facts.insert(
            "dynamic_quest_seed".to_owned(),
            json!("settlement_identity"),
        );

        let text = fallback_game_event_text(&event);

        assert!(text.contains("Chronicle: Zulu Empire founded Ulundi."));
        assert!(text.contains("Council:"));
        assert!(text.contains("Quest Hook:"));
        assert!(text.contains("World Arc:"));
    }

    #[test]
    fn generated_text_sanitizer_removes_coordinate_pairs() {
        assert_eq!(
            sanitize_player_text("Ulundi rose at the coordinates (12, 23)."),
            "Ulundi rose."
        );
    }

    #[test]
    fn world_arc_title_sanitizer_keeps_creative_titles() {
        assert_eq!(
            sanitize_world_arc_title(
                "World Arc: The Saffron Argument of Timbuktu\nextra text",
                "Faith"
            ),
            "The Saffron Argument of Timbuktu"
        );
    }

    #[test]
    fn world_arc_title_sanitizer_removes_coordinates_and_uses_fallback() {
        assert_eq!(
            sanitize_world_arc_title("World Arc: (12, 23)", "The Oracle Wonder"),
            "The Oracle Wonder"
        );
    }
}
