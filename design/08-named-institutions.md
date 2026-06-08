# Named Institutions

## Goal

Major quest resolutions should be able to found persistent named institutions:
laws, courts, orders, roads, cults, compacts, treasuries, memorials, and schools.
These become reusable memory objects rather than one-off chronicle lines.

## Player Experience

History creates institutions:

- `Memphis Restoration Court`
- `Law of the Burned Cities`
- `The Oracle Compact`
- `Apollo Treasury`
- `Order of the Eastern Road`

Later chronicles and diplomacy can cite them as real parts of the world.

## Non-Goals

- Do not add buildings, units, or Civ IV XML assets in v1.
- Do not require the LLM to decide whether an institution exists.
- Do not create an institution for every quest.

## State Model

Add `Institution` records:

- `id`
- `owner_player_id`
- `name`
- `kind`
- `origin_quest_id`
- `origin_event_type`
- `target`
- `founded_turn`
- `status`
- `evidence: VecDeque<String>`

Institution kinds:

- `court`
- `law`
- `order`
- `compact`
- `memorial`
- `school`
- `treasury`
- `road`
- `cult`

## Creation Rules

Institutions can be created when:

- quest completes with a civic or stabilizing stance
- quest chain reaches depth 2 or more
- era-end chronicle identifies a dominant completed quest
- a project or wonder completes an existing quest

Examples:

- restoration + mercy -> court
- restoration + warning -> memorial law
- faith + diplomacy -> compact
- breakthrough + project -> school
- project + power -> treasury

## Naming

Rust produces deterministic fallback names. LLM can generate display names from
structured facts:

- owner civilization
- quest title
- stance
- target
- event deed
- institution kind

Generated names must be one line, bounded, non-empty, and free of coordinates.

## Projection

Institutions should appear in:

- memory snapshot
- chronicle `Memory:` or future `Institution:` projection
- event enrichment when actor owns relevant institution
- diplomacy enrichment when institution involves both civilizations

## Tests

Add Rust tests for:

- restoration completion with mercy creates court institution
- breakthrough completion with project creates school institution
- generated/fallback institution name is persisted
- old snapshots without institutions restore
- event enrichment includes relevant institution
- diplomacy context includes relationship-relevant institution

## Risks

Institution spam can make memory noisy. Require meaningful quest resolution and
keep a per-civilization cap, evicting low-impact or obsolete institutions first.
