# Reputation Tags

## Goal

Civilizations should develop remembered reputations from quest decisions and
event outcomes. Diplomacy and chronicle prompts can then talk about how a civ
acts over time, not just what happened most recently.

## Player Experience

Repeated choices become identity:

- Egypt becomes `restorer`, `vengeful`, or `merciful`.
- Rome becomes `city-burner`, `oathbreaker`, or `lawgiver`.
- Mali becomes `knowledge-builder` or `golden-age-spender`.

Diplomacy lines can reference these reputations when relevant.

## Non-Goals

- Do not alter Civ IV attitude values initially.
- Do not expose raw numeric scores in the in-game UI.
- Do not let LLM output invent reputations that Rust has not stored.

## State Model

Add a `CivilizationReputation` map keyed by player id:

- `tags: HashMap<String, ReputationTag>`

Each tag stores:

- `key`
- `label`
- `score`
- `evidence: VecDeque<String>`
- `updated_turn`

Scores can range from `-100` to `100`, but prompts should receive only compact
labels and evidence.

## Tag Sources

Quest decisions:

- `Swear vengeance` -> `vengeful`
- `Govern with mercy` -> `merciful`
- `Make an example` -> `punitive`
- `Make it diplomacy` -> `bridge-builder`
- `Use it for power` -> `power-seeking`

Events:

- city razed -> `city-burner`
- city restored -> `restorer`
- repeated peace -> `treaty-maker`
- war without settlement -> `reckless`
- wonder/project completions -> `builder`
- discoveries applied -> `knowledge-builder`

## Scoring

Use small increments:

- minor evidence: +5
- major quest completion: +15
- stance-backed completion: +20
- failure or contradiction: -10 against opposing tags

Decay can reduce old scores slowly after era transitions, but evidence strings
should remain in memory for recent high-impact tags.

## Prompt Context

Diplomacy context should include relationship-relevant reputations:

- active player's top tags
- rival leader's top tags
- tags tied to the current relationship

Chronicle context should include the event actor's top tags only. Keep the
prompt compact to avoid crowding out event facts.

## UI and Files

First implementation can be inspectable only:

- include top reputation tags in `AgesBeyondMemory.json`
- optionally add a `## Reputations` section to a future memory log

No Python UI is required for v1.

## Tests

Add Rust tests for:

- quest decision response adds expected tag
- quest completion strengthens stance tag
- city razing adds `city-burner`
- restoration completion adds `restorer`
- reputation persists through memory snapshot
- diplomacy context includes active and rival tags
- prompt context remains bounded

## Risks

Tags can become reductive if every event adds one. Keep the source list small,
use evidence strings, and prefer a few strong tags over many weak ones.
