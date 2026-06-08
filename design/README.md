# Ages Beyond Design Notes

This directory holds forward-looking design docs for deepening the Living
Quests system. The docs assume the current Rust companion owns director memory,
Living Quest state, chronicle projections, quest notifications, quest rewards,
quest decisions, and restart persistence through `AgesBeyondMemory.json`.

## Recommended Build Order

1. [Living Quest Failure and Decay](01-quest-failure-decay.md)
2. [Quest Chains](02-quest-chains.md)
3. [Diplomatic Consequences](03-diplomatic-consequences.md)
4. [Reputation Tags](04-reputation-tags.md)
5. [Rival Quest Mirrors](05-rival-quest-mirrors.md)
6. [World Tension System](06-world-tension-system.md)
7. [Civilization Ideology From Play](07-civilization-ideology-from-play.md)
8. [Named Institutions](08-named-institutions.md)
9. [Era-End Chronicle](09-era-end-chronicle.md)

The order starts with deterministic state changes, then adds richer narrative
projection once the underlying facts are durable enough to reference from
prompts, diplomacy, and quest generation.

## Shared Principles

- Rust owns canonical state. Python only polls projections, displays UI, and
  applies explicitly supported active-player commands.
- LLM output can name, summarize, and color the world, but it should not be the
  sole source of game-state truth.
- Every generated narrative artifact should trace back to structured event
  facts, stored decisions, or durable director memory.
- New state should survive companion restarts through the director memory
  snapshot, with versioned defaults for old snapshots.
- Quest and memory projections should remain readable outside the game through
  Markdown or TSV inspection files.
