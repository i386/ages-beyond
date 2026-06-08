# Civilization Ideology From Play

## Goal

Civilizations should develop emergent ideological identities from what they do,
what they build, what quests they complete, and what stances they choose.

## Player Experience

The chronicle can describe civilizations as becoming something:

- restoration monarchy
- punitive empire
- treaty republic
- sacred war state
- knowledge commonwealth
- builder kingdom
- frontier confederation

These are inferred from play, not selected from a menu.

## Non-Goals

- Do not replace Civ IV civics or leader traits.
- Do not force one ideology per civilization forever.
- Do not let the LLM silently change ideology state.

## State Model

Add `CivilizationIdeology` per player:

- `label: String`
- `scores: HashMap<String, i32>`
- `evidence: VecDeque<String>`
- `updated_turn`
- `confidence: i32`

Candidate ideology keys:

- `restoration_monarchy`
- `punitive_empire`
- `treaty_republic`
- `sacred_law`
- `knowledge_commonwealth`
- `builder_kingdom`
- `frontier_state`
- `merchant_order`
- `war_camp`

## Inputs

Use reputation tags, tension, and quest outcomes:

- restoration completions -> restoration monarchy
- repeated punitive choices -> punitive empire
- peace and mercy choices -> treaty republic
- faith public-law choices -> sacred law
- discoveries and projects -> knowledge commonwealth
- wonders and golden ages -> builder kingdom
- city founding and border settlement -> frontier state

## Inference

Rust computes scores deterministically. The top score becomes the current
ideology when it exceeds a confidence threshold and leads the runner-up by a
margin.

LLM may rename the public label later:

- key: `knowledge_commonwealth`
- fallback: `Knowledge Commonwealth`
- generated: `The Scribes' Compact`

The key remains canonical.

## Prompt Context

Chronicle prompts get the actor's ideology label and one evidence line.
Diplomacy prompts get active and rival ideology labels only when confidence is
high or when ideology differs sharply.

## Quest Integration

Ideology can bias chain templates:

- punitive empire receives harsher war follow-ups
- treaty republic receives settlement institution quests
- sacred law receives faith diplomacy quests
- knowledge commonwealth receives discovery application quests

## Tests

Add Rust tests for:

- repeated restoration outcomes infer restoration ideology
- mercy and peace infer treaty ideology
- project and discovery infer knowledge ideology
- low evidence does not produce confident ideology
- ideology persists in memory snapshot
- ideology appears in event and diplomacy enrichment when confident

## Risks

The labels can feel too modern or too prescriptive. Keep fallback labels broad,
and prefer generated names only as display labels over canonical state.
