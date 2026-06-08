# Rival Quest Mirrors

## Goal

Some Living Quests should create counterpart quests for rival civilizations.
This makes conflicts and settlements feel shared rather than player-centric.

## Player Experience

When one civ gains a quest, the rival may gain a mirrored problem:

- Egypt: `The Claim on Memphis`
- Rome: `Hold Memphis`

If Egypt restores the city, Rome can later receive:

- `Avenge the Lost Province`
- `Accept the Reversal`
- `Make the Border Hold`

The world feels like both sides remember.

## Non-Goals

- Do not show popups for non-active-player rival decisions.
- Do not require the rival AI to make explicit quest choices.
- Do not mechanically force war or peace.

## State Model

Add optional mirror metadata:

- `mirror_group_id: Option<String>`
- `mirror_role: Option<String>` values such as `claimant`, `holder`,
  `attacker`, `defender`, `settler_a`, `settler_b`
- `mirror_of: Option<String>`

Mirror groups should survive snapshot persistence and appear in quest logs.

## Mirror Seeds

Initial mirror groups:

- city capture:
  - old owner restoration quest
  - new owner legitimacy quest
- war declaration:
  - attacker war aim quest
  - defender survival quest
- peace:
  - settlement quest for each side

Some of this already exists as paired seeds. Mirror metadata makes the
relationship explicit and enables later cross-resolution logic.

## Cross-Resolution Rules

When one mirrored quest resolves, Rust checks siblings:

- claimant restores city -> holder quest fails or transforms
- holder legitimizes city -> claimant restoration quest gains urgency
- attacker war aim completes -> defender survival quest transforms
- peace settlement completes for one side -> other side receives reduced
  failure pressure

The first implementation should only add progress notes to sibling quests.
Transformations can come later.

## Rival Decisions

For non-active players, Rust can assign an implicit stance from event facts:

- city razer -> `Make an example`
- peace signer -> `Honor the peace`
- repeated aggressor -> `Seek decisive victory`

These should be marked as inferred, not chosen by the user.

## Prompt Context

When generating diplomacy between mirrored participants, include:

- mirror group title or target
- both quest statuses
- each side's stance if known
- latest cross-resolution note

This gives diplomacy a strong reason to mention shared history.

## Tests

Add Rust tests for:

- city capture creates mirror group metadata
- war declaration links attacker and defender quests
- peace creates paired settlement mirror quests
- resolving one side adds note to sibling quest
- mirror metadata persists through memory snapshot
- diplomacy context includes mirror group when relevant

## Risks

Mirrors can double quest volume quickly. Keep the active quest cap, deduplicate
by mirror group, and only create mirrors for major relationship events.
