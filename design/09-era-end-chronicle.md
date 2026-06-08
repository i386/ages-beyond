# Era-End Chronicle

## Goal

When a civilization crosses into a new era, the companion should create a
structured era-end judgment that summarizes what the previous era meant for that
civilization and the wider world.

## Player Experience

Era transitions become chapter breaks:

```text
The Ancient Era Ends: The Age of Broken Cities
Egypt entered the Classical Era carrying the restored claim of Memphis, the
unfinished vengeance against Rome, and the Oracle Compact's growing authority.
```

The player gets a sense of accumulated history, not just a tech notification.

## Non-Goals

- Do not block gameplay waiting for a long generation.
- Do not replace normal tech discovery chronicle entries.
- Do not summarize the entire save every time.

## Inputs

For the transitioning civilization:

- era memories
- completed quests since previous era
- failed quests since previous era
- active high-pressure quests
- top reputation tags
- current ideology
- institutions founded
- named conflicts and treaties involving the civ
- relevant world tension tracks

For global context:

- current world arc
- top global tension
- recent world events

## State Model

Add `EraChapter` records:

- `id`
- `player_id`
- `civilization`
- `old_era`
- `new_era`
- `turn`
- `title`
- `summary`
- `quest_ids`
- `institution_ids`
- `conflict_ids`

Persist chapters in memory snapshot and include recent chapters in prompt
context.

## Generation

Rust creates a deterministic era chapter request when `observe_era_transition`
emits an era event. LLM can generate:

- chapter title
- short summary

Fallback:

- `{civilization} entered {new_era} after {trigger}.`

The fallback must include enough structured facts to be useful even without
Ollama.

## Projection

Era chapters should append to the Markdown chronicle under an `Era Chapters`
section. Python notification should show only a short headline, not the full
chapter.

Potential files:

- `AgesBeyondEraChapters.md`
- or reuse `AgesBeyondChronicle.md` with a dedicated chapter

Start by reusing the chronicle to avoid another poller file.

## Quest Integration

Era transition should:

- decay old quest pressure
- optionally fail overdue quests
- seed era-specific follow-up quests
- summarize unresolved quests in the chapter

## Tests

Add Rust tests for:

- era transition creates chapter request
- chapter includes completed and failed quests from previous era
- fallback chapter is written when LLM fails
- chapter persists in memory snapshot
- chronicle appends era chapter without ordinary notification spam
- old snapshots without era chapters restore

## Risks

Prompt size can grow quickly. The era chapter builder must select only the most
important facts and cap each evidence list.
