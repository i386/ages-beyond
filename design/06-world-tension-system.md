# World Tension System

## Goal

The director should track historical pressure in the world and use it to bias
quest creation, arc naming, and prompt context. This gives Rust a simple model
of what kind of history the campaign is producing.

## Player Experience

The world starts to develop moods:

- conquest pressure rises after captures and razing
- religious pressure rises after faith founding and holy conflict
- legitimacy pressure rises after occupied cities
- revenge pressure rises after razing, failed restoration, and empty wars
- prestige pressure rises after wonders, projects, and golden ages

Chronicles and quests can then lean into what the world is actually doing.

## Non-Goals

- Do not make a hidden grand-strategy AI.
- Do not alter game mechanics in v1.
- Do not expose raw global meters in UI yet.

## State Model

Add `WorldTensionState` to director memory:

- `global: HashMap<String, TensionTrack>`
- `by_player: HashMap<i32, HashMap<String, TensionTrack>>`
- `by_relationship: HashMap<RelationshipKey, HashMap<String, TensionTrack>>`

Each `TensionTrack` stores:

- `key`
- `score`
- `evidence: VecDeque<String>`
- `updated_turn`

Suggested keys:

- `conquest`
- `revenge`
- `legitimacy`
- `faith`
- `prestige`
- `settlement`
- `knowledge`
- `collapse`

## Event Inputs

Examples:

- city captured: conquest + legitimacy
- city razed: conquest + revenge + collapse
- peace signed: settlement, reduced active war tension
- wonder built: prestige
- project built: prestige + knowledge
- religion founded: faith
- tech discovered: knowledge
- quest failed: collapse or revenge depending on kind

## Decay

At era transitions, reduce scores by a small percentage unless reinforced.
Evidence should stay bounded and recent.

## Usage

Quest seeding:

- high revenge pressure increases restoration and reprisal quests
- high legitimacy pressure increases occupation and settlement quests
- high knowledge pressure increases breakthrough and institution quests

Prompt context:

- include top global tracks for chronicle prompts
- include relationship tracks for diplomacy prompts
- include player tracks for civilization arc prompts

## Files and UI

First implementation:

- persist in `AgesBeyondMemory.json`
- include a compact summary in companion logs or future memory log

Avoid Python UI until the track values prove useful.

## Tests

Add Rust tests for:

- capture raises conquest and legitimacy
- razing raises revenge
- peace raises settlement for relationship
- era transition decays older tension
- high tension appears in enriched event context
- high relationship tension appears in diplomacy context
- snapshot persists and restores tension

## Risks

If every event affects too many tracks, the system becomes noise. Start with
small event-to-track mappings and tune from observed campaigns.
