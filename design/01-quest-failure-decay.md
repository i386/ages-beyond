# Living Quest Failure and Decay

## Goal

Living Quests should resolve even when the player ignores them. A quest that
stays active for too long should become a remembered failure, compromise, or
unresolved wound instead of remaining active forever.

## Player Experience

The player sees that history moved on:

- `The Claim on Memphis` becomes `The Lost Claim`.
- `Name the War Aim` becomes `The Empty War`.
- `Spend the Bright Years` becomes `The Squandered Bright Years`.

The result appears in the quest log, chronicle, diplomacy context, and future
quest generation. Failure is narrative pressure, not a hard punishment at first.

## Non-Goals

- Do not add direct mechanical penalties in the first implementation.
- Do not require Python timers or UI state.
- Do not ask the LLM whether a quest failed; Rust determines failure.

## State Model

Extend `LivingQuest` with optional deterministic lifecycle fields:

- `deadline_turn: Option<i32>`
- `decay_stage: i32`
- `failure_title: Option<String>`
- `failure_note: Option<String>`
- `resolved_as: Option<String>` where values include `completed`, `failed`,
  `expired`, `abandoned`, or `transformed`.

Old snapshots should default missing fields to active, undecayed behavior.

## Deadline Rules

Initial deadlines can be kind-specific:

- restoration: 80 turns
- legitimacy: 45 turns
- legacy: 60 turns
- faith: 70 turns
- breakthrough: 50 turns
- mandate: 35 turns
- war aim: 30 turns or until peace closes the conflict
- survival: 30 turns or until peace closes the conflict
- settlement: 50 turns
- project: 60 turns

If `origin_turn` is missing, the quest does not time out until a later event
allows Rust to establish a deadline.

## Decay Rules

On each accepted event, Rust checks active quests:

1. If the quest completion matcher succeeds, complete it normally.
2. Else if the quest has reached `deadline_turn`, resolve it as failed.
3. Else if the current turn is near the deadline, add a progress note at most
   once per stage.

Suggested stages:

- warning at 50 percent of deadline
- urgent at 80 percent
- failed at 100 percent

## Failure Text

Rust can generate deterministic notes first:

- restoration: `{civ} let the claim on {target} harden into grievance.`
- legitimacy: `{civ} failed to turn {target} from conquest into accepted rule.`
- war aim: `{civ} ended the war without proving its declared aim.`

Later, an optional LLM pass can rename the failed quest. The deterministic note
must remain stored even if naming fails.

## Projections

Failure should write:

- `Quest:` projection to the `Living Quests` chapter.
- quest notification TSV line.
- updated quest log with a `Failed` section.
- updated quest journal summary counts.
- memory snapshot update.

The quest journal should expose active, completed, and failed counts.

## Diplomacy and Prompt Context

Failed quests should be included in diplomacy context for involved players.
Examples:

- active player failed to restore a city
- rival survived an empty war
- both sides remember a settlement that decayed into mistrust

Keep context bounded; include only recent failed quests plus high-pressure
failures tied to the current relationship.

## Tests

Add Rust tests for:

- restoration quest fails after deadline without city retake
- war aim fails on peace if no meaningful victory or settlement exists
- failed quest is persisted and restored from memory snapshot
- failed quest appears in quest log and journal counts
- failed quest is included in diplomacy enrichment
- old snapshots without deadline fields restore successfully

## Risks

Failure can feel arbitrary if deadlines are too short. Start conservative and
make deadlines visible in the quest log before adding stronger consequences.
