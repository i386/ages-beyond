# Diplomatic Consequences

## Goal

Living Quest outcomes should change generated diplomacy context. Leaders should
remember what civilizations chose, failed, restored, betrayed, or endured when
they speak to each other.

## Player Experience

Diplomacy starts reflecting historical conduct:

- Rome greets Egypt differently if Egypt chose `Swear vengeance`.
- A merciful conqueror is described differently from a punitive conqueror.
- A failed war aim becomes something rivals can mock.
- A restored city becomes a legitimacy claim in later negotiations.

The first version changes generated text only. Mechanical attitude changes can
be considered later.

## Non-Goals

- Do not modify Civ IV diplomacy attitude numbers in v1.
- Do not replace vanilla fallback text when no generated line is cached.
- Do not let the LLM invent diplomatic memory that Rust did not provide.

## State Inputs

Diplomatic context should draw from existing or planned state:

- relationship memories
- named conflicts and treaties
- active, completed, and failed Living Quests
- quest decisions and inferred stances
- mirror quest groups
- reputation tags
- institutions involving both civilizations
- world and relationship tension

## Consequence Types

Store compact consequence labels on quest resolution:

- `restored_claim`
- `vengeance_sworn`
- `mercy_shown`
- `example_made`
- `war_aim_proven`
- `empty_war`
- `peace_honored`
- `peace_betrayed`
- `faith_as_law`
- `faith_as_diplomacy`

These labels are deterministic. They can later feed reputation tags and
relationship tension.

## Context Selection

For a diplomacy request between active player A and leader B:

1. Include direct relationship memories between A and B.
2. Include active or recent named conflict/treaty between A and B.
3. Include quests whose target or mirror group involves both A and B.
4. Include recent consequences from either side that mention the other side.
5. Include top reputation tags only when they are relevant to the relationship.

Keep the final prompt compact. Prefer one high-signal consequence over a long
list of unrelated history.

## Prompt Shape

Add a diplomacy section like:

```text
Diplomatic Consequences:
- Egypt restored Memphis from Rome and chose Swear vengeance.
- Rome failed to hold its legitimacy quest for Memphis.
- Recent treaty: The Memphis Settlement.
```

This should be separate from generic world arc context so the LLM can use it
directly for leader speech.

## Optional Mechanical Layer

After text-only behavior is stable, Rust can emit suggested mechanical hooks:

- no direct attitude mutation by default
- possible TSV command file for future Python application
- commands must be idempotent and active-player safe

Examples:

- minor gold or culture bonus for honored peace
- warning popup for repeated betrayal
- no hidden attitude changes until explicitly designed

## Tests

Add Rust tests for:

- restoration completion appears in diplomacy context between former owners
- vengeance stance appears in diplomacy context
- failed war aim appears in rival diplomacy context
- unrelated quests are excluded from diplomacy context
- context remains bounded when many quests exist
- generated diplomacy cache is cleared when quest decisions change

## Risks

Too much context can make leader lines rambling. The selector should rank direct
relationship consequences above global reputation or world tension.
